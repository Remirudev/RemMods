//! Cross-row passes: ambiguous keys, case collisions and pak conflicts (spec §4.6), plus the
//! original app's view of conflicts for the P3 interop diff.

use std::collections::{BTreeMap, BTreeSet};

use super::issues::ModIssue;
use super::model::{ConflictLink, ModKind, ModRow, ModState, RootKind, ScanContent};
use super::rows::RowDraft;
use crate::safe_fs::names::{fold_key, order_key};

/// The original's IsEnabled. `None` = the original has no such row (Loose) or would fail (Unknown).
pub fn original_enabled(row: &ModRow) -> Option<bool> {
    match (row.kind, row.state) {
        (ModKind::Loose, _) | (_, ModState::Unknown) => None,
        (ModKind::Logic, _) => Some(!row.logic_prefixed),
        (_, ModState::Enabled | ModState::Partial) => Some(true),
        (_, ModState::Disabled | ModState::NoPakFiles) => Some(false),
    }
}

/// v1.1.1 ConflictDetector: ASCII-lowercased names of enabled rows sharing a live `.pak` basename
/// (compared ASCII case-insensitively, deduplicated per row).
pub fn original_conflict_names(content: &ScanContent) -> BTreeSet<String> {
    let mut owners: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for row in &content.rows {
        if original_enabled(row) != Some(true) || row.payload.live_pak == 0 {
            continue;
        }
        let name = row.name.to_ascii_lowercase();
        for pak in live_pak_names(row) {
            owners
                .entry(pak.to_ascii_lowercase())
                .or_default()
                .insert(name.clone());
        }
    }
    owners
        .into_values()
        .filter(|names| names.len() > 1)
        .flatten()
        .collect()
}

/// The row's own spelling of every live `.pak` it shares with another row. RemMods' fold key
/// unifies everything ASCII case-insensitivity does, so every name the original would match is
/// listed here.
fn live_pak_names(row: &ModRow) -> BTreeSet<String> {
    row.conflicts_with
        .iter()
        .flat_map(|link| link.paks.iter().cloned())
        .collect()
}

/// Rows with byte-identical keys.
pub(crate) fn mark_ambiguous(drafts: &mut [RowDraft]) {
    let mut by_key: BTreeMap<String, usize> = BTreeMap::new();
    for draft in drafts.iter() {
        *by_key.entry(draft.row.key.clone()).or_default() += 1;
    }
    for draft in drafts.iter_mut() {
        if by_key.get(&draft.row.key).is_some_and(|count| *count > 1) {
            draft.row.ambiguous = true;
            draft.row.issues.push(ModIssue::AmbiguousKey);
        }
    }
}

/// Rows of one kind (loose rows: of one key root) whose names fold alike but differ.
pub(crate) fn mark_case_collisions(drafts: &mut [RowDraft]) {
    type Group = (ModKind, Option<RootKind>, String);
    let mut groups: BTreeMap<Group, BTreeSet<(String, Vec<u8>)>> = BTreeMap::new();
    let group_of = |draft: &RowDraft| -> Group {
        (draft.row.kind, draft.loose_root, fold_key(&draft.row.name))
    };
    for draft in drafts.iter() {
        groups
            .entry(group_of(draft))
            .or_default()
            .insert(order_key(&draft.row.name));
    }
    for draft in drafts.iter_mut() {
        let Some(names) = groups.get(&group_of(draft)) else {
            continue;
        };
        let others: Vec<String> = names
            .iter()
            .filter(|(_, bytes)| bytes.as_slice() != draft.row.name.as_bytes())
            .filter_map(|(_, bytes)| String::from_utf8(bytes.clone()).ok())
            .collect();
        if !others.is_empty() {
            draft.row.issues.push(ModIssue::CaseCollision { others });
        }
    }
}

/// Two rows conflict when their live `.pak` basenames share a fold key (spec §4.6).
pub(crate) fn link_conflicts(drafts: &mut [RowDraft]) {
    let mut owners: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, draft) in drafts.iter().enumerate() {
        if draft.row.state == ModState::Unknown || draft.row.payload.live_pak == 0 {
            continue;
        }
        for fold in draft.live_paks.keys() {
            owners.entry(fold.as_str()).or_default().push(index);
        }
    }
    let mut shared: BTreeMap<(usize, usize), Vec<String>> = BTreeMap::new();
    for (fold, rows) in &owners {
        for &this in rows {
            for &other in rows.iter().filter(|&&other| other != this) {
                shared
                    .entry((this, other))
                    .or_default()
                    .push((*fold).to_owned());
            }
        }
    }
    let mut links: Vec<Vec<(SortKey, ConflictLink)>> = vec![Vec::new(); drafts.len()];
    for ((this, other), folds) in shared {
        let (this_draft, other_draft) = (&drafts[this], &drafts[other]);
        let mut paks: Vec<String> = folds
            .iter()
            .filter_map(|fold| this_draft.live_paks.get(fold).cloned())
            .collect();
        paks.sort_by_cached_key(|name| order_key(name));
        let link = ConflictLink {
            other: other_draft.row.mod_ref.clone(),
            key: other_draft.row.key.clone(),
            name: other_draft.row.name.clone(),
            paks,
        };
        let sort_key = (
            order_key(&other_draft.logical_key),
            other_draft.row.key.clone().into_bytes(),
            other_draft.row.mod_ref.as_str().to_owned(),
        );
        links[this].push((sort_key, link));
    }
    for (draft, mut found) in drafts.iter_mut().zip(links) {
        found.sort_by(|a, b| a.0.cmp(&b.0));
        draft.row.conflicts_with = found.into_iter().map(|(_, link)| link).collect();
    }
}

type SortKey = ((String, Vec<u8>), Vec<u8>, String);
