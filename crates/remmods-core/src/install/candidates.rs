//! Candidates of an inspection (spec §5.3 steps 8–11): per-file rules, names, existing mods,
//! path lengths.

use std::collections::{HashMap, HashSet};

use super::detect_root::{is_live_payload_name, is_off_payload_name};
use super::existing::find_existing;
use super::files::{FileItem, folded_path};
use super::issues::{Issues, count};
use super::model::{Candidate, IssueCode, NameOverride};
use super::names_fix::{
    extension_of, file_name_issue, fix_folder_name, fix_name, folder_name_issue, stem_of,
};
use super::plan::{CandidatePlan, PlannedCandidate};
use super::service::{StoredCandidate, StoredFile};
use crate::mods::path_length::path_too_long;
use crate::mods::{RootKind, ScanContent};
use crate::safe_fs::names::{DISABLED_PREFIX, LOGIC_NAME_MAX_BYTES, fold_key, utf16_len};

const NESTED: [&str; 4] = ["zip", "7z", "rar", "tkmod"];

/// What the candidate step needs besides the plan.
pub(crate) struct CandidateInput<'a> {
    /// Files below the detected root (`plan` indexes them).
    pub(crate) items: &'a [FileItem],
    /// The same files' paths inside the source (before the root was stripped).
    pub(crate) source_paths: &'a [Vec<String>],
    pub(crate) root: RootKind,
    /// The root folder as spelled on disk (or the name it will be created with).
    pub(crate) root_name: &'a str,
    pub(crate) game_view_len: usize,
    pub(crate) names: &'a [NameOverride],
    pub(crate) content: &'a ScanContent,
}

pub(crate) fn build_candidates(
    input: &CandidateInput<'_>,
    plan: &CandidatePlan,
    issues: &mut Issues,
) -> (Vec<Candidate>, Vec<StoredCandidate>) {
    let mut candidates = Vec::new();
    let mut stored = Vec::new();
    for planned in &plan.candidates {
        let (candidate, files) = build_one(input, planned, issues);
        candidates.push(candidate);
        stored.push(files);
    }
    check_candidate_collisions(&candidates, issues);
    (candidates, stored)
}

fn build_one(
    input: &CandidateInput<'_>,
    planned: &PlannedCandidate,
    issues: &mut Issues,
) -> (Candidate, StoredCandidate) {
    let name = input
        .names
        .iter()
        .find(|name| name.id == planned.id)
        .map_or_else(|| planned.name.clone(), |name| name.name.clone());
    let mut kept: Vec<(&FileItem, &Vec<String>)> = Vec::new();
    for (index, rel) in &planned.files {
        let item = &input.items[*index];
        let file_name = rel.last().map_or("", String::as_str);
        if is_nested_archive(file_name) {
            issues.info(IssueCode::NestedArchive, &item.display);
            continue;
        }
        if is_off_payload_name(file_name) {
            issues.info(IssueCode::ShipsDisabled, &item.display);
        }
        kept.push((item, rel));
    }
    check_both_states(&kept, issues);
    let dests = fixed_dests(&kept, issues);
    let (name_issue, fixed_name) = candidate_name(&name, input.root, issues);
    let final_name = fixed_name.clone().unwrap_or_else(|| name.clone());
    let (existing, also_in) = find_existing(input.content, input.root, &final_name);
    if let Some(existing) = &existing {
        if existing.ambiguous {
            issues.block(IssueCode::Ambiguous, &existing.rel);
        } else {
            issues.warn(IssueCode::Replaces, &existing.rel);
        }
    }
    if !also_in.is_empty() {
        issues.info(IssueCode::AlsoInRoot, &final_name);
    }
    let files: Vec<StoredFile> = kept
        .iter()
        .zip(dests)
        .map(|((item, _), dest)| StoredFile {
            index: item.index,
            live_payload: dest.last().is_some_and(|name| is_live_payload_name(name)),
            dest,
            size: item.size,
            mtime: item.mtime,
        })
        .collect();
    let too_long = is_path_too_long(input, &final_name, &files);
    if too_long {
        issues.warn(IssueCode::PathTooLong, &final_name);
    }
    let inner = kept
        .first()
        .map(|(item, rel)| inner_path(input, item, rel))
        .unwrap_or_default();
    let candidate = Candidate {
        id: planned.id,
        name,
        inner,
        files: count(files.len()),
        payload: count(files.iter().filter(|file| file.live_payload).count()),
        shipped_off: count(
            files
                .iter()
                .filter(|file| {
                    file.dest
                        .last()
                        .is_some_and(|name| is_off_payload_name(name))
                })
                .count(),
        ),
        bytes: files.iter().map(|file| file.size).sum(),
        group: planned.group,
        selected: planned.selected,
        name_issue,
        fixed_name,
        existing,
        also_in,
        path_too_long: too_long,
    };
    let stored = StoredCandidate {
        id: planned.id,
        files,
    };
    (candidate, stored)
}

