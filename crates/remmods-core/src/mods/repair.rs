//! The Repair tab's list (spec §5.6): pure on the snapshot; every action reuses an existing
//! command.

use std::collections::HashSet;
use std::ffi::OsStr;

use super::issues::ModIssue;
use super::manage_target::ModTarget;
use super::model::{ModKind, ModRow, ScanContent, UnmanagedKind};
use crate::safe_fs::names::{
    NAME_MAX_BYTES, fold_key, has_disabled_prefix_ascii, is_bidi_control, is_install_temp_name,
    is_reserved_device_name, validate_name,
};
use crate::{Core, CoreError};

const FORBIDDEN_CHARS: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
const DISABLED_PREFIX_LEN: usize = 10;
const FALLBACK_NAME: &str = "Mod";
const JOURNAL_FILE: &str = "journal.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepairKind {
    Leftover,
    AmbiguousKey,
    InvalidName,
    NonUnicodeName,
    LogicDisabledButLoaded,
    LogicEnabledNotLoaded,
    TempFolderLive,
    StuckJournal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepairAction {
    Rename,
    Trash,
    DeletePermanently,
    Reveal,
    FinishDisable,
    FinishEnable,
    OpenLogs,
}

/// id: stable within a snapshot, "<kind>|<modRef>" or "stuck"; label: display name or rel path;
/// suggested_name (InvalidName/NonUnicodeName): passes validate_name and does not collide.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepairItem {
    pub id: String,
    pub kind: RepairKind,
    pub target: Option<ModTarget>,
    pub label: String,
    pub actions: Vec<RepairAction>,
    pub suggested_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepairList {
    pub revision: u32,
    pub items: Vec<RepairItem>,
}

impl Core {
    pub fn repair_list(&self) -> Result<RepairList, CoreError> {
        let snapshot = self.mods_list()?;
        let stuck = stuck_journal_entries(&self.paths.data_dir.join(JOURNAL_FILE));
        Ok(RepairList {
            revision: snapshot.revision,
            items: repair_items(&snapshot.content, stuck),
        })
    }
}

/// Entries recovery marked stuck; an unreadable journal counts as none (it is logged by the gate).
fn stuck_journal_entries(path: &std::path::Path) -> u32 {
    let Ok(bytes) = std::fs::read(path) else {
        return 0;
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return 0;
    };
    let stuck = value
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .map_or(0, |entries| {
            entries
                .iter()
                .filter(|entry| entry.get("stuck") == Some(&serde_json::Value::Bool(true)))
                .count()
        });
    u32::try_from(stuck).unwrap_or(u32::MAX)
}

pub fn repair_items(content: &ScanContent, stuck_journal: u32) -> Vec<RepairItem> {
    use RepairAction as A;
    let mut items = Vec::new();
    for entry in &content.leftovers {
        let mut actions = vec![A::Trash];
        if stuck_journal == 0 {
            actions.push(A::DeletePermanently);
        }
        actions.push(A::Reveal);
        items.push(entry_item(
            RepairKind::Leftover,
            &entry.mod_ref,
            &entry.rel_path,
            actions,
        ));
    }
    for entry in &content.unmanaged {
        if entry.kind == UnmanagedKind::TempFolder {
            let actions = vec![A::Reveal];
            items.push(entry_item(
                RepairKind::TempFolderLive,
                &entry.mod_ref,
                &entry.rel_path,
                actions,
            ));
        }
    }
    let rows_with = |test: fn(&ModRow) -> bool| content.rows.iter().filter(move |row| test(row));
    for row in rows_with(|row| row.ambiguous) {
        items.push(row_item(
            RepairKind::AmbiguousKey,
            row,
            vec![A::Rename, A::Trash, A::Reveal],
            None,
        ));
    }
    let named = [
        (
            RepairKind::InvalidName,
            has_invalid_name as fn(&ModRow) -> bool,
        ),
        (RepairKind::NonUnicodeName, |row: &ModRow| {
            row.issues.contains(&ModIssue::NonUnicodeName)
        }),
    ];
    for (kind, test) in named {
        for row in rows_with(test) {
            let suggested = suggest_name(&row.name, &taken_in_root(content, row));
            items.push(row_item(
                kind,
                row,
                vec![A::Rename, A::Trash],
                Some(suggested),
            ));
        }
    }
    let logic = [
        (
            RepairKind::LogicDisabledButLoaded,
            ModIssue::LogicDisabledButLoaded,
            A::FinishDisable,
        ),
        (
            RepairKind::LogicEnabledNotLoaded,
            ModIssue::LogicEnabledNotLoaded,
            A::FinishEnable,
        ),
    ];
    for (kind, issue, action) in logic {
        for row in content
            .rows
            .iter()
            .filter(|row| row.issues.contains(&issue))
        {
            items.push(row_item(kind, row, vec![action, A::Reveal], None));
        }
    }
    if stuck_journal > 0 {
        items.push(RepairItem {
            id: "stuck".to_owned(),
            kind: RepairKind::StuckJournal,
            target: None,
            label: String::new(),
            actions: vec![A::OpenLogs],
            suggested_name: None,
        });
    }
    items
}

