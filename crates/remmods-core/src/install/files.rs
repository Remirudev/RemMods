//! From normalized entries to the files an install may stage (spec §5.3 steps 3–5): kinds,
//! folder case unification, duplicates, case clashes, empty folders.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::SystemTime;

use super::issues::{Issues, count};
use super::model::{CaseClash, IssueCode, IssueSeverity};
use super::plan::path_order;
use crate::archive::{Entry, EntryKind, Listing, entry_is_unsafe};
use crate::safe_fs::names::fold_key;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileItem {
    pub(crate) index: u32,
    pub(crate) path: Vec<String>,
    pub(crate) display: String,
    pub(crate) size: u64,
    pub(crate) crc32: Option<u32>,
    pub(crate) mtime: Option<SystemTime>,
}

#[derive(Debug, Default)]
pub(crate) struct Files {
    pub(crate) items: Vec<FileItem>,
    /// Per clash id: the entry indices in the order of `CaseClash::paths`.
    pub(crate) clash_members: Vec<Vec<u32>>,
    pub(crate) clashes: Vec<CaseClash>,
    pub(crate) legacy: bool,
}

/// `None` when an entry is unsafe (the issue is recorded and the inspection stops).
pub(crate) fn collect_files(
    listing: &Listing,
    entries: &[Entry],
    issues: &mut Issues,
) -> Option<Files> {
    let unsafe_entries: Vec<&Entry> = entries.iter().filter(|e| entry_is_unsafe(e)).collect();
    if let Some(first) = unsafe_entries.first() {
        let n = count(unsafe_entries.len());
        issues.add_count(
            IssueSeverity::Block,
            IssueCode::UnsafePath,
            &first.display,
            n,
        );
        return None;
    }
    let anti: HashSet<u32> = listing
        .entries
        .iter()
        .filter(|raw| raw.anti)
        .map(|raw| raw.index)
        .collect();
    if listing.duplicate_raw {
        issues.block(IssueCode::DuplicateRaw, "");
    }
    let mut files = Files::default();
    let mut dirs: Vec<&Entry> = Vec::new();
    for entry in entries {
        if entry.legacy {
            files.legacy = true;
            issues.info(IssueCode::LegacyNames, &entry.display);
        }
        if entry.undecodable {
            issues.block(IssueCode::Undecodable, &entry.display);
        }
        match entry.kind {
            EntryKind::Link => issues.info(IssueCode::Link, &entry.display),
            EntryKind::Special if anti.contains(&entry.index) => {
                issues.info(IssueCode::AntiItem, &entry.display);
            }
            EntryKind::Special => issues.info(IssueCode::Special, &entry.display),
            EntryKind::Dir if entry.problem.is_none() => dirs.push(entry),
            EntryKind::Dir => {}
            EntryKind::File => files.items.push(FileItem {
                index: entry.index,
                path: entry.path.clone(),
                display: entry.display.clone(),
                size: entry.size,
                crc32: entry.crc32,
                mtime: entry.mtime,
            }),
        }
    }
    unify_folder_case(&mut files.items);
    drop_duplicates(&mut files.items, issues);
    count_empty_dirs(&dirs, &files.items, issues);
    find_clashes(&mut files);
    if let Some(first) = files.clashes.first() {
        let n = count(files.clashes.len());
        let path = first.paths.first().cloned().unwrap_or_default();
        issues.add_count(IssueSeverity::Warn, IssueCode::CaseClash, &path, n);
    }
    Some(files)
}

/// Folder components that differ only in case take the first spelling (`order_key`).
fn unify_folder_case(items: &mut [FileItem]) {
    let mut chosen: HashMap<String, String> = HashMap::new();
    for item in items.iter() {
        let mut key = String::new();
        for part in item.path.iter().take(item.path.len().saturating_sub(1)) {
            key.push('/');
            key.push_str(&fold_key(part));
            let spelling = chosen.entry(key.clone()).or_insert_with(|| part.clone());
            if path_order(std::slice::from_ref(part)) < path_order(std::slice::from_ref(spelling)) {
                *spelling = part.clone();
            }
        }
    }
    for item in items.iter_mut() {
        let mut key = String::new();
        let folders = item.path.len().saturating_sub(1);
        for part in item.path.iter_mut().take(folders) {
            key.push('/');
            key.push_str(&fold_key(part));
            if let Some(spelling) = chosen.get(&key) {
                part.clone_from(spelling);
            }
        }
    }
}

/// Exact path duplicates: same size and both CRCs known and equal → keep the first, info;
/// otherwise block (unknown CRC counts as differing).
fn drop_duplicates(items: &mut Vec<FileItem>, issues: &mut Issues) {
    let mut by_path: BTreeMap<Vec<String>, Vec<usize>> = BTreeMap::new();
    for (position, item) in items.iter().enumerate() {
        by_path.entry(item.path.clone()).or_default().push(position);
    }
    let mut drop = HashSet::new();
    for positions in by_path.values().filter(|positions| positions.len() > 1) {
        let first = &items[positions[0]];
        let identical = positions.iter().all(|&other| {
            let other = &items[other];
            other.size == first.size && other.crc32.is_some() && other.crc32 == first.crc32
        });
        if identical {
            issues.info(IssueCode::DuplicateIdentical, &first.display);
            drop.extend(positions[1..].iter().copied());
        } else {
            issues.block(IssueCode::DuplicateDiffers, &first.display);
        }
    }
    let mut position = 0;
    items.retain(|_| {
        let keep = !drop.contains(&position);
        position += 1;
        keep
    });
}

fn count_empty_dirs(dirs: &[&Entry], items: &[FileItem], issues: &mut Issues) {
    let prefixes: HashSet<String> = items
        .iter()
        .flat_map(|item| (1..item.path.len()).map(|depth| folded_path(&item.path[..depth])))
        .collect();
    let empty: Vec<&&Entry> = dirs
        .iter()
        .filter(|dir| !prefixes.contains(&folded_path(&dir.path)))
        .collect();
    if let Some(first) = empty.first() {
        let n = count(empty.len());
        issues.add_count(IssueSeverity::Info, IssueCode::EmptyDirs, &first.display, n);
    }
}

/// Files whose paths are fold-equal but byte-different.
fn find_clashes(files: &mut Files) {
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (position, item) in files.items.iter().enumerate() {
        groups
            .entry(folded_path(&item.path))
            .or_default()
            .push(position);
    }
    let mut found: Vec<Vec<usize>> = groups
        .into_values()
        .filter(|positions| positions.len() > 1)
        .map(|mut positions| {
            positions.sort_by_key(|&position| path_order(&files.items[position].path));
            positions
        })
        .collect();
    found.sort_by_key(|positions| path_order(&files.items[positions[0]].path));
    for (id, positions) in found.into_iter().enumerate() {
        files.clashes.push(CaseClash {
            id: count(id),
            paths: positions
                .iter()
                .map(|&position| files.items[position].path.join("/"))
                .collect(),
        });
        files.clash_members.push(
            positions
                .iter()
                .map(|&position| files.items[position].index)
                .collect(),
        );
    }
}

pub(crate) fn folded_path(path: &[String]) -> String {
    path.iter()
        .map(|part| fold_key(part))
        .collect::<Vec<_>>()
        .join("/")
}
