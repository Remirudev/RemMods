//! `scan_paks`: one read-only pass over Paks and its mod roots (spec §4.5).

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use super::conflicts::{link_conflicts, mark_ambiguous, mark_case_collisions};
use super::error::ModsError;
use super::listing::{Entry, EntryKind, Lister, problem};
use super::model::{
    ArchiveEntry, LeftoverEntry, RootInfo, RootKind, ScanContent, ScanInput, ScanOutput, ScanStats,
    UnmanagedEntry, UnmanagedKind,
};
use super::modref::{ModRef, RefKind};
use super::payload::{classify_payload, is_archive, is_game_file};
use super::rows::{RootRef, RowDraft, folder_row, loose_row};
use super::strategy::{LooseFile, group_loose_files};
use super::unmanaged::{UnmanagedDraft, mark_loose_copies};
use super::walk::{Walk, WalkOptions, display_rel_path, walk};
use crate::platform;
use crate::safe_fs::FsError;
use crate::safe_fs::names::{
    display_escaped, fold_key, has_disabled_prefix_ascii, is_install_temp_name, order_key,
    utf16_len,
};

pub fn scan_paks(input: &ScanInput) -> Result<ScanOutput, ModsError> {
    let started = Instant::now();
    let base =
        platform::fs_path(&input.paks).map_err(|error| FsError::io("scan", &input.paks, error))?;
    let mut scan = Scan {
        input,
        lister: Lister::default(),
        roots: Vec::new(),
        drafts: Vec::new(),
        archives: Vec::new(),
        unmanaged: Vec::new(),
        leftovers: Vec::new(),
    };
    let top = scan
        .lister
        .list(&base)
        .map_err(|error| FsError::io("scan", &input.paks, error))?;
    let mut root_entries = Vec::new();
    let mut folders = Vec::new();
    for entry in top {
        match (entry.kind, root_kind(&entry)) {
            (EntryKind::Dir | EntryKind::LinkToDir, Some(kind)) => root_entries.push((kind, entry)),
            (EntryKind::Dir, None) => folders.push(entry),
            _ => scan.top_level_entry(&entry),
        }
    }
    for (kind, entry, case_variant) in order_roots(root_entries) {
        scan.root(kind, &entry, case_variant);
    }
    for entry in folders {
        scan.non_root_folder(&entry);
    }
    let (dirs_read, entries_seen) = (scan.lister.dirs_read, scan.lister.entries_seen);
    let content = scan.finish();
    let stats = ScanStats {
        dirs_read,
        entries_seen,
        duration: started.elapsed(),
    };
    Ok(ScanOutput { content, stats })
}

struct Scan<'a> {
    input: &'a ScanInput,
    lister: Lister,
    roots: Vec<RootInfo>,
    drafts: Vec<RowDraft>,
    archives: Vec<ArchiveEntry>,
    unmanaged: Vec<UnmanagedDraft>,
    leftovers: Vec<LeftoverEntry>,
}

