//! Bulk trash and the shared bulk types (P3b §5.3–§5.4): one call, one write guard, one fresh
//! scan, one final snapshot; per-row results in request order. The bulk delete is in
//! `bulk_delete.rs`.

use std::collections::HashSet;

use super::error::ModsError;
use super::manage_target::{ManageAction, Managed, ModTarget, resolve_managed};
use super::model::{ScanContent, ScanInput};
use super::service::Refreshed;
use super::trash::trash_managed;
use crate::platform::{TrashKind, TrashUnavailableReason};
use crate::safe_fs::{FaultPlan, FsError, WriteGuard};
use crate::{Core, CoreError};

pub const BULK_MAX_TARGETS: usize = 1000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BulkTrashRequest {
    pub targets: Vec<ModTarget>,
    pub apply_while_running: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BulkDeleteRequest {
    pub targets: Vec<ModTarget>,
    pub token: String,
    pub apply_while_running: bool,
}

/// `refusal`: why this row cannot be deleted (Stale, ManageBlocked); None = covered by the token.
#[derive(Debug)]
pub struct PreparedRow {
    pub target: ModTarget,
    pub name: String,
    pub size: Option<u64>,
    pub refusal: Option<ModsError>,
}

/// `token`: None when no row is eligible; it covers exactly the rows with `refusal == None`, in
/// order.
#[derive(Debug)]
pub struct BulkDeletePrepared {
    pub token: Option<String>,
    pub rows: Vec<PreparedRow>,
    pub expires_in_secs: u32,
}

#[derive(Debug)]
pub enum BulkRowResult {
    Trashed {
        items: u32,
        kind: TrashKind,
    },
    /// `leftover`: rel path of a hidden, inert folder the removal left behind (Repair lists it).
    Deleted {
        leftover: Option<String>,
    },
    /// Trash only; nothing changed for this row. The UI offers "Delete permanently".
    TrashUnavailable {
        reason: TrashUnavailableReason,
        size: Option<u64>,
    },
    /// Refused or failed; nothing changed for this row unless the error is a Batch with `stuck`
    /// or the "partially recycled" Io error. A delete whose set-aside went through is never
    /// Failed: it is `Deleted { leftover }`.
    Failed(CoreError),
    /// Not attempted: the user cancelled a folder-busy wait on an earlier row.
    NotRun,
}

/// `name`: the row's display name in the fresh scan (the key when it was not found).
#[derive(Debug)]
pub struct BulkRow {
    pub target: ModTarget,
    pub name: String,
    pub result: BulkRowResult,
}

/// `refreshed`: None when the final rescan failed after rows ran (logged); the rows still count.
#[derive(Debug)]
pub struct BulkOutcome {
    pub refreshed: Option<Refreshed>,
    pub rows: Vec<BulkRow>,
}

impl Core {
    /// Moves several entries to the Trash under one write guard. Whole-call errors: the target
    /// list, Busy, the smoke marker, the first scan and GameRunning (asked once); everything
    /// else is per row. Profiles and `mod-meta.json` keep the keys (the Trash can give the mods
    /// back).
    pub fn mods_trash_many(&self, request: &BulkTrashRequest) -> Result<BulkOutcome, CoreError> {
        validate_targets(&request.targets)?;
        let (rows, refreshed) = self.run_managed_bulk(|guard, input, content| {
            self.running_retry(request.apply_while_running)?;
            let mut rows = Vec::with_capacity(request.targets.len());
            let mut stopped = false;
            for target in &request.targets {
                let fault = self.take_batch_fault();
                if stopped {
                    rows.push(not_run(content, target));
                    continue;
                }
                rows.push(self.trash_row(guard, input, content, target, fault));
                stopped = self.ops.busy().cancel_requested();
            }
            Ok(rows)
        })?;
        Ok(BulkOutcome { refreshed, rows })
    }

    fn trash_row(
        &self,
        guard: &WriteGuard<'_>,
        input: &ScanInput,
        content: &ScanContent,
        target: &ModTarget,
        fault: Option<FaultPlan>,
    ) -> BulkRow {
        let managed = match resolve_managed(content, target) {
            Ok(managed) => managed,
            Err(error) => return failed(target, target.key.clone(), error.into()),
        };
        let name = display_name(&managed);
        let trashed = managed
            .check(ManageAction::Trash)
            .and_then(|()| trash_managed(guard, &self.trash_env, input, &managed, fault.as_ref()));
        let result = match trashed {
            Ok((items, kind)) => BulkRowResult::Trashed { items, kind },
            Err(ModsError::TrashUnavailable { reason, size }) => {
                BulkRowResult::TrashUnavailable { reason, size }
            }
            Err(error) => BulkRowResult::Failed(error.into()),
        };
        BulkRow {
            target: target.clone(),
            name,
            result,
        }
    }
}

/// Non-empty, at most `BULK_MAX_TARGETS`, no mod_ref twice; else `BulkTargets`.
pub(super) fn validate_targets(targets: &[ModTarget]) -> Result<(), ModsError> {
    if targets.is_empty() || targets.len() > BULK_MAX_TARGETS {
        return Err(ModsError::BulkTargets);
    }
    let mut seen = HashSet::with_capacity(targets.len());
    if targets.iter().all(|target| seen.insert(&target.mod_ref)) {
        Ok(())
    } else {
        Err(ModsError::BulkTargets)
    }
}

/// A row's name as the single prepare shows it; other entries show their key.
pub(super) fn display_name(managed: &Managed<'_>) -> String {
    match managed {
        Managed::Row(row) => row.name.clone(),
        other => other.key().to_owned(),
    }
}

pub(super) fn not_run(content: &ScanContent, target: &ModTarget) -> BulkRow {
    let name = resolve_managed(content, target)
        .map_or_else(|_| target.key.clone(), |managed| display_name(&managed));
    BulkRow {
        target: target.clone(),
        name,
        result: BulkRowResult::NotRun,
    }
}

pub(super) fn failed(target: &ModTarget, name: String, error: CoreError) -> BulkRow {
    BulkRow {
        target: target.clone(),
        name,
        result: BulkRowResult::Failed(error),
    }
}

/// The user cancelled a folder-busy wait (bare, or as the cause of a rolled-back batch).
pub(super) fn is_busy_cancelled(error: &CoreError) -> bool {
    let fs = match error {
        CoreError::Fs(fs) | CoreError::Mods(ModsError::Fs(fs)) => fs,
        _ => return false,
    };
    match fs {
        FsError::BusyCancelled { .. } => true,
        FsError::Batch(batch) => matches!(batch.cause, FsError::BusyCancelled { .. }),
        _ => false,
    }
}
