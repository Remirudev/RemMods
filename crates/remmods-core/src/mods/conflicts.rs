//! Cross-row passes: ambiguous keys, case collisions and pak conflicts (spec §4.6).

use std::collections::{BTreeMap, BTreeSet};

use super::issues::ModIssue;
use super::model::{ConflictLink, ModKind, ModState, RootKind};
use super::rows::RowDraft;
use crate::safe_fs::names::{fold_key, order_key};

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
