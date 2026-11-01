//! Building one row: folder rows of the three roots and loose groups (spec §4.5 steps 4–5, §4.6).

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::issues::{ModIssue, NameProblem};
use super::listing::Entry;
use super::metadata::{MODINFO_MAX_BYTES, parse_modinfo};
use super::model::{
    ModInfo, ModInfoOrigin, ModKind, ModRow, ModState, PayloadCounts, PayloadKind, RootKind,
    ToggleBlock,
};
use super::modref::{ModRef, RefKind};
use super::path_length::path_too_long;
use super::strategy::{LooseGroup, StateInput, logic_state, pak_state};
use super::walk::{TopFile, Walk, insert_smallest};
use crate::content::{ContentSummary, UtocRef};
use crate::safe_fs::names::{
    DISABLED_PREFIX, LOGIC_NAME_MAX_BYTES, NAME_MAX_BYTES, PAYLOAD_NAME_MAX_BYTES, display_escaped,
    fold_key, has_disabled_prefix_ascii, portable_name_issue, utf16_len, windows_name_issue,
};

const MODINFO_NAME: &str = "modinfo.json";
const OFF_SUFFIX_UNITS: usize = 2;

/// A root folder as the row builders need it.
#[derive(Clone, Debug)]
pub(crate) struct RootRef {
    pub kind: RootKind,
    /// Key text of the on-disk name (`Mods`, `mods`, `~mods`, …).
    pub key: String,
    pub path: PathBuf,
    pub is_link: bool,
    pub case_variant: bool,
    /// UTF-16 length of the lossy on-disk name (W1).
    pub units: usize,
}

/// A row plus what the cross-row passes and the final order need.
#[derive(Clone, Debug)]
pub(crate) struct RowDraft {
    pub row: ModRow,
    pub logical_key: String,
    pub root_key: String,
    pub folder_key: String,
    /// Loose rows: the root kind the key names (`Loose:Mods/…`).
    pub loose_root: Option<RootKind>,
    /// Live `.pak` basenames (fold → display); empty for an `Unknown` row.
    pub live_paks: BTreeMap<String, String>,
    /// Every payload basename, `-x` stripped (fold → display).
    pub payload_names: BTreeMap<String, String>,
    /// Every `.utoc` / `.utoc-x` of the row, read by `content::attach`.
    pub utocs: Vec<UtocRef>,
}

pub(crate) fn folder_row(root: &RootRef, entry: &Entry, walk: Walk, mod_ref: ModRef) -> RowDraft {
    let prefixed = root.kind == RootKind::Logic && has_disabled_prefix_ascii(&entry.name);
    let logical_key = if prefixed {
        entry.key.get(DISABLED_PREFIX.len()..).unwrap_or_default()
    } else {
        entry.key.as_str()
    }
    .to_owned();
    let state_input = StateInput {
        counts: walk.counts,
        unreadable: walk.root_unreadable,
        contains_link: !walk.links.is_empty(),
    };
    let mut issues = Vec::new();
    let state = if root.kind == RootKind::Logic {
        let (state, issue) = logic_state(state_input, prefixed);
        issues.extend(issue);
        state
    } else {
        pak_state(state_input)
    };
    issues.extend(folder_name_issue(root.kind, entry, &logical_key, prefixed));
    if root.kind != RootKind::Logic && has_disabled_prefix_ascii(&entry.name) {
        issues.push(ModIssue::DisabledPrefixName);
    }
    let info = if walk.root_unreadable {
        ModInfo::defaults(ModInfoOrigin::Missing)
    } else {
        read_modinfo(&walk.top_files, &mut issues)
    };
    issues.extend(path_too_long(walk.longest_file, walk.longest_dir));
    common_issues(root, &mut issues);
    if entry.kind.is_link() {
        issues.push(ModIssue::LinkedMod);
    }
    if !walk.links.is_empty() {
        issues.push(ModIssue::ContainsLink);
    }
    file_name_issues(&walk, &mut issues);
    if let Some(problem) = walk.problem {
        issues.push(ModIssue::Unreadable { problem });
    }
    let row = ModRow {
        mod_ref,
        key: format!("{}:{logical_key}", root.kind.key_prefix()),
        kind: mod_kind(root.kind),
        name: display_escaped(&logical_key),
        folder_name: display_escaped(&entry.key),
        root: display_escaped(&root.key),
        path: entry.path.clone(),
        state,
        logic_prefixed: prefixed,
        payload: walk.counts,
        size: walk.problem.is_none().then_some(walk.size),
        info,
        conflicts_with: Vec::new(),
        content: ContentSummary::default(),
        content_conflicts: Vec::new(),
        ambiguous: false,
        issues,
        toggle_block: None,
    };
    RowDraft {
        row,
        logical_key,
        root_key: root.key.clone(),
        folder_key: entry.key.clone(),
        loose_root: None,
        live_paks: walk.live_paks,
        payload_names: walk.payload_names,
        utocs: walk.utocs,
    }
}

