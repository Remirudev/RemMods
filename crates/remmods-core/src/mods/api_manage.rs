//! The manage half of the `Core` facade (spec §5.1–§5.3): one fresh scan under the write guard,
//! the matrix, then one mutation; a fresh snapshot is always published afterwards.

use std::ffi::OsString;

use chrono::Local;

use super::error::ModsError;
use super::listing::Lister;
use super::manage_target::{ManageAction, Managed, plain_path, resolve_managed};
use super::model::{ModRow, ScanContent, ScanInput};
use super::rename::{RekeyReport, RenameOutcome, RenameRequest, new_key, plan_rename};
use super::service::Refreshed;
use super::trash::{TrashOutcome, TrashRequest, trash_items};
use crate::safe_fs::{BatchOptions, FsError, RetryMode, Scope, WriteGuard};
use crate::{Core, CoreError};

const RENAME_LABEL: &str = "rename";

impl Core {
    pub fn mods_rename(&self, request: &RenameRequest) -> Result<RenameOutcome, CoreError> {
        let ((old_key, new_key, rekey), refreshed) =
            self.run_managed(|guard, input, content| {
                self.rename_under_guard(guard, input, content, request)
            })?;
        let new_ref = refreshed
            .snapshot
            .content
            .rows
            .iter()
            .find(|row| row.key == new_key)
            .map(|row| row.mod_ref.clone());
        Ok(RenameOutcome {
            refreshed,
            old_key,
            new_key,
            new_ref,
            rekey,
        })
    }

    pub fn mods_trash(&self, request: &TrashRequest) -> Result<TrashOutcome, CoreError> {
        let ((key, items, kind), refreshed) = self.run_managed(|guard, input, content| {
            let managed = resolve_managed(content, &request.target)?;
            managed.check(ManageAction::Trash)?;
            self.running_retry(request.apply_while_running)?;
            let items = trash_items(&managed, input)?;
            let size = match &managed {
                Managed::Row(row) => row.size,
                Managed::Archive(entry) => Some(entry.size),
                Managed::Leftover(_) | Managed::Unmanaged(_) => None,
            };
            let fault = self.take_batch_fault();
            let report = guard
                .trash_with_fault(&self.trash_env, &items, Local::now(), fault.as_ref())
                .map_err(|error| match error {
                    FsError::TrashUnavailable { reason } => {
                        ModsError::TrashUnavailable { reason, size }
                    }
                    other => ModsError::Fs(other),
                })?;
            let count = u32::try_from(report.items.len()).unwrap_or(u32::MAX);
            Ok((managed.key().to_owned(), count, report.kind))
        })?;
        Ok(TrashOutcome {
            refreshed,
            key,
            items,
            kind,
        })
    }

    /// Write guard, marker, fresh scan, `work`; then always a fresh snapshot (P2 §5.5 pattern).
    pub(crate) fn run_managed<T>(
        &self,
        work: impl FnOnce(&WriteGuard<'_>, &ScanInput, &ScanContent) -> Result<T, CoreError>,
    ) -> Result<(T, Refreshed), CoreError> {
        let input = self.mods_scan_input()?;
        let guard = self.ops.write(self.manage_scope(&input)?)?;
        self.check_marker(&input)?;
        let result = self
            .mods
            .refresh(&input)
            .map_err(CoreError::from)
            .and_then(|fresh| work(&guard, &input, &fresh.snapshot.content));
        let refreshed = self.mods.refresh(&input);
        match (result, refreshed) {
            (Ok(value), Ok(refreshed)) => Ok((value, refreshed)),
            (Ok(_), Err(error)) => Err(error.into()),
            (Err(error), refreshed) => {
                if let Err(refresh_error) = refreshed {
                    log::warn!("refresh after a failed manage action failed: {refresh_error}");
                }
                Err(error)
            }
        }
    }

    pub(crate) fn manage_scope(&self, input: &ScanInput) -> Result<Scope, CoreError> {
        let app_dirs = [
            self.paths.data_dir.as_path(),
            self.paths.config_dir.as_path(),
        ];
        Ok(Scope::for_paks(&input.paks, &app_dirs)?)
    }

    /// `GameRunning` unless the user chose to apply anyway; then renames are not retried.
    pub(crate) fn running_retry(&self, apply_while_running: bool) -> Result<RetryMode, ModsError> {
        let running = self.game_running_now();
        if running && !apply_while_running {
            return Err(ModsError::GameRunning);
        }
        Ok(if running {
            RetryMode::Never
        } else {
            RetryMode::Normal
        })
    }

    pub(crate) fn batch_options(&self, label: &'static str, retry: RetryMode) -> BatchOptions {
        BatchOptions {
            label,
            retry,
            fault: self.take_batch_fault(),
        }
    }

    fn rename_under_guard(
        &self,
        guard: &WriteGuard<'_>,
        input: &ScanInput,
        content: &ScanContent,
        request: &RenameRequest,
    ) -> Result<(String, String, RekeyReport), CoreError> {
        let managed = resolve_managed(content, &request.target)?;
        managed.check(ManageAction::Rename)?;
        let Managed::Row(row) = managed else {
            return Err(ModsError::Stale.into());
        };
        let retry = self.running_retry(request.apply_while_running)?;
        let entries = root_entries(row)?;
        let other_keys: Vec<&str> = content
            .rows
            .iter()
            .filter(|other| !std::ptr::eq(*other, row))
            .map(|other| other.key.as_str())
            .collect();
        let mut plain = row.clone();
        plain.path = plain_path(&row.path, input)?;
        let op = plan_rename(&plain, &entries, &other_keys, &request.new_name)?;
        guard.rename_batch(&[op], &self.batch_options(RENAME_LABEL, retry))?;
        let new_key = new_key(&row.key, &request.new_name);
        let rekey = if row.ambiguous {
            RekeyReport {
                skipped_ambiguous: true,
                ..RekeyReport::default()
            }
        } else {
            self.rekey(guard, &row.key, &new_key)
        };
        Ok((row.key.clone(), new_key, rekey))
    }
}

/// The names in the row's root folder.
fn root_entries(row: &ModRow) -> Result<Vec<OsString>, ModsError> {
    let root = row.path.parent().ok_or(ModsError::Stale)?;
    let entries = Lister::default()
        .list(root)
        .map_err(|error| FsError::io("list", root, error))?;
    Ok(entries.into_iter().map(|entry| entry.name).collect())
}
