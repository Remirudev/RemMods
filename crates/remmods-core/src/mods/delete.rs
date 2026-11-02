//! Permanent delete with a second confirmation (spec §5.4): set the folder aside (one batch,
//! inert and hidden), then remove it. Nothing is removed without a valid token.

use std::path::{Path, PathBuf};
use std::time::Instant;

use super::delete_token::DeleteTokens;
use super::details::resolve_ref;
use super::error::ModsError;
use super::manage_target::{
    ManageAction, ManageBlock, Managed, ModTarget, plain_path, resolve_managed,
};
use super::model::{ModRow, ScanContent, ScanInput};
use super::service::Refreshed;
use super::toggle_files::{FileClass, ModFiles, list_toggle_files};
use crate::safe_fs::{
    BatchOptions, FaultPlan, FsError, RenameOp, RetryMode, WriteGuard, set_aside_name,
};
use crate::{Core, CoreError};

const SET_ASIDE_LABEL: &str = "set_aside";
const OFF_SUFFIX: &str = "-x";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeletePrepared {
    pub token: String,
    pub key: String,
    pub name: String,
    pub size: Option<u64>,
    pub expires_in_secs: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeleteRequest {
    pub target: ModTarget,
    pub token: String,
    pub apply_while_running: bool,
}

#[derive(Clone, Debug)]
pub struct DeleteOutcome {
    pub refreshed: Refreshed,
    pub key: String,
    /// Rel path of what is left (shown in Repair).
    pub leftover: Option<String>,
}

impl Core {
    /// First step, after the UI's first confirm: resolves on a fresh scan (read guard), checks the
    /// matrix, issues a token. Writes nothing.
    pub fn mods_delete_prepare(&self, target: &ModTarget) -> Result<DeletePrepared, CoreError> {
        let fresh = self.mods_refresh()?;
        let managed = resolve_managed(&fresh.snapshot.content, target)?;
        managed.check(ManageAction::Delete)?;
        let (name, size) = delete_label(&managed);
        let key = managed.key().to_owned();
        let token = self
            .delete_tokens
            .issue(&target.mod_ref, &key, Instant::now());
        Ok(DeletePrepared {
            token,
            key,
            name,
            size,
            expires_in_secs: u32::try_from(DeleteTokens::TTL.as_secs()).unwrap_or(u32::MAX),
        })
    }

    /// Second step, after the UI's second confirm.
    pub fn mods_delete_permanently(
        &self,
        request: &DeleteRequest,
    ) -> Result<DeleteOutcome, CoreError> {
        let target = &request.target;
        self.delete_tokens
            .consume(&request.token, &target.mod_ref, &target.key, Instant::now())?;
        let (leftover, refreshed) = self.run_managed(|guard, input, content| {
            self.delete_under_guard(guard, input, content, request)
        })?;
        Ok(DeleteOutcome {
            refreshed,
            key: target.key.clone(),
            leftover,
        })
    }

    fn delete_under_guard(
        &self,
        guard: &WriteGuard<'_>,
        input: &ScanInput,
        content: &ScanContent,
        request: &DeleteRequest,
    ) -> Result<Option<String>, CoreError> {
        let managed = resolve_managed(content, &request.target)?;
        managed.check(ManageAction::Delete)?;
        let retry = self.running_retry(request.apply_while_running)?;
        let fault = match managed {
            Managed::Row(_) => self.take_batch_fault(),
            _ => None,
        };
        let leftover = self.delete_row(guard, input, &managed, retry, fault)?;
        if let Managed::Row(row) = managed {
            self.forget_meta(guard, content, row);
        }
        Ok(leftover)
    }

    /// One checked entry (P3 §5.4 steps 3–5): a row is set aside in its own journaled batch
    /// (all or none), then removed; a leftover is removed directly. Once a row's set-aside went
    /// through, the mod is hidden and inert, so a failed removal is logged and reported as the
    /// leftover, never as an error.
    pub(crate) fn delete_row(
        &self,
        guard: &WriteGuard<'_>,
        input: &ScanInput,
        managed: &Managed<'_>,
        retry: RetryMode,
        fault: Option<FaultPlan>,
    ) -> Result<Option<String>, CoreError> {
        let options = BatchOptions {
            label: SET_ASIDE_LABEL,
            retry,
            fault,
        };
        let aside = match managed {
            Managed::Row(row) => {
                let files = list_toggle_files(row, input)?;
                let suffix = uuid::Uuid::new_v4().simple().to_string();
                let (ops, aside) = plan_set_aside(row, &files, &suffix)?;
                guard.rename_batch(&ops, &options)?;
                return Ok(match guard.remove_aside(&aside) {
                    Ok(()) => None,
                    Err(error) => {
                        log::error!("permanent delete left a hidden, inert folder behind: {error}");
                        Some(rel_display(&aside, &input.paks))
                    }
                });
            }
            Managed::Leftover(entry) => plain_path(&resolve_ref(input, &entry.mod_ref)?, input)?,
            Managed::Archive(_) | Managed::Unmanaged(_) => return Err(ModsError::Stale.into()),
        };
        match guard.remove_aside(&aside) {
            Ok(()) => Ok(None),
            Err(FsError::AsideJournaled { .. }) => Err(ModsError::ManageBlocked {
                key: managed.key().to_owned(),
                action: ManageAction::Delete,
                reason: ManageBlock::JournalPending,
            }
            .into()),
            Err(error @ (FsError::Io { .. } | FsError::Batch(_))) => {
                log::error!("permanent delete left a hidden, inert folder behind: {error}");
                Ok(Some(rel_display(&aside, &input.paks)))
            }
            Err(error) => Err(error.into()),
        }
    }

    /// Drops the `mod-meta.json` entry unless another row still has the key; logged on failure.
    fn forget_meta(&self, guard: &WriteGuard<'_>, content: &ScanContent, row: &ModRow) {
        let shared = content
            .rows
            .iter()
            .filter(|other| other.key == row.key)
            .count()
            > 1;
        if shared {
            return;
        }
        if let Err(error) = self.mods.meta().update(guard, |meta| meta.remove(&row.key)) {
            log::error!("mod-meta.json entry of a deleted mod not removed: {error}");
        }
    }
}

/// The name and size a delete confirmation shows: a row's name and size, else the key.
pub(crate) fn delete_label(managed: &Managed<'_>) -> (String, Option<u64>) {
    match managed {
        Managed::Row(row) => (row.name.clone(), row.size),
        other => (other.key().to_owned(), None),
    }
}

/// The folder is renamed to a hidden temp name first (the batch wants folders before what lies
/// below them), then every live payload file inside it gets `-x`. Returns the ops and the
/// set-aside folder (plain form). Pure.
pub fn plan_set_aside(
    row: &ModRow,
    files: &ModFiles,
    suffix: &str,
) -> Result<(Vec<RenameOp>, PathBuf), ModsError> {
    let base = &files.base;
    let label = base.file_name().ok_or(ModsError::Stale)?;
    let parent = base.parent().ok_or(ModsError::Stale)?;
    if row.path.file_name() != Some(label) {
        return Err(ModsError::Stale);
    }
    let aside = parent.join(set_aside_name(label, suffix));
    let mut ops = vec![RenameOp {
        from: base.clone(),
        to: aside.clone(),
        is_dir: true,
    }];
    for file in &files.files {
        if file.class != FileClass::LivePayload {
            continue;
        }
        let below = file.path.strip_prefix(base).map_err(|_| ModsError::Stale)?;
        let from = aside.join(below);
        let mut name = from.as_os_str().to_os_string();
        name.push(OFF_SUFFIX);
        ops.push(RenameOp {
            from,
            to: PathBuf::from(name),
            is_dir: false,
        });
    }
    Ok((ops, aside))
}

/// `Root/name` below Paks, `/`-joined, lossy (display only).
pub(crate) fn rel_display(path: &Path, paks: &Path) -> String {
    path.strip_prefix(paks)
        .unwrap_or(path)
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}
