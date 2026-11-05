//! The read-only half of an install (spec §5.3): from a listing to an `Inspection`. Writes
//! nothing.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::candidates::{CandidateInput, build_candidates};
use super::detect_root::detect_root;
use super::files::{FileItem, Files, collect_files, folded_path};
use super::issues::{Issues, count};
use super::model::{
    Candidate, InspectOptions, Inspection, IssueCode, IssueSeverity, RootReason, VariantGroup,
};
use super::plan::{PlanFile, plan_candidates};
use super::service::StoredInspection;
use crate::archive::{Entry, EntryKind, Listing, SourceFingerprint};
use crate::mods::{RootKind, ScanContent, ScanInput};
use crate::platform;

/// Space headroom: an install needs 1.1 × its bytes.
pub(crate) fn needs_space(bytes: u64) -> u64 {
    bytes.saturating_add(bytes / 10)
}

pub(crate) struct InspectInput<'a> {
    pub(crate) source: &'a Path,
    pub(crate) folder: bool,
    pub(crate) fingerprint: Option<SourceFingerprint>,
    pub(crate) source_name: String,
    pub(crate) source_stem: String,
    pub(crate) options: &'a InspectOptions,
    pub(crate) listing: &'a Listing,
    pub(crate) entries: &'a [Entry],
    pub(crate) content: &'a ScanContent,
    pub(crate) scan_input: &'a ScanInput,
    pub(crate) available_override: Option<u64>,
}

pub(crate) fn inspect(input: &InspectInput<'_>) -> StoredInspection {
    let mut issues = Issues::default();
    let mut inspection = Inspection {
        id: format!("insp-{}", uuid::Uuid::new_v4().simple()),
        source_name: input.source_name.clone(),
        format: input.listing.format,
        encoding: input.options.encoding,
        legacy_names: false,
        entries: count(input.entries.len()),
        files: count(
            input
                .entries
                .iter()
                .filter(|entry| entry.kind == EntryKind::File)
                .count(),
        ),
        total_bytes: input
            .entries
            .iter()
            .filter(|entry| entry.kind == EntryKind::File)
            .map(|entry| entry.size)
            .sum(),
        detected_root: RootKind::Standard,
        root_reason: RootReason::Default,
        root: input.options.root.unwrap_or(RootKind::Standard),
        candidates: Vec::new(),
        groups: Vec::new(),
        clashes: Vec::new(),
        issues: Vec::new(),
        blocked: false,
        available_bytes: None,
    };
    let mut stored = StoredInspection {
        source: input.source.to_path_buf(),
        folder: input.folder,
        fingerprint: input.fingerprint,
        entry_count: input.listing.entries.len(),
        candidates: Vec::new(),
        clash_members: Vec::new(),
        inspection: inspection.clone(),
    };
    if let Some(files) = collect_files(input.listing, input.entries, &mut issues) {
        inspection.legacy_names = files.legacy;
        plan_files(input, files, &mut inspection, &mut stored, &mut issues);
    }
    inspection.blocked = issues.blocked();
    inspection.issues = issues.into_list();
    stored.inspection = inspection;
    stored
}

fn plan_files(
    input: &InspectInput<'_>,
    files: Files,
    inspection: &mut Inspection,
    stored: &mut StoredInspection,
    issues: &mut Issues,
) {
    inspection.clashes = files.clashes;
    stored.clash_members = files.clash_members;
    let paths: Vec<Vec<String>> = files.items.iter().map(|item| item.path.clone()).collect();
    check_file_folder_overlap(&files.items, issues);
    let detection = detect_root(&paths);
    for code in &detection.blocks {
        issues.block(*code, "");
    }
    let outside: Vec<&FileItem> = files
        .items
        .iter()
        .zip(&detection.stripped)
        .filter(|(_, stripped)| stripped.is_none())
        .map(|(item, _)| item)
        .collect();
    if let Some(first) = outside.first() {
        let n = count(outside.len());
        issues.add_count(
            IssueSeverity::Info,
            IssueCode::OutsideRoot,
            &first.display,
            n,
        );
    }
    inspection.detected_root = detection.kind;
    inspection.root_reason = detection.reason;
    inspection.root = detection.kind;
    if let Some(chosen) = input.options.root {
        inspection.root = chosen;
        inspection.root_reason = RootReason::Chosen;
    }
    let mut items = Vec::new();
    let mut source_paths = Vec::new();
    for (item, stripped) in files.items.iter().zip(detection.stripped) {
        if let Some(path) = stripped {
            source_paths.push(item.path.clone());
            items.push(FileItem {
                path,
                ..item.clone()
            });
        }
    }
    let plan_input: Vec<PlanFile> = items
        .iter()
        .map(|item| PlanFile {
            path: item.path.clone(),
        })
        .collect();
    let plan = plan_candidates(&plan_input, &input.source_stem);
    if let Some(&first) = plan.extra_files.first() {
        let n = count(plan.extra_files.len());
        issues.add_count(
            IssueSeverity::Info,
            IssueCode::ExtraFiles,
            &items[first].display,
            n,
        );
    }
    let root_path = root_path(input.content, input.scan_input, inspection.root);
    let root_name = root_path
        .as_ref()
        .and_then(|path| path.file_name())
        .map_or_else(
            || inspection.root.folder().to_owned(),
            |name| name.to_string_lossy().into_owned(),
        );
    let candidate_input = CandidateInput {
        items: &items,
        source_paths: &source_paths,
        root: inspection.root,
        root_name: &root_name,
        game_view_len: input.scan_input.game_view_len,
        names: &input.options.names,
        content: input.content,
    };
    let (candidates, stored_candidates) = build_candidates(&candidate_input, &plan, issues);
    inspection.groups = plan
        .groups
        .iter()
        .map(|group| VariantGroup {
            id: group.id,
            candidates: group.candidates.clone(),
        })
        .collect();
    let space_at = root_path.unwrap_or_else(|| input.scan_input.paks.clone());
    inspection.available_bytes = input
        .available_override
        .or_else(|| available_space(&space_at));
    check_space(&candidates, inspection.available_bytes, issues);
    inspection.candidates = candidates;
    stored.candidates = stored_candidates;
}

/// A file path that is also (fold-equal) a folder prefix of another file.
fn check_file_folder_overlap(items: &[FileItem], issues: &mut Issues) {
    let folders: HashSet<String> = items
        .iter()
        .flat_map(|item| (1..item.path.len()).map(|depth| folded_path(&item.path[..depth])))
        .collect();
    for item in items {
        if folders.contains(&folded_path(&item.path)) {
            issues.block(IssueCode::DuplicateDiffers, &item.display);
        }
    }
}

fn check_space(candidates: &[Candidate], available: Option<u64>, issues: &mut Issues) {
    let bytes: u64 = candidates
        .iter()
        .filter(|candidate| candidate.selected)
        .map(|candidate| candidate.bytes)
        .sum();
    if available.is_some_and(|available| available < needs_space(bytes)) {
        issues.warn(IssueCode::NoSpace, "");
    }
}

pub(crate) fn root_path(
    content: &ScanContent,
    input: &ScanInput,
    kind: RootKind,
) -> Option<PathBuf> {
    super::existing::root_folder(content, input, kind)
}

pub(crate) fn available_space(path: &Path) -> Option<u64> {
    platform::fs_path(path).and_then(fs4::available_space).ok()
}
