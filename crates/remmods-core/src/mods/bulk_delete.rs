//! Bulk permanent delete (P3b §5.5): one token for the exact, ordered list of eligible rows,
//! then one write guard; every row is set aside in its own journaled batch (all or none per
//! row) and removed. A cancelled folder-busy wait stops the rows after it.

use std::collections::{BTreeSet, HashSet};
use std::time::Instant;

use super::bulk::{
    BulkDeletePrepared, BulkDeleteRequest, BulkOutcome, BulkRow, BulkRowResult, PreparedRow,
    display_name, failed, is_busy_cancelled, not_run, validate_targets,
};
use super::delete::delete_label;
use super::delete_token::DeleteTokens;
use super::manage_target::{ManageAction, ModTarget, resolve_managed};
use super::model::{ScanContent, ScanInput};
use super::modref::ModRef;
use crate::safe_fs::{FaultPlan, RetryMode, WriteGuard};
use crate::{Core, CoreError};

impl Core {
    /// First step of a bulk delete, after the UI's first confirm: checks every row on a fresh
    /// scan (read guard) and issues one token for the eligible ones. Writes nothing.
    pub fn mods_delete_prepare_many(
        &self,
        targets: &[ModTarget],
    ) -> Result<BulkDeletePrepared, CoreError> {
        validate_targets(targets)?;
        let fresh = self.mods_refresh()?;
        let content = &fresh.snapshot.content;
        let rows: Vec<PreparedRow> = targets
            .iter()
            .map(|target| prepare_row(content, target))
            .collect();
        let eligible: Vec<ModTarget> = rows
            .iter()
            .filter(|row| row.refusal.is_none())
            .map(|row| row.target.clone())
            .collect();
        let token = (!eligible.is_empty())
            .then(|| self.delete_tokens.issue_many(&eligible, Instant::now()));
        Ok(BulkDeletePrepared {
            token,
            rows,
            expires_in_secs: u32::try_from(DeleteTokens::TTL.as_secs()).unwrap_or(u32::MAX),
        })
    }

    /// Second step, after the UI's second confirm: the token must cover exactly these rows in
    /// this order (it is spent by every later refusal).
    pub fn mods_delete_many(&self, request: &BulkDeleteRequest) -> Result<BulkOutcome, CoreError> {
        validate_targets(&request.targets)?;
        self.delete_tokens
            .consume_many(&request.token, &request.targets, Instant::now())?;
        let (rows, refreshed) = self.run_managed_bulk(|guard, input, content| {
            let retry = self.running_retry(request.apply_while_running)?;
            let mut rows = Vec::with_capacity(request.targets.len());
            let mut stopped = false;
            for target in &request.targets {
                let fault = self.take_batch_fault();
                if stopped {
                    rows.push(not_run(content, target));
                    continue;
                }
                let row = self.delete_bulk_row(guard, input, content, target, retry, fault);
                stopped = matches!(&row.result, BulkRowResult::Failed(error) if is_busy_cancelled(error))
                    || self.ops.busy().cancel_requested();
                rows.push(row);
            }
            self.forget_deleted_meta(guard, content, &rows);
            Ok(rows)
        })?;
        Ok(BulkOutcome { refreshed, rows })
    }

    fn delete_bulk_row(
        &self,
        guard: &WriteGuard<'_>,
        input: &ScanInput,
        content: &ScanContent,
        target: &ModTarget,
        retry: RetryMode,
        fault: Option<FaultPlan>,
    ) -> BulkRow {
        let managed = match resolve_managed(content, target) {
            Ok(managed) => managed,
            Err(error) => return failed(target, target.key.clone(), error.into()),
        };
        let name = display_name(&managed);
        let deleted = managed
            .check(ManageAction::Delete)
            .map_err(CoreError::from)
            .and_then(|()| self.delete_row(guard, input, &managed, retry, fault));
        let result = match deleted {
            Ok(leftover) => BulkRowResult::Deleted { leftover },
            Err(error) => BulkRowResult::Failed(error),
        };
        BulkRow {
            target: target.clone(),
            name,
            result,
        }
    }

    /// One `mod-meta.json` update dropping the keys of deleted rows that no surviving row of the
    /// fresh scan still has (logged on failure). Profiles keep the keys.
    fn forget_deleted_meta(&self, guard: &WriteGuard<'_>, content: &ScanContent, rows: &[BulkRow]) {
        let deleted: HashSet<&ModRef> = rows
            .iter()
            .filter(|row| matches!(row.result, BulkRowResult::Deleted { .. }))
            .map(|row| &row.target.mod_ref)
            .collect();
        let keys: BTreeSet<&str> = content
            .rows
            .iter()
            .filter(|row| deleted.contains(&row.mod_ref))
            .map(|row| row.key.as_str())
            .filter(|key| {
                content
                    .rows
                    .iter()
                    .filter(|other| other.key == *key)
                    .all(|other| deleted.contains(&other.mod_ref))
            })
            .collect();
        if keys.is_empty() {
            return;
        }
        let removed = self.mods.meta().update(guard, |meta| {
            for key in &keys {
                meta.remove(key);
            }
        });
        if let Err(error) = removed {
            log::error!("mod-meta.json entries of deleted mods not removed: {error}");
        }
    }
}

fn prepare_row(content: &ScanContent, target: &ModTarget) -> PreparedRow {
    let checked = resolve_managed(content, target)
        .and_then(|managed| managed.check(ManageAction::Delete).map(|()| managed));
    match checked {
        Ok(managed) => {
            let (name, size) = delete_label(&managed);
            PreparedRow {
                target: target.clone(),
                name,
                size,
                refusal: None,
            }
        }
        Err(error) => {
            let (name, size) = resolve_managed(content, target).map_or_else(
                |_| (target.key.clone(), None),
                |managed| delete_label(&managed),
            );
            PreparedRow {
                target: target.clone(),
                name,
                size,
                refusal: Some(error),
            }
        }
    }
}