fn has_invalid_name(row: &ModRow) -> bool {
    row.issues
        .iter()
        .any(|issue| matches!(issue, ModIssue::InvalidName { .. }))
}

fn entry_item(
    kind: RepairKind,
    mod_ref: &super::ModRef,
    rel_path: &str,
    actions: Vec<RepairAction>,
) -> RepairItem {
    RepairItem {
        id: format!("{kind:?}|{mod_ref}"),
        kind,
        target: Some(ModTarget {
            mod_ref: mod_ref.clone(),
            key: rel_path.to_owned(),
        }),
        label: rel_path.to_owned(),
        actions,
        suggested_name: None,
    }
}

fn row_item(
    kind: RepairKind,
    row: &ModRow,
    actions: Vec<RepairAction>,
    suggested_name: Option<String>,
) -> RepairItem {
    RepairItem {
        id: format!("{kind:?}|{}", row.mod_ref),
        kind,
        target: Some(ModTarget {
            mod_ref: row.mod_ref.clone(),
            key: row.key.clone(),
        }),
        label: row.name.clone(),
        actions,
        suggested_name,
    }
}

/// Folded names in the row's root folder: folder rows, and the entries listed by rel path.
fn taken_in_root(content: &ScanContent, row: &ModRow) -> HashSet<String> {
    let prefix = format!("{}/", row.root);
    let rows = content
        .rows
        .iter()
        .filter(|other| other.root == row.root && other.kind != ModKind::Loose)
        .map(|other| fold_key(&other.folder_name));
    let rel_paths = content
        .archives
        .iter()
        .map(|entry| entry.rel_path.as_str())
        .chain(
            content
                .unmanaged
                .iter()
                .map(|entry| entry.rel_path.as_str()),
        )
        .chain(
            content
                .leftovers
                .iter()
                .map(|entry| entry.rel_path.as_str()),
        )
        .filter_map(|rel| rel.strip_prefix(&prefix))
        .filter(|rest| !rest.contains('/'))
        .map(fold_key);
    rows.chain(rel_paths).collect()
}

/// A name that passes `validate_name` and whose fold is not in `taken_folds` (spec §5.6).
pub fn suggest_name(name: &str, taken_folds: &HashSet<String>) -> String {
    let base = sanitize(name);
    for attempt in 1_u32.. {
        let suffix = if attempt == 1 {
            String::new()
        } else {
            format!(" ({attempt})")
        };
        let candidate = compose(&base, &suffix);
        if !taken_folds.contains(&fold_key(&candidate)) {
            return candidate;
        }
    }
    FALLBACK_NAME.to_owned()
}

fn sanitize(name: &str) -> String {
    let mut text: String = name
        .chars()
        .map(|c| {
            if FORBIDDEN_CHARS.contains(&c) || c.is_control() || is_bidi_control(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    if has_disabled_prefix_ascii(OsStr::new(&text)) {
        text = text
            .get(DISABLED_PREFIX_LEN..)
            .unwrap_or_default()
            .to_owned();
    }
    text
}

/// `base` cut so that `base + suffix` fits, cleaned up until it validates.
fn compose(base: &str, suffix: &str) -> String {
    let budget = NAME_MAX_BYTES.saturating_sub(suffix.len());
    let mut stem = cut_at_char(base, budget)
        .trim_end_matches(['.', ' '])
        .to_owned();
    if is_reserved_device_name(&stem) {
        let dot = stem.find('.').unwrap_or(stem.len());
        stem.insert(dot, '_');
    }
    if is_install_temp_name(OsStr::new(&stem)) {
        stem.push('_');
    }
    if stem.is_empty() {
        stem = FALLBACK_NAME.to_owned();
    }
    let candidate = format!("{stem}{suffix}");
    if validate_name(&candidate).is_ok() {
        candidate
    } else {
        let short = cut_at_char(&stem, budget.saturating_sub(FALLBACK_NAME.len() + 1));
        let retry = format!("{FALLBACK_NAME}_{short}{suffix}");
        if validate_name(&retry).is_ok() {
            retry
        } else {
            format!("{FALLBACK_NAME}{suffix}")
        }
    }
}

fn cut_at_char(text: &str, max_bytes: usize) -> &str {
    if text.len() <= max_bytes {
        return text;
    }
    let mut end = max_bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}
