//! The commit batch and what follows it (spec §5.4 steps 6–7): set-aside of a replaced mod,
//! staging → final name, `-x` strip of the journaled inert files; then mod-meta.json and the
//! Trash.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use chrono::Local;

use super::model::{KeptReason, ReplacedOld};
use crate::mods::{ModRow, ModsError, ScanInput, list_toggle_files, plan_set_aside};
use crate::platform::TrashEnv;
use crate::safe_fs::{FsError, RenameOp, StagingDir, WriteGuard};

const OFF_SUFFIX: &str = "-x";

/// One pick's part of the commit batch.
pub(crate) struct PickOps {
    pub(crate) ops: Vec<RenameOp>,
    /// The set-aside folder of a replaced mod.
    pub(crate) aside: Option<PathBuf>,
}

/// (a) the old row set aside (when replacing), (b) staging → `<root>/<name>`, (c) every inert
/// file back to its live name below the new folder. Folders come before what lies below them.
pub(crate) fn pick_ops(
    staging: &StagingDir,
    inert: &[PathBuf],
    target: &Path,
    replaced: Option<(&ModRow, &ScanInput)>,
) -> Result<PickOps, ModsError> {
    let mut ops = Vec::new();
    let mut aside = None;
    if let Some((row, input)) = replaced {
        let files = list_toggle_files(row, input)?;
        let suffix = uuid::Uuid::new_v4().simple().to_string();
        let (set_aside, folder) = plan_set_aside(row, &files, &suffix)?;
        ops.extend(set_aside);
        aside = Some(folder);
    }
    ops.push(RenameOp {
        from: staging.path().to_path_buf(),
        to: target.to_path_buf(),
        is_dir: true,
    });
    for file in inert {
        let below = file
            .strip_prefix(staging.path())
            .map_err(|_| ModsError::Stale)?;
        let from = target.join(below);
        let to = live_name(&from).ok_or(ModsError::Stale)?;
        ops.push(RenameOp {
            from,
            to,
            is_dir: false,
        });
    }
    Ok(PickOps { ops, aside })
}

/// `x.pak-x` → `x.pak`.
fn live_name(path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?.to_str()?;
    let live = name.strip_suffix(OFF_SUFFIX)?;
    Some(path.with_file_name(OsString::from(live)))
}

/// Trashes a set-aside folder; when that is not possible it stays hidden and inert for Repair.
pub(crate) fn trash_aside(guard: &WriteGuard<'_>, env: &TrashEnv, aside: &Path) -> ReplacedOld {
    match guard.trash(env, &[aside.to_path_buf()], Local::now()) {
        Ok(_) => ReplacedOld::Trashed,
        Err(FsError::TrashUnavailable { reason }) => {
            log::warn!(
                "no usable trash ({reason:?}); {} stays for Repair",
                aside.display()
            );
            ReplacedOld::KeptInRepair {
                reason: KeptReason::TrashUnavailable,
            }
        }
        Err(error) => {
            log::warn!(
                "trashing {} failed; it stays for Repair: {error}",
                aside.display()
            );
            ReplacedOld::KeptInRepair {
                reason: KeptReason::TrashFailed,
            }
        }
    }
}