fn is_nested_archive(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(stem, extension)| {
        !stem.is_empty()
            && NESTED
                .iter()
                .any(|nested| extension.eq_ignore_ascii_case(nested))
    })
}

/// `X.pak` together with `X.pak-x` (fold key) in one candidate.
fn check_both_states(kept: &[(&FileItem, &Vec<String>)], issues: &mut Issues) {
    let off: HashSet<String> = kept
        .iter()
        .filter(|(_, rel)| rel.last().is_some_and(|name| is_off_payload_name(name)))
        .map(|(_, rel)| folded_path(rel))
        .collect();
    for (item, rel) in kept {
        let live = rel.last().is_some_and(|name| is_live_payload_name(name));
        if live && off.contains(&format!("{}-x", folded_path(rel))) {
            issues.block(IssueCode::ShipsBothStates, &item.display);
        }
    }
}

/// Every component through the name rules; failing ones get `fix_name`, payload companions
/// (same folder and stem) the same new stem. Fixed names that collide block.
fn fixed_dests(kept: &[(&FileItem, &Vec<String>)], issues: &mut Issues) -> Vec<Vec<String>> {
    let mut changed = vec![false; kept.len()];
    let mut dests: Vec<Vec<String>> = Vec::with_capacity(kept.len());
    for (position, (item, rel)) in kept.iter().enumerate() {
        let mut dest = Vec::with_capacity(rel.len());
        for (depth, part) in rel.iter().enumerate() {
            let is_file = depth + 1 == rel.len();
            let issue = if is_file {
                file_name_issue(part)
            } else {
                folder_name_issue(part)
            };
            if issue.is_none() {
                dest.push(part.clone());
                continue;
            }
            let fixed = fix_name(part);
            let still = if is_file {
                file_name_issue(&fixed)
            } else {
                folder_name_issue(&fixed)
            };
            if still.is_some() {
                issues.block(IssueCode::InvalidName, &item.display);
            }
            changed[position] = true;
            dest.push(fixed);
        }
        if changed[position] {
            issues.warn(IssueCode::InvalidName, &item.display);
        }
        dests.push(dest);
    }
    align_companions(kept, &changed, &mut dests);
    let mut seen: HashMap<String, (usize, bool)> = HashMap::new();
    for (position, dest) in dests.iter().enumerate() {
        let key = folded_path(dest);
        if let Some((other, other_changed)) = seen.get(&key) {
            if (changed[position] || *other_changed) && kept[*other].1 != kept[position].1 {
                issues.block(IssueCode::InvalidName, &kept[position].0.display);
            }
            continue;
        }
        seen.insert(key, (position, changed[position]));
    }
    dests
}

