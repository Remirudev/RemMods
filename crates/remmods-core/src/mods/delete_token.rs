//! Single-use tokens for the second confirmation of a permanent delete (spec §5.4); one token
//! covers one row or an exact, ordered list of rows (P3b §5.5).

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use super::error::ModsError;
use super::manage_target::ModTarget;
use super::modref::ModRef;

#[derive(Debug)]
struct PendingDelete {
    rows: Vec<ModTarget>,
    issued: Instant,
}

#[derive(Debug, Default)]
pub struct DeleteTokens {
    pending: Mutex<HashMap<String, PendingDelete>>,
}

impl DeleteTokens {
    pub const TTL: Duration = Duration::from_secs(120);

    pub fn new() -> Self {
        Self::default()
    }

    /// A fresh uuid v4 (32 lowercase hex) for exactly this mod_ref and key.
    pub fn issue(&self, mod_ref: &ModRef, key: &str, now: Instant) -> String {
        self.issue_many(&[one_row(mod_ref, key)], now)
    }

    /// One token for exactly these rows, in this order (TTL 120 s, single use).
    pub fn issue_many(&self, rows: &[ModTarget], now: Instant) -> String {
        let token = uuid::Uuid::new_v4().simple().to_string();
        let mut pending = self.lock();
        pending.retain(|_, entry| now.saturating_duration_since(entry.issued) <= Self::TTL);
        pending.insert(
            token.clone(),
            PendingDelete {
                rows: rows.to_vec(),
                issued: now,
            },
        );
        token
    }

    /// Removes the token (single use, even when the delete then fails). Ok only if it exists,
    /// is not older than TTL, and was issued for exactly this mod_ref AND key.
    pub fn consume(
        &self,
        token: &str,
        mod_ref: &ModRef,
        key: &str,
        now: Instant,
    ) -> Result<(), ModsError> {
        self.consume_many(token, &[one_row(mod_ref, key)], now)
    }

    /// Removes the token (single use, even when refused). Ok only if it exists, is fresh and was
    /// issued for exactly `rows` (same mod_refs and keys, same order).
    pub fn consume_many(
        &self,
        token: &str,
        rows: &[ModTarget],
        now: Instant,
    ) -> Result<(), ModsError> {
        let Some(entry) = self.lock().remove(token) else {
            return Err(ModsError::DeleteToken);
        };
        let fresh = now.saturating_duration_since(entry.issued) <= Self::TTL;
        if fresh && entry.rows == rows {
            Ok(())
        } else {
            Err(ModsError::DeleteToken)
        }
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<String, PendingDelete>> {
        self.pending.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn one_row(mod_ref: &ModRef, key: &str) -> ModTarget {
    ModTarget {
        mod_ref: mod_ref.clone(),
        key: key.to_owned(),
    }
}
