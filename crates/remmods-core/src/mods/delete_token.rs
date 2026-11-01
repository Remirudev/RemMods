//! Single-use tokens for the second confirmation of a permanent delete (spec §5.4).

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use super::error::ModsError;
use super::modref::ModRef;

#[derive(Debug)]
struct PendingDelete {
    mod_ref: ModRef,
    key: String,
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
        let token = uuid::Uuid::new_v4().simple().to_string();
        let mut pending = self.lock();
        pending.retain(|_, entry| now.saturating_duration_since(entry.issued) <= Self::TTL);
        pending.insert(
            token.clone(),
            PendingDelete {
                mod_ref: mod_ref.clone(),
                key: key.to_owned(),
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
        let Some(entry) = self.lock().remove(token) else {
            return Err(ModsError::DeleteToken);
        };
        let fresh = now.saturating_duration_since(entry.issued) <= Self::TTL;
        if fresh && entry.mod_ref == *mod_ref && entry.key == key {
            Ok(())
        } else {
            Err(ModsError::DeleteToken)
        }
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<String, PendingDelete>> {
        self.pending.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