/// Companions of a fixed payload file (same folder, same original stem) share the shortest
/// fixed stem, so `a:b.pak`, `a:b.ucas` and `a:b.utoc` stay one container.
fn align_companions(
    kept: &[(&FileItem, &Vec<String>)],
    changed: &[bool],
    dests: &mut [Vec<String>],
) {
    let mut shortest: HashMap<(String, String), String> = HashMap::new();
    let key_of = |position: usize, dest: &[String]| {
        let original = kept[position].1.last().map_or("", String::as_str);
        let folder = dest.split_last().map_or(&[][..], |(_, folder)| folder);
        (folded_path(folder), stem_of(original).to_owned())
    };
    let payload = |name: &str| is_live_payload_name(name) || is_off_payload_name(name);
    for (position, dest) in dests.iter().enumerate() {
        let Some(name) = dest.last() else { continue };
        if !changed[position] || !payload(name) {
            continue;
        }
        let stem = stem_of(name).to_owned();
        shortest
            .entry(key_of(position, dest))
            .and_modify(|best| {
                if stem.len() < best.len() {
                    best.clone_from(&stem);
                }
            })
            .or_insert(stem);
    }
    for (position, dest) in dests.iter_mut().enumerate() {
        let key = key_of(position, dest);
        let Some(name) = dest.last_mut() else {
            continue;
        };
        if !payload(name) {
            continue;
        }
        if let Some(stem) = shortest.get(&key) {
            let original = kept[position].1.last().map_or("", String::as_str);
            *name = format!("{stem}{}", extension_of(original));
        }
    }
}

/// The candidate's folder name: its issue and the fix (`None` when valid).
fn candidate_name(
    name: &str,
    root: RootKind,
    issues: &mut Issues,
) -> (Option<crate::safe_fs::names::NameIssue>, Option<String>) {
    let issue = folder_name_issue(name);
    let fixed = issue.map(|_| fix_folder_name(name));
    if let Some(fixed) = &fixed {
        if folder_name_issue(fixed).is_some() {
            issues.block(IssueCode::InvalidName, name);
        } else {
            issues.warn(IssueCode::InvalidName, name);
        }
    }
    let final_name = fixed.as_deref().unwrap_or(name);
    if root == RootKind::Logic && final_name.len() > LOGIC_NAME_MAX_BYTES {
        issues.warn(IssueCode::NotPortable, final_name);
    }
    (issue, fixed)
}

/// Two candidates whose final names are fold-equal while one of them was fixed.
fn check_candidate_collisions(candidates: &[Candidate], issues: &mut Issues) {
    for (position, candidate) in candidates.iter().enumerate() {
        let final_name = candidate.fixed_name.as_ref().unwrap_or(&candidate.name);
        let clash = candidates[..position].iter().any(|other| {
            let other_name = other.fixed_name.as_ref().unwrap_or(&other.name);
            (candidate.fixed_name.is_some() || other.fixed_name.is_some())
                && fold_key(other_name) == fold_key(final_name)
        });
        if clash {
            issues.block(IssueCode::InvalidName, &candidate.name);
        }
    }
}

/// W1: the longest final path as the game sees it (P1 game-view limit).
fn is_path_too_long(input: &CandidateInput<'_>, name: &str, files: &[StoredFile]) -> bool {
    let base = input.game_view_len + 1 + utf16_len(input.root_name) + 1 + utf16_len(name);
    let logic_extra = if input.root == RootKind::Logic {
        DISABLED_PREFIX.len()
    } else {
        0
    };
    let mut longest_file = 0;
    let mut longest_dir = base + logic_extra;
    for file in files {
        let mut length = base;
        for (depth, part) in file.dest.iter().enumerate() {
            length += 1 + utf16_len(part);
            if depth + 1 < file.dest.len() {
                longest_dir = longest_dir.max(length + logic_extra);
            }
        }
        longest_file = longest_file.max(length + logic_extra);
    }
    path_too_long(longest_file, longest_dir).is_some()
}

/// The candidate folder's path inside the source, `/`-joined ("" = top level).
fn inner_path(input: &CandidateInput<'_>, item: &FileItem, rel: &[String]) -> String {
    let position = input
        .items
        .iter()
        .position(|other| std::ptr::eq(other, item))
        .unwrap_or_default();
    let source = &input.source_paths[position];
    let keep = source.len().saturating_sub(rel.len());
    source[..keep].join("/")
}
