//! Renaming a mod folder (spec §5.2): pure planning; `api_manage.rs` runs it.

use std::ffi::{OsStr, OsString};

use super::error::ModsError;
use super::manage_target::ModTarget;
use super::model::{ModKind, ModRow};
use super::modref::ModRef;
use super::service::Refreshed;
use crate::safe_fs::RenameOp;
use crate::safe_fs::names::{
    DISABLED_PREFIX, NAME_MAX_BYTES, NameIssue, fold_key, key_text, validate_name,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenameRequest {
    pub target: ModTarget,
    pub new_name: String,
    pub apply_while_running: bool,
}

/// new_ref: the renamed row in the fresh snapshot (None if the rescan did not find it).
#[derive(Clone, Debug)]
pub struct RenameOutcome {
    pub refreshed: Refreshed,
    pub old_key: String,
    pub new_key: String,
    pub new_ref: Option<ModRef>,
    pub rekey: RekeyReport,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RekeyReport {
    pub profiles: u32,
    pub meta: bool,
    pub skipped_ambiguous: bool,
    pub failed: bool,
}

/// Rules 2–6 of §5.2. `root_entries`: the names in the row's root folder (the row's own folder
/// included); `other_keys`: the keys of every other row in the snapshot.
pub fn plan_rename(
    row: &ModRow,
    root_entries: &[OsString],
    other_keys: &[&str],
    new_name: &str,
) -> Result<RenameOp, ModsError> {
    if new_name == row.name {
        return Err(ModsError::RenameUnchanged);
    }
    validate_name(new_name).map_err(invalid)?;
    let prefixed = row.kind == ModKind::Logic && row.logic_prefixed;
    let target = if prefixed {
        format!("{DISABLED_PREFIX}{new_name}")
    } else {
        new_name.to_owned()
    };
    if target.len() > NAME_MAX_BYTES {
        return Err(invalid(NameIssue::TooLong));
    }
    let own = row.path.file_name();
    let mut forbidden = vec![fold_key(&target)];
    if row.kind == ModKind::Logic {
        let other_form = if prefixed {
            new_name.to_owned()
        } else {
            format!("{DISABLED_PREFIX}{new_name}")
        };
        forbidden.push(fold_key(&other_form));
    }
    let taken = root_entries
        .iter()
        .filter(|entry| Some(entry.as_os_str()) != own)
        .any(|entry| forbidden.contains(&fold_key(&key_text(entry))));
    if taken {
        return Err(ModsError::TargetExists);
    }
    let new_key = new_key(&row.key, new_name);
    let new_fold = fold_key(&new_key);
    if other_keys.iter().any(|key| fold_key(key) == new_fold) {
        return Err(ModsError::TargetExists);
    }
    Ok(RenameOp {
        from: row.path.clone(),
        to: row.path.with_file_name(OsStr::new(&target)),
        is_dir: true,
    })
}

/// `old_key` with `new_name` as its last part: only the mod's own folder changes name.
pub(crate) fn new_key(old_key: &str, new_name: &str) -> String {
    let (prefix, name) = old_key.split_once(':').unwrap_or((old_key, ""));
    match name.rsplit_once('/') {
        Some((author, _)) => format!("{prefix}:{author}/{new_name}"),
        None => format!("{prefix}:{new_name}"),
    }
}

fn invalid(issue: NameIssue) -> ModsError {
    ModsError::InvalidName { code: issue.code() }
}
