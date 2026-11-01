//! Batch preflight: checks every op against the disk and the scope and writes nothing. Ops may
//! lie under an earlier op's target folder; they are mapped back to their pre-batch paths.

use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};

use super::FsError;
use super::batch::RenameOp;
use super::names::{fold_key, key_text};
use super::scope::{Scope, ScopeChecker, has_dot_component};
use crate::platform;

const CASE_TEMP_PREFIX: &str = "_case_tmp_";

/// The expanded ops of a batch that passed preflight; `verify[k]` asks for an exact-name check
/// after op k (the second half of a case-only pair).
pub(crate) struct Planned {
    pub(crate) ops: Vec<RenameOp>,
    pub(crate) verify: Vec<bool>,
}

/// One op's paths as they are before the batch runs.
struct Before {
    from: PathBuf,
    to: PathBuf,
}

pub(crate) fn preflight(scope: &Scope, ops: &[RenameOp]) -> Result<Planned, FsError> {
    check_shape(ops)?;
    let before = pre_batch_forms(ops);
    check_order(&before)?;
    let mut checker = ScopeChecker::new(scope);
    for op in &before {
        checker.check(&op.from)?;
        checker.check(&op.to)?;
    }
    let case_only = check_targets_distinct(&before)?;
    for op in &before {
        if !exists(&op.from) {
            return Err(FsError::SourceMissing {
                path: op.from.clone(),
            });
        }
    }
    check_targets_free(&before, &case_only)?;
    for (op, original) in before.iter().zip(ops) {
        platform::lock_probe(&op.from, original.is_dir)
            .map_err(|error| FsError::io("probe", &op.from, error))?;
    }
    Ok(expand(ops, &case_only))
}

/// Same parent, real final names, no dot components.
fn check_shape(ops: &[RenameOp]) -> Result<(), FsError> {
    let cross_folder = FsError::InvalidBatch {
        reason: "cross_folder",
    };
    for op in ops {
        let named = op.from.file_name().is_some() && op.to.file_name().is_some();
        let dotted = has_dot_component(&op.from) || has_dot_component(&op.to);
        if !named || dotted || op.from.parent() != op.to.parent() {
            return Err(cross_folder);
        }
    }
    Ok(())
}

/// Maps each op back to the paths it has before the batch: an earlier op's `to` prefix becomes
/// its `from`, latest op first.
fn pre_batch_forms(ops: &[RenameOp]) -> Vec<Before> {
    ops.iter()
        .enumerate()
        .map(|(index, op)| Before {
            from: map_back(&op.from, &ops[..index]),
            to: map_back(&op.to, &ops[..index]),
        })
        .collect()
}

fn map_back(path: &Path, earlier: &[RenameOp]) -> PathBuf {
    let mut path = path.to_path_buf();
    for op in earlier.iter().rev() {
        if let Ok(rest) = path.strip_prefix(&op.to)
            && !rest.as_os_str().is_empty()
        {
            path = op.from.join(rest);
        }
    }
    path
}

/// Folders go first: no op may lie under a later op's source.
fn check_order(before: &[Before]) -> Result<(), FsError> {
    for (index, op) in before.iter().enumerate() {
        let under_later = before[index + 1..]
            .iter()
            .any(|later| op.from != later.from && op.from.starts_with(&later.from));
        if under_later {
            return Err(FsError::InvalidBatch { reason: "order" });
        }
    }
    Ok(())
}

/// No two targets fold-equal, no target fold-equal to another op's source. Returns which ops are
/// case-only (fold-equal names whose bytes differ).
fn check_targets_distinct(before: &[Before]) -> Result<Vec<bool>, FsError> {
    let sources: Vec<_> = before.iter().map(|op| folded(&op.from)).collect();
    let mut targets = HashSet::new();
    let mut case_only = Vec::with_capacity(before.len());
    for (index, op) in before.iter().enumerate() {
        let target = folded(&op.to);
        if !targets.insert(target.clone()) {
            return Err(FsError::InvalidBatch {
                reason: "duplicate_target",
            });
        }
        let chained = sources
            .iter()
            .enumerate()
            .any(|(other, source)| other != index && *source == target);
        if chained {
            return Err(FsError::InvalidBatch { reason: "chained" });
        }
        let source_repeated = sources[index + 1..].contains(&sources[index]);
        if source_repeated {
            return Err(FsError::InvalidBatch {
                reason: "duplicate_source",
            });
        }
        case_only.push(sources[index] == target && op.from != op.to);
    }
    Ok(case_only)
}

/// Each target absent, and its folder holds no fold-equal entry except the op's own source.
fn check_targets_free(before: &[Before], case_only: &[bool]) -> Result<(), FsError> {
    let mut listings: HashMap<PathBuf, Vec<(String, OsString)>> = HashMap::new();
    for (op, &case_only) in before.iter().zip(case_only) {
        let taken = || FsError::TargetExists {
            path: op.to.clone(),
        };
        if !case_only && exists(&op.to) {
            return Err(taken());
        }
        let (Some(parent), Some(target_name), Some(source_name)) =
            (op.to.parent(), op.to.file_name(), op.from.file_name())
        else {
            return Err(FsError::InvalidBatch {
                reason: "cross_folder",
            });
        };
        let listing = match listings.get(parent) {
            Some(listing) => listing,
            None => {
                let listing = list_folded(parent)?;
                listings.entry(parent.to_path_buf()).or_insert(listing)
            }
        };
        let target_key = fold_key(&key_text(target_name));
        let clash = listing
            .iter()
            .any(|(key, name)| *key == target_key && name.as_os_str() != source_name);
        if clash {
            return Err(taken());
        }
    }
    Ok(())
}

pub(crate) fn list_folded(folder: &Path) -> Result<Vec<(String, OsString)>, FsError> {
    let entries = platform::fs_path(folder)
        .and_then(fs::read_dir)
        .map_err(|error| FsError::io("list", folder, error))?;
    let mut listing = Vec::new();
    for entry in entries {
        let name = entry
            .map_err(|error| FsError::io("list", folder, error))?
            .file_name();
        listing.push((fold_key(&key_text(&name)), name));
    }
    Ok(listing)
}

/// Case-only ops become `from → _case_tmp_<32 hex>` and `tmp → to`.
fn expand(ops: &[RenameOp], case_only: &[bool]) -> Planned {
    let mut planned = Planned {
        ops: Vec::with_capacity(ops.len()),
        verify: Vec::with_capacity(ops.len()),
    };
    for (op, &case_only) in ops.iter().zip(case_only) {
        if !case_only {
            planned.ops.push(op.clone());
            planned.verify.push(false);
            continue;
        }
        let mut temp_name = OsString::from(CASE_TEMP_PREFIX);
        temp_name.push(uuid::Uuid::new_v4().simple().to_string());
        let temp = op.from.with_file_name(&temp_name);
        planned.ops.push(RenameOp {
            from: op.from.clone(),
            to: temp.clone(),
            is_dir: op.is_dir,
        });
        planned.ops.push(RenameOp {
            from: temp,
            to: op.to.clone(),
            is_dir: op.is_dir,
        });
        planned.verify.extend([false, true]);
    }
    planned
}

fn folded(path: &Path) -> (Option<PathBuf>, String) {
    let name = path.file_name().unwrap_or(OsStr::new(""));
    (
        path.parent().map(Path::to_path_buf),
        fold_key(&key_text(name)),
    )
}

fn exists(path: &Path) -> bool {
    platform::fs_path(path)
        .and_then(fs::symlink_metadata)
        .is_ok()
}
