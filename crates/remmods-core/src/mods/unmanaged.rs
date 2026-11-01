//! Payload the game loads outside RemMods' rows: files at the Paks top level, non-root folders,
//! linked folders, temp folders; and loose copies of row files among them (spec §4.6).

use std::collections::{BTreeMap, BTreeSet};

use super::issues::ModIssue;
use super::model::{UnmanagedEntry, UnmanagedKind};
use super::rows::RowDraft;
use crate::safe_fs::names::order_key;

/// An entry plus the live `.pak/.ucas/.utoc` basenames it holds (fold → display).
#[derive(Clone, Debug)]
pub(crate) struct UnmanagedDraft {
    pub entry: UnmanagedEntry,
    pub names: BTreeMap<String, String>,
}

/// Every unmanaged entry but `LinkedFolder` whose names match a row's payload basename gets
/// `copy_of`; the rows get `LooseCopy` with their own spelling of the duplicated names.
pub(crate) fn mark_loose_copies(drafts: &mut [RowDraft], unmanaged: &mut [UnmanagedDraft]) {
    let mut owners: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, draft) in drafts.iter().enumerate() {
        for fold in draft.payload_names.keys() {
            owners.entry(fold.as_str()).or_default().push(index);
        }
    }
    let mut copied: Vec<BTreeSet<String>> = vec![BTreeSet::new(); drafts.len()];
    for item in unmanaged.iter_mut() {
        if item.entry.kind == UnmanagedKind::LinkedFolder {
            continue;
        }
        let mut copy_of = BTreeSet::new();
        for fold in item.names.keys() {
            for &row in owners.get(fold.as_str()).into_iter().flatten() {
                copy_of.insert(order_key(&drafts[row].row.name));
                copied[row].insert(fold.clone());
            }
        }
        item.entry.copy_of = copy_of
            .into_iter()
            .filter_map(|(_, bytes)| String::from_utf8(bytes).ok())
            .collect();
    }
    for (draft, folds) in drafts.iter_mut().zip(copied) {
        if folds.is_empty() {
            continue;
        }
        let mut files: Vec<String> = folds
            .iter()
            .filter_map(|fold| draft.payload_names.get(fold).cloned())
            .collect();
        files.sort_by_cached_key(|name| order_key(name));
        files.dedup();
        draft.row.issues.push(ModIssue::LooseCopy { files });
    }
}