impl Scan<'_> {
    /// A Paks top-level entry that is neither a root nor a plain folder.
    fn top_level_entry(&mut self, entry: &Entry) {
        if let Some(found) = entry.problem {
            log::debug!("scan: cannot stat {}: {found:?}", entry.path.display());
            return;
        }
        match entry.kind {
            EntryKind::LinkToDir => self.push_unmanaged(
                UnmanagedKind::LinkedFolder,
                RefKind::Dir,
                entry,
                &[&entry.key],
                0,
                BTreeMap::new(),
            ),
            EntryKind::File | EntryKind::LinkToFile => {
                if is_game_file(&entry.name) {
                    return;
                }
                if let Some(payload) = classify_payload(&entry.name)
                    && payload.live
                    && payload.is_loadable()
                {
                    let display = display_escaped(&entry.key);
                    let names = BTreeMap::from([(fold_key(&display), display)]);
                    self.push_unmanaged(
                        UnmanagedKind::PaksFile,
                        RefKind::File,
                        entry,
                        &[&entry.key],
                        1,
                        names,
                    );
                } else if is_archive(&entry.name) {
                    self.push_archive(entry, &[&entry.key]);
                }
            }
            EntryKind::Dir | EntryKind::Other => {}
        }
    }

    fn root(&mut self, kind: RootKind, entry: &Entry, case_variant: bool) {
        let root = RootRef {
            kind,
            key: entry.key.clone(),
            path: entry.path.clone(),
            is_link: entry.kind.is_link(),
            case_variant,
            units: utf16_len(&entry.name.to_string_lossy()),
        };
        let listed = self.lister.list(&root.path);
        self.roots.push(RootInfo {
            kind,
            name: display_escaped(&root.key),
            path: root.path.clone(),
            is_link: root.is_link,
            case_variant,
            unreadable: listed.as_ref().err().map(|error| problem("list", error)),
        });
        let Ok(entries) = listed else {
            return;
        };
        let mut loose = Vec::new();
        for child in entries {
            if child.problem.is_some() {
                continue;
            }
            if child.kind.is_dir_like() {
                self.root_folder(&root, &child);
            } else if child.kind.is_file_like() {
                if let Some(payload) = classify_payload(&child.name) {
                    loose.push(LooseFile {
                        size: child.size(),
                        name: child.name,
                        key: child.key,
                        path: child.path,
                        payload,
                    });
                } else if is_archive(&child.name) {
                    self.push_archive(&child, &[&root.key, &child.key]);
                }
            }
        }
        for group in group_loose_files(loose) {
            let draft = loose_row(&root, &group, &self.input.paks_id, self.input.game_view_len);
            self.drafts.push(draft);
        }
    }

    /// A folder in a root: an install temp folder or a mod folder row.
    fn root_folder(&mut self, root: &RootRef, entry: &Entry) {
        let options = WalkOptions {
            game_view_len: self.input.game_view_len,
            root_rel_len: root.units + 1 + utf16_len(&entry.name.to_string_lossy()),
            logic_unprefixed: root.kind == RootKind::Logic
                && !has_disabled_prefix_ascii(&entry.name),
            collect_files: false,
        };
        let walked = walk(&mut self.lister, &entry.path, options);
        let keys = [root.key.as_str(), entry.key.as_str()];
        self.push_links(&keys, &walked);
        if is_install_temp_name(&entry.name) {
            let live = walked.live_payload_files();
            if live > 0 {
                let names = walked.live_loadable;
                self.push_unmanaged(
                    UnmanagedKind::TempFolder,
                    RefKind::Dir,
                    entry,
                    &keys,
                    live,
                    names,
                );
            } else {
                self.leftovers.push(LeftoverEntry {
                    mod_ref: self.mod_ref(RefKind::Dir, &entry.path, &keys),
                    rel_path: display_rel_path(&keys),
                });
            }
            return;
        }
        let mod_ref = self.mod_ref(RefKind::Dir, &entry.path, &keys);
        self.drafts.push(folder_row(root, entry, walked, mod_ref));
    }

    fn non_root_folder(&mut self, entry: &Entry) {
        let options = WalkOptions {
            game_view_len: self.input.game_view_len,
            root_rel_len: utf16_len(&entry.name.to_string_lossy()),
            ..WalkOptions::default()
        };
        let walked = walk(&mut self.lister, &entry.path, options);
        let keys = [entry.key.as_str()];
        self.push_links(&keys, &walked);
        let live = walked.live_payload_files();
        if live > 0 {
            self.push_unmanaged(
                UnmanagedKind::Folder,
                RefKind::Dir,
                entry,
                &keys,
                live,
                walked.live_loadable,
            );
        }
    }

    fn push_links(&mut self, parent: &[&str], walked: &Walk) {
        for link in &walked.links {
            let mut keys: Vec<&str> = parent.to_vec();
            keys.extend(link.keys.iter().map(String::as_str));
            let entry = UnmanagedEntry {
                mod_ref: self.mod_ref(RefKind::Dir, &link.path, &keys),
                kind: UnmanagedKind::LinkedFolder,
                rel_path: display_rel_path(&keys),
                live_files: 0,
                copy_of: Vec::new(),
            };
            self.unmanaged.push(UnmanagedDraft {
                entry,
                names: BTreeMap::new(),
            });
        }
    }

    fn push_unmanaged(
        &mut self,
        kind: UnmanagedKind,
        ref_kind: RefKind,
        entry: &Entry,
        keys: &[&str],
        live_files: u32,
        names: BTreeMap<String, String>,
    ) {
        let entry = UnmanagedEntry {
            mod_ref: self.mod_ref(ref_kind, &entry.path, keys),
            kind,
            rel_path: display_rel_path(keys),
            live_files,
            copy_of: Vec::new(),
        };
        self.unmanaged.push(UnmanagedDraft { entry, names });
    }

    fn push_archive(&mut self, entry: &Entry, keys: &[&str]) {
        let size = entry.size().unwrap_or_default();
        self.archives.push(ArchiveEntry {
            mod_ref: self.mod_ref(RefKind::File, &entry.path, keys),
            rel_path: display_rel_path(keys),
            size,
        });
    }

    fn mod_ref(&self, kind: RefKind, path: &Path, keys: &[&str]) -> ModRef {
        let file_id = platform::file_id(path).ok();
        ModRef::build(kind, file_id, &self.input.paks_id, keys)
    }

    fn finish(mut self) -> ScanContent {
        mark_ambiguous(&mut self.drafts);
        mark_case_collisions(&mut self.drafts);
        link_conflicts(&mut self.drafts);
        mark_loose_copies(&mut self.drafts, &mut self.unmanaged);
        self.drafts.sort_by(compare_drafts);
        let rows = self
            .drafts
            .into_iter()
            .map(|mut draft| {
                draft.row.issues.sort_by_key(super::issues::ModIssue::rank);
                draft.row
            })
            .collect();
        let mut unmanaged: Vec<UnmanagedEntry> =
            self.unmanaged.into_iter().map(|item| item.entry).collect();
        for entry in &mut unmanaged {
            entry.copy_of.sort_by_cached_key(|name| order_key(name));
        }
        unmanaged.sort_by_cached_key(|e| (order_key(&e.rel_path), e.mod_ref.as_str().to_owned()));
        self.archives
            .sort_by_cached_key(|e| (order_key(&e.rel_path), e.mod_ref.as_str().to_owned()));
        self.leftovers
            .sort_by_cached_key(|e| (order_key(&e.rel_path), e.mod_ref.as_str().to_owned()));
        ScanContent {
            paks: self.input.paks.clone(),
            paks_id: self.input.paks_id.clone(),
            roots: self.roots,
            rows,
            archives: self.archives,
            unmanaged,
            leftovers: self.leftovers,
        }
    }
}