pub(crate) fn loose_row(
    root: &RootRef,
    group: &LooseGroup,
    paks_id: &str,
    game_view_len: usize,
) -> RowDraft {
    let stem_key = group.stem_key().to_owned();
    let mut counts = PayloadCounts::default();
    let mut size = Some(0_u64);
    let mut problem = None;
    let mut live_paks = BTreeMap::new();
    let mut payload_names = BTreeMap::new();
    let mut longest_file = 0;
    let mut utocs = Vec::new();
    for file in &group.files {
        let payload = file.payload;
        let counter = match (payload.is_pak(), payload.live) {
            (true, true) => &mut counts.live_pak,
            (true, false) => &mut counts.off_pak,
            (false, true) => &mut counts.live_companion,
            (false, false) => &mut counts.off_companion,
        };
        *counter = counter.saturating_add(1);
        if payload.kind == PayloadKind::Utoc {
            utocs.push(UtocRef {
                path: file.path.clone(),
                rel_path: display_escaped(&file.key),
                live: payload.live,
                size: *file.size.as_ref().unwrap_or(&0),
                modified: file.modified,
            });
        }
        match file.size {
            Ok(bytes) => size = size.map(|sum| sum.saturating_add(bytes)),
            Err(found) => {
                size = None;
                problem.get_or_insert(found);
            }
        }
        let display = display_escaped(&file.key);
        let stripped = super::payload::strip_off_suffix(&display).to_owned();
        insert_smallest(&mut payload_names, fold_key(&stripped), stripped);
        if payload.live && payload.is_pak() {
            insert_smallest(&mut live_paks, fold_key(&display), display);
        }
        let suffix = if payload.live { OFF_SUFFIX_UNITS } else { 0 };
        let name_units = utf16_len(&file.name.to_string_lossy());
        longest_file = longest_file.max(game_view_len + 1 + root.units + 1 + name_units + suffix);
    }
    let mut issues = Vec::new();
    issues.extend(loose_name_issues(group));
    issues.extend(path_too_long(longest_file, 0));
    common_issues(root, &mut issues);
    if group.case_variants() {
        issues.push(ModIssue::LooseCaseVariants);
    }
    if let Some(problem) = problem {
        issues.push(ModIssue::Unreadable { problem });
    }
    let row = ModRow {
        mod_ref: ModRef::build(RefKind::Loose, None, paks_id, &[&root.key, &stem_key]),
        key: format!("Loose:{}/{stem_key}", root.kind.folder()),
        kind: ModKind::Loose,
        name: display_escaped(&stem_key),
        folder_name: display_escaped(&stem_key),
        root: display_escaped(&root.key),
        path: root.path.clone(),
        state: pak_state(StateInput {
            counts,
            ..StateInput::default()
        }),
        logic_prefixed: false,
        payload: counts,
        size,
        info: ModInfo::defaults(ModInfoOrigin::Missing),
        conflicts_with: Vec::new(),
        content: ContentSummary::default(),
        content_conflicts: Vec::new(),
        ambiguous: false,
        issues,
        toggle_block: None,
    };
    RowDraft {
        row,
        logical_key: stem_key.clone(),
        root_key: root.key.clone(),
        folder_key: stem_key,
        loose_root: Some(root.kind),
        live_paks,
        payload_names,
        utocs,
    }
}

fn mod_kind(kind: RootKind) -> ModKind {
    match kind {
        RootKind::Standard => ModKind::Standard,
        RootKind::Legacy => ModKind::Legacy,
        RootKind::Logic => ModKind::Logic,
    }
}

fn common_issues(root: &RootRef, issues: &mut Vec<ModIssue>) {
    if root.case_variant {
        issues.push(ModIssue::CaseVariantRoot);
    }
    if root.is_link {
        issues.push(ModIssue::LinkedRoot);
    }
}

