//! What applying a profile would change (spec §5.8). Pure on the snapshot.

use std::collections::BTreeMap;

use super::capture::is_on;
use super::model::{EntryState, Profile, ProfileMode};
use crate::mods::{ModRef, ModRow, ScanContent, ToggleBlock};
use crate::safe_fs::names::order_key;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeReason {
    Listed,
    Unlisted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffChange {
    pub mod_ref: ModRef,
    pub key: String,
    pub name: String,
    pub enable: bool,
    pub reason: ChangeReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipReason {
    Ambiguous,
    Linked,
    UnknownState,
    NoPakFiles,
    InvalidLogicName,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffSkip {
    pub key: String,
    pub name: Option<String>,
    pub wanted: EntryState,
    pub reason: SkipReason,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileDiff {
    pub profile_id: String,
    pub mode: ProfileMode,
    /// Sorted: enables first, then by order_key(key).
    pub changes: Vec<DiffChange>,
    /// Listed keys with no row, sorted.
    pub missing: Vec<String>,
    pub skipped: Vec<DiffSkip>,
    pub unchanged: u32,
    /// Exact identity of `changes`: "<+|-><key>\n" per change in order. Compared by apply.
    pub fingerprint: String,
}

impl From<ToggleBlock> for SkipReason {
    fn from(block: ToggleBlock) -> Self {
        match block {
            ToggleBlock::Ambiguous => Self::Ambiguous,
            ToggleBlock::Linked => Self::Linked,
            ToggleBlock::UnknownState => Self::UnknownState,
            ToggleBlock::NoPakFiles => Self::NoPakFiles,
            ToggleBlock::InvalidLogicName => Self::InvalidLogicName,
        }
    }
}

pub fn diff_profile(profile: &Profile, content: &ScanContent) -> ProfileDiff {
    let mut by_key: BTreeMap<&str, Vec<&ModRow>> = BTreeMap::new();
    for row in &content.rows {
        by_key.entry(row.key.as_str()).or_default().push(row);
    }
    let mut diff = ProfileDiff {
        profile_id: profile.id.clone(),
        mode: profile.mode,
        changes: Vec::new(),
        missing: Vec::new(),
        skipped: Vec::new(),
        unchanged: 0,
        fingerprint: String::new(),
    };
    for (key, entry) in &profile.mods {
        let wanted = entry.state;
        match by_key.get(key.as_str()).map(Vec::as_slice) {
            None | Some([]) => diff.missing.push(key.clone()),
            Some([row]) => {
                if (wanted == EntryState::Enabled) == is_on(row) {
                    diff.unchanged += 1;
                } else {
                    push_change(&mut diff, row, wanted, ChangeReason::Listed);
                }
            }
            Some(rows) => diff
                .skipped
                .push(skip(key, rows, wanted, SkipReason::Ambiguous)),
        }
    }
    if profile.mode == ProfileMode::Exclusive {
        for (key, rows) in &by_key {
            if profile.mods.contains_key(*key) || !rows.iter().any(|row| is_on(row)) {
                continue;
            }
            match rows.as_slice() {
                [row] => push_change(&mut diff, row, EntryState::Disabled, ChangeReason::Unlisted),
                _ => {
                    diff.skipped
                        .push(skip(key, rows, EntryState::Disabled, SkipReason::Ambiguous))
                }
            }
        }
    }
    diff.changes
        .sort_by_cached_key(|change| (!change.enable, order_key(&change.key)));
    diff.missing.sort_by_cached_key(|key| order_key(key));
    diff.fingerprint = diff
        .changes
        .iter()
        .map(|change| format!("{}{}\n", if change.enable { '+' } else { '-' }, change.key))
        .collect();
    diff
}

fn push_change(diff: &mut ProfileDiff, row: &ModRow, wanted: EntryState, reason: ChangeReason) {
    if let Some(block) = row.toggle_block {
        diff.skipped.push(DiffSkip {
            key: row.key.clone(),
            name: Some(row.name.clone()),
            wanted,
            reason: block.into(),
        });
        return;
    }
    diff.changes.push(DiffChange {
        mod_ref: row.mod_ref.clone(),
        key: row.key.clone(),
        name: row.name.clone(),
        enable: wanted == EntryState::Enabled,
        reason,
    });
}

fn skip(key: &str, rows: &[&ModRow], wanted: EntryState, reason: SkipReason) -> DiffSkip {
    DiffSkip {
        key: key.to_owned(),
        name: rows.first().map(|row| row.name.clone()),
        wanted,
        reason,
    }
}