fn root_kind(entry: &Entry) -> Option<RootKind> {
    let name = entry.name.as_encoded_bytes();
    RootKind::ALL
        .into_iter()
        .find(|kind| name.eq_ignore_ascii_case(kind.folder().as_bytes()))
}

/// Standard, Legacy, Logic; within a kind the literal name first, then by name bytes.
fn order_roots(mut roots: Vec<(RootKind, Entry)>) -> Vec<(RootKind, Entry, bool)> {
    let literal =
        |kind: RootKind, entry: &Entry| entry.name.as_encoded_bytes() != kind.folder().as_bytes();
    roots.sort_by(|(a_kind, a), (b_kind, b)| {
        a_kind
            .cmp(b_kind)
            .then_with(|| literal(*a_kind, a).cmp(&literal(*b_kind, b)))
            .then_with(|| a.name.as_encoded_bytes().cmp(b.name.as_encoded_bytes()))
    });
    let mut per_kind: BTreeMap<RootKind, usize> = BTreeMap::new();
    for (kind, _) in &roots {
        *per_kind.entry(*kind).or_default() += 1;
    }
    roots
        .into_iter()
        .map(|(kind, entry)| {
            let several = per_kind.get(&kind).is_some_and(|count| *count > 1);
            let case_variant = several && literal(kind, &entry);
            (kind, entry, case_variant)
        })
        .collect()
}

fn compare_drafts(a: &RowDraft, b: &RowDraft) -> Ordering {
    a.row
        .kind
        .cmp(&b.row.kind)
        .then_with(|| order_key(&a.logical_key).cmp(&order_key(&b.logical_key)))
        .then_with(|| a.root_key.as_bytes().cmp(b.root_key.as_bytes()))
        .then_with(|| a.folder_key.as_bytes().cmp(b.folder_key.as_bytes()))
        .then_with(|| a.row.mod_ref.as_str().cmp(b.row.mod_ref.as_str()))
}