/// At most one of `NonUnicodeName`, `InvalidName`, `NameNotPortable`; `EmptyName` for a Logic
/// folder named exactly `.disabled_`.
fn folder_name_issue(
    kind: RootKind,
    entry: &Entry,
    logical_key: &str,
    prefixed: bool,
) -> Option<ModIssue> {
    let Some(name) = entry.name.to_str() else {
        return Some(ModIssue::NonUnicodeName);
    };
    if let Some(issue) = windows_name_issue(name) {
        return Some(ModIssue::InvalidName {
            problem: issue.into(),
        });
    }
    if prefixed && logical_key.is_empty() {
        return Some(ModIssue::EmptyName);
    }
    if kind == RootKind::Logic
        && let Some(issue) = windows_name_issue(logical_key)
    {
        return Some(ModIssue::InvalidName {
            problem: issue.into(),
        });
    }
    let budget = if kind == RootKind::Logic && !prefixed {
        LOGIC_NAME_MAX_BYTES
    } else {
        NAME_MAX_BYTES
    };
    portable_name_issue(name, budget).map(|issue| ModIssue::NameNotPortable {
        problem: issue.into(),
    })
}

fn loose_name_issues(group: &LooseGroup) -> Vec<ModIssue> {
    let mut issues = Vec::new();
    if group.files.iter().any(|file| file.name.to_str().is_none()) {
        issues.push(ModIssue::NonUnicodeName);
    }
    let names: Vec<(&str, bool)> = group
        .files
        .iter()
        .filter_map(|file| Some((file.name.to_str()?, file.payload.live)))
        .collect();
    let invalid = names
        .iter()
        .filter(|(name, _)| windows_name_issue(name).is_some())
        .count();
    let not_portable: Option<NameProblem> = names
        .iter()
        .filter(|(name, live)| *live && windows_name_issue(name).is_none())
        .find_map(|(name, _)| portable_name_issue(name, PAYLOAD_NAME_MAX_BYTES))
        .map(NameProblem::from);
    if let Some(problem) = not_portable {
        issues.push(ModIssue::NameNotPortable { problem });
    }
    if invalid > 0 {
        issues.push(ModIssue::InvalidFileNames {
            count: u32::try_from(invalid).unwrap_or(u32::MAX),
        });
    }
    issues
}

fn file_name_issues(walk: &Walk, issues: &mut Vec<ModIssue>) {
    if walk.non_unicode_files > 0 {
        issues.push(ModIssue::NonUnicodeFiles {
            count: walk.non_unicode_files,
        });
    }
    if walk.invalid_file_names > 0 {
        issues.push(ModIssue::InvalidFileNames {
            count: walk.invalid_file_names,
        });
    }
    if walk.non_portable_file_names > 0 {
        issues.push(ModIssue::NonPortableFileNames {
            count: walk.non_portable_file_names,
        });
    }
}

/// The exact `modinfo.json`, else its single case variant; several variants → none used.
fn read_modinfo(top_files: &[TopFile], issues: &mut Vec<ModIssue>) -> ModInfo {
    let exact = top_files.iter().find(|file| file.name == MODINFO_NAME);
    let candidate = exact.or_else(|| {
        let mut variants = top_files.iter().filter(|file| {
            file.name
                .as_encoded_bytes()
                .eq_ignore_ascii_case(MODINFO_NAME.as_bytes())
        });
        let first = variants.next();
        if variants.next().is_some() {
            issues.push(ModIssue::ModinfoCaseVariants);
            return None;
        }
        first
    });
    let Some(file) = candidate else {
        return ModInfo::defaults(ModInfoOrigin::Missing);
    };
    let too_large = file.size.is_none_or(|size| size > MODINFO_MAX_BYTES as u64);
    let info = if too_large {
        ModInfo::defaults(ModInfoOrigin::Invalid)
    } else {
        match std::fs::read(&file.path) {
            Ok(bytes) => parse_modinfo(&bytes),
            Err(error) => {
                log::debug!("modinfo.json unreadable {}: {error}", file.path.display());
                ModInfo::defaults(ModInfoOrigin::Invalid)
            }
        }
    };
    if info.origin == ModInfoOrigin::Invalid {
        issues.push(ModIssue::ModinfoInvalid);
    }
    info
}

/// Runs after the cross-row passes, which set `ambiguous` (spec §5.1).
pub(crate) fn toggle_block(row: &ModRow) -> Option<ToggleBlock> {
    if row.ambiguous {
        return Some(ToggleBlock::Ambiguous);
    }
    if row.issues.contains(&ModIssue::LinkedMod) {
        return Some(ToggleBlock::Linked);
    }
    match row.state {
        ModState::Unknown => Some(ToggleBlock::UnknownState),
        ModState::NoPakFiles => Some(ToggleBlock::NoPakFiles),
        _ if row.kind == ModKind::Logic && row.key == "Logic:" => {
            Some(ToggleBlock::InvalidLogicName)
        }
        _ => None,
    }
}
