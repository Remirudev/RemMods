//! Pure planning of one row's renames (spec §5.3, PLAN §7).

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use chrono::{SecondsFormat, Utc};

use super::error::ModsError;
use super::meta_store::DisableRecord;
use super::model::{ModKind, ModRow, ModState};
use super::toggle_files::{FileClass, ModFiles};
use crate::platform;
use crate::safe_fs::names::DISABLED_PREFIX;
use crate::safe_fs::{FsError, RenameOp};

const OFF_SUFFIX: &str = "-x";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordChange {
    Keep,
    Set(DisableRecord),
    Clear,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowPlan {
    pub ops: Vec<RenameOp>,
    pub record: RecordChange,
}

/// Err only when a name cut fails (unreachable: every cut removes bytes matched as ASCII).
pub fn plan_row(
    row: &ModRow,
    files: &ModFiles,
    enable: bool,
    record: Option<&DisableRecord>,
) -> Result<RowPlan, ModsError> {
    if enable {
        plan_enable(row, files, record)
    } else {
        plan_disable(row, files)
    }
}

/// Every recorded file is off under exactly `rel + "-x"` and none of them is live.
pub fn record_matches(record: &DisableRecord, files: &ModFiles) -> bool {
    !record.files.is_empty()
        && record.files.iter().all(|rel| {
            let off = format!("{rel}{OFF_SUFFIX}");
            let is_off = files
                .files
                .iter()
                .any(|file| file.class == FileClass::OffPayload && file.rel == off);
            let is_live = files
                .files
                .iter()
                .any(|file| file.class == FileClass::LivePayload && file.rel == *rel);
            is_off && !is_live
        })
}

fn plan_disable(row: &ModRow, files: &ModFiles) -> Result<RowPlan, ModsError> {
    let live: Vec<_> = files
        .files
        .iter()
        .filter(|file| file.class == FileClass::LivePayload)
        .collect();
    let rename_folder = is_logic(row) && !row.logic_prefixed && row.state != ModState::Disabled;
    if live.is_empty() && !rename_folder {
        return Ok(no_ops(RecordChange::Keep));
    }
    let mut ops = Vec::new();
    let mut base = files.base.clone();
    if rename_folder {
        let mut name = OsString::from(DISABLED_PREFIX);
        name.push(folder_name(&files.base)?);
        base = files.base.with_file_name(name);
        ops.push(folder_op(&files.base, &base));
    }
    for file in &live {
        let from = rebase(&file.path, &files.base, &base)?;
        let mut name = file_name(&from)?.to_os_string();
        name.push(OFF_SUFFIX);
        let to = from.with_file_name(name);
        ops.push(file_op(from, to));
    }
    let record = if live.is_empty() {
        RecordChange::Keep
    } else {
        RecordChange::Set(DisableRecord {
            files: live.iter().map(|file| file.rel.clone()).collect(),
            at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        })
    };
    Ok(RowPlan { ops, record })
}

fn plan_enable(
    row: &ModRow,
    files: &ModFiles,
    record: Option<&DisableRecord>,
) -> Result<RowPlan, ModsError> {
    let targets: Vec<_> = match record.filter(|record| record_matches(record, files)) {
        Some(record) => files
            .files
            .iter()
            .filter(|file| {
                file.class == FileClass::OffPayload
                    && record
                        .files
                        .iter()
                        .any(|rel| file.rel.strip_suffix(OFF_SUFFIX) == Some(rel.as_str()))
            })
            .collect(),
        None => files
            .files
            .iter()
            .filter(|file| file.class != FileClass::LivePayload)
            .collect(),
    };
    let rename_folder = is_logic(row) && row.logic_prefixed;
    if targets.is_empty() && !rename_folder {
        return Ok(no_ops(RecordChange::Clear));
    }
    let mut ops = Vec::new();
    let mut base = files.base.clone();
    if rename_folder {
        let name = cut(folder_name(&files.base)?, DISABLED_PREFIX.len(), 0)?;
        base = files.base.with_file_name(name);
        ops.push(folder_op(&files.base, &base));
    }
    for file in targets {
        let from = rebase(&file.path, &files.base, &base)?;
        let name = cut(file_name(&from)?, 0, OFF_SUFFIX.len())?;
        let to = from.with_file_name(name);
        ops.push(file_op(from, to));
    }
    Ok(RowPlan {
        ops,
        record: RecordChange::Clear,
    })
}

fn is_logic(row: &ModRow) -> bool {
    row.kind == ModKind::Logic
}

fn no_ops(record: RecordChange) -> RowPlan {
    RowPlan {
        ops: Vec::new(),
        record,
    }
}

fn folder_op(from: &Path, to: &Path) -> RenameOp {
    RenameOp {
        from: from.to_path_buf(),
        to: to.to_path_buf(),
        is_dir: true,
    }
}

fn file_op(from: PathBuf, to: PathBuf) -> RenameOp {
    RenameOp {
        from,
        to,
        is_dir: false,
    }
}

/// `path` below `old_base` moved below `new_base` (the folder op runs first).
fn rebase(path: &Path, old_base: &Path, new_base: &Path) -> Result<PathBuf, ModsError> {
    let below = path
        .strip_prefix(old_base)
        .map_err(|_| plan_bug("rebase"))?;
    Ok(new_base.join(below))
}

fn folder_name(path: &Path) -> Result<&OsStr, ModsError> {
    path.file_name().ok_or_else(|| plan_bug("folder_name"))
}

fn file_name(path: &Path) -> Result<&OsStr, ModsError> {
    path.file_name().ok_or_else(|| plan_bug("file_name"))
}

fn cut(name: &OsStr, prefix: usize, suffix: usize) -> Result<OsString, ModsError> {
    platform::os_str_cut(name, prefix, suffix).ok_or_else(|| plan_bug("name_cut"))
}

fn plan_bug(reason: &'static str) -> ModsError {
    ModsError::Fs(FsError::InvalidBatch { reason })
}
