//! Wrappers, candidates and variant groups (spec §5.3 step 7). Pure and independent of the order
//! of its input.

use std::collections::{BTreeMap, BTreeSet};

use super::detect_root::is_payload_name;
use crate::safe_fs::names::{fold_key, order_key};

type OrderKey = (String, Vec<u8>);
/// An input index with the path below the candidate folder.
type Member = (usize, Vec<String>);

/// One file below the detected root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanFile {
    pub path: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedCandidate {
    /// Index in `order_key` order of `inner`.
    pub id: u32,
    pub name: String,
    /// Components (below the root) before the candidate's own files.
    pub inner: Vec<String>,
    /// Input indices with the path below the candidate folder, in `order_key` order of the path.
    pub files: Vec<(usize, Vec<String>)>,
    pub group: Option<u32>,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedGroup {
    pub id: u32,
    pub candidates: Vec<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CandidatePlan {
    pub candidates: Vec<PlannedCandidate>,
    pub groups: Vec<PlannedGroup>,
    /// Top-level items outside the chosen candidate folders (not installed).
    pub extra_files: Vec<usize>,
}

pub fn plan_candidates(files: &[PlanFile], source_stem: &str) -> CandidatePlan {
    let (depth, wrapper) = wrapper_depth(files);
    let top_level_payload = files
        .iter()
        .any(|file| file.path.len() == depth + 1 && is_payload_name(last(&file.path)));
    if files.is_empty() {
        return CandidatePlan::default();
    }
    if top_level_payload {
        let inner = files[0].path[..depth].to_vec();
        let name = wrapper.unwrap_or_else(|| source_stem.to_owned());
        let members = (0..files.len())
            .map(|index| (index, files[index].path[depth..].to_vec()))
            .collect();
        return CandidatePlan {
            candidates: vec![candidate(0, name, inner, members)],
            groups: Vec::new(),
            extra_files: Vec::new(),
        };
    }
    let folders: BTreeSet<OrderKey> = files
        .iter()
        .filter(|file| file.path.len() > depth + 1 && is_payload_name(last(&file.path)))
        .map(|file| order_key(&file.path[depth]))
        .collect();
    let mut by_folder: BTreeMap<OrderKey, Vec<Member>> = BTreeMap::new();
    let mut extra_files = Vec::new();
    for (index, file) in files.iter().enumerate() {
        let key = file.path.get(depth).map(|part| order_key(part));
        match key.filter(|key| folders.contains(key) && file.path.len() > depth + 1) {
            Some(key) => by_folder
                .entry(key)
                .or_default()
                .push((index, file.path[depth + 1..].to_vec())),
            None => extra_files.push(index),
        }
    }
    let mut candidates: Vec<PlannedCandidate> = by_folder
        .into_values()
        .enumerate()
        .map(|(id, members)| {
            let first = &files[members[0].0].path;
            let name = first[depth].clone();
            let inner = first[..=depth].to_vec();
            candidate(id, name, inner, members)
        })
        .collect();
    let groups = variant_groups(&mut candidates);
    extra_files.sort_by_key(|&index| path_order(&files[index].path));
    CandidatePlan {
        candidates,
        groups,
        extra_files,
    }
}

/// Strips the common first component while every file has at least two more and all agree.
fn wrapper_depth(files: &[PlanFile]) -> (usize, Option<String>) {
    let mut depth = 0;
    let mut wrapper = None;
    loop {
        let Some(first) = files.first().and_then(|file| file.path.get(depth)) else {
            return (depth, wrapper);
        };
        let shared = files
            .iter()
            .all(|file| file.path.len() >= depth + 2 && file.path[depth] == *first);
        if !shared {
            return (depth, wrapper);
        }
        wrapper = Some(first.clone());
        depth += 1;
    }
}

fn candidate(
    id: usize,
    name: String,
    inner: Vec<String>,
    mut files: Vec<(usize, Vec<String>)>,
) -> PlannedCandidate {
    files.sort_by_key(|(_, path)| path_order(path));
    PlannedCandidate {
        id: u32::try_from(id).unwrap_or(u32::MAX),
        name,
        inner,
        files,
        group: None,
        selected: true,
    }
}

/// Candidates whose payload basenames overlap (fold key) are alternatives: one group each, only
/// the first (by id) selected.
fn variant_groups(candidates: &mut [PlannedCandidate]) -> Vec<PlannedGroup> {
    let names: Vec<BTreeSet<String>> = candidates
        .iter()
        .map(|candidate| {
            candidate
                .files
                .iter()
                .map(|(_, path)| last(path))
                .filter(|name| is_payload_name(name))
                .map(fold_key)
                .collect()
        })
        .collect();
    let mut parent: Vec<usize> = (0..candidates.len()).collect();
    for left in 0..candidates.len() {
        for right in left + 1..candidates.len() {
            if !names[left].is_disjoint(&names[right]) {
                let (a, b) = (find(&mut parent, left), find(&mut parent, right));
                parent[a.max(b)] = a.min(b);
            }
        }
    }
    let mut members: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for index in 0..candidates.len() {
        let root = find(&mut parent, index);
        members.entry(root).or_default().push(index);
    }
    let mut groups = Vec::new();
    for list in members.into_values().filter(|list| list.len() > 1) {
        let id = u32::try_from(groups.len()).unwrap_or(u32::MAX);
        for (position, &index) in list.iter().enumerate() {
            candidates[index].group = Some(id);
            candidates[index].selected = position == 0;
        }
        groups.push(PlannedGroup {
            id,
            candidates: list.iter().map(|&index| candidates[index].id).collect(),
        });
    }
    groups
}

fn find(parent: &mut [usize], index: usize) -> usize {
    let mut current = index;
    while parent[current] != current {
        current = parent[current];
    }
    current
}

fn last(path: &[String]) -> &str {
    path.last().map_or("", String::as_str)
}

pub(crate) fn path_order(path: &[String]) -> Vec<(String, Vec<u8>)> {
    path.iter().map(|part| order_key(part)).collect()
}
