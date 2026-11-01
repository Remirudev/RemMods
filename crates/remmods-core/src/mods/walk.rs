//! One folder walk per mod (spec §4.5 step 3): iterative, never follows links below the walk root.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use super::listing::{Entry, EntryKind, Lister, problem};
use super::model::{PayloadCounts, PayloadFile, PayloadKind, ScanProblem};
use super::payload::{Payload, classify_payload, strip_off_suffix};
use crate::content::UtocRef;
use crate::safe_fs::names::{
    DISABLED_PREFIX, NAME_MAX_BYTES, PAYLOAD_NAME_MAX_BYTES, display_escaped, fold_key, order_key,
    portable_name_issue, utf16_len, windows_name_issue,
};

/// Disabling a payload file appends `-x`.
const OFF_SUFFIX_UNITS: usize = 2;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WalkOptions {
    /// UTF-16 length of Paks as the game sees it.
    pub game_view_len: usize,
    /// UTF-16 length of the walk root relative to Paks (`Mods\Cool`), lossy for non-Unicode names.
    pub root_rel_len: usize,
    /// Paths below an unprefixed Logic folder grow by `.disabled_` when it is disabled.
    pub logic_unprefixed: bool,
    /// `read_details`: keep every payload file.
    pub collect_files: bool,
}

/// A link to a directory found below the walk root; it is reported, never followed.
#[derive(Clone, Debug)]
pub(crate) struct LinkedDir {
    /// Key texts below the walk root.
    pub keys: Vec<String>,
    pub path: PathBuf,
}

/// A regular file (or link to one) directly in the walk root.
#[derive(Clone, Debug)]
pub(crate) struct TopFile {
    pub name: std::ffi::OsString,
    pub path: PathBuf,
    pub size: Option<u64>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Walk {
    pub counts: PayloadCounts,
    pub size: u64,
    /// The walk root itself could not be listed.
    pub root_unreadable: bool,
    /// The first read failure anywhere in the walk.
    pub problem: Option<ScanProblem>,
    /// Live `.pak` basenames: fold key → display name (the smallest by `order_key`).
    pub live_paks: BTreeMap<String, String>,
    /// Live `.pak/.ucas/.utoc` basenames, same shape.
    pub live_loadable: BTreeMap<String, String>,
    /// Every payload basename with `-x` stripped, same shape.
    pub payload_names: BTreeMap<String, String>,
    pub non_unicode_files: u32,
    pub invalid_file_names: u32,
    pub non_portable_file_names: u32,
    pub longest_file: usize,
    pub longest_dir: usize,
    pub top_files: Vec<TopFile>,
    pub links: Vec<LinkedDir>,
    /// `collect_files` only.
    pub files: Vec<PayloadFile>,
    pub other_files: u32,
    /// Every `.utoc` / `.utoc-x` (fs form), for the content reader.
    pub utocs: Vec<UtocRef>,
}

impl Walk {
    pub(crate) fn live_payload_files(&self) -> u32 {
        self.counts
            .live_pak
            .saturating_add(self.counts.live_companion)
    }
}

struct Pending {
    path: PathBuf,
    keys: Vec<String>,
    rel_len: usize,
}

pub(crate) fn walk(lister: &mut Lister, root: &Path, options: WalkOptions) -> Walk {
    let mut walk = Walk::default();
    let mut stack = vec![Pending {
        path: root.to_path_buf(),
        keys: Vec::new(),
        rel_len: options.root_rel_len,
    }];
    while let Some(dir) = stack.pop() {
        let is_root = dir.keys.is_empty();
        let logic_extra = if options.logic_unprefixed {
            DISABLED_PREFIX.len()
        } else {
            0
        };
        let dir_len = options.game_view_len + 1 + dir.rel_len + logic_extra;
        walk.longest_dir = walk.longest_dir.max(dir_len);
        let entries = match lister.list(&dir.path) {
            Ok(entries) => entries,
            Err(error) => {
                walk.root_unreadable |= is_root;
                walk.problem.get_or_insert(problem("list", &error));
                continue;
            }
        };
        for entry in entries {
            visit(
                &mut walk, &mut stack, &dir, entry, is_root, dir_len, options,
            );
        }
    }
    walk
}

fn visit(
    walk: &mut Walk,
    stack: &mut Vec<Pending>,
    dir: &Pending,
    entry: Entry,
    is_root: bool,
    dir_len: usize,
    options: WalkOptions,
) {
    if let Some(found) = entry.problem {
        walk.problem.get_or_insert(found);
        return;
    }
    let name_units = utf16_len(&entry.name.to_string_lossy());
    let child_keys = || {
        let mut keys = dir.keys.clone();
        keys.push(entry.key.clone());
        keys
    };
    match entry.kind {
        EntryKind::Dir => stack.push(Pending {
            keys: child_keys(),
            path: entry.path,
            rel_len: dir.rel_len + 1 + name_units,
        }),
        EntryKind::LinkToDir => walk.links.push(LinkedDir {
            keys: child_keys(),
            path: entry.path,
        }),
        EntryKind::File | EntryKind::LinkToFile => {
            let file_len = dir_len + 1 + name_units;
            visit_file(walk, &entry, &dir.keys, is_root, file_len, options);
        }
        EntryKind::Other => {}
    }
}

fn visit_file(
    walk: &mut Walk,
    entry: &Entry,
    dir_keys: &[String],
    is_root: bool,
    file_len: usize,
    options: WalkOptions,
) {
    let stat = match entry.stat() {
        Ok(stat) => Some(stat),
        Err(found) => {
            walk.problem.get_or_insert(found);
            None
        }
    };
    let size = stat.map(|stat| stat.size);
    walk.size = walk.size.saturating_add(size.unwrap_or(0));
    let payload = classify_payload(&entry.name);
    let live_payload = payload.is_some_and(|payload| payload.live);
    let file_len = file_len + if live_payload { OFF_SUFFIX_UNITS } else { 0 };
    walk.longest_file = walk.longest_file.max(file_len);
    count_name(walk, &entry.name, live_payload);
    if is_root {
        walk.top_files.push(TopFile {
            name: entry.name.clone(),
            path: entry.path.clone(),
            size,
        });
    }
    match payload {
        Some(payload) => record_payload(walk, &entry.key, payload),
        None => walk.other_files = walk.other_files.saturating_add(1),
    }
    let Some(payload) = payload else {
        return;
    };
    let is_utoc = payload.kind == PayloadKind::Utoc;
    if !options.collect_files && !is_utoc {
        return;
    }
    let mut keys = dir_keys.to_vec();
    keys.push(entry.key.clone());
    let rel_path = display_rel_path(&keys);
    if is_utoc {
        walk.utocs.push(UtocRef {
            path: entry.path.clone(),
            rel_path: rel_path.clone(),
            live: payload.live,
            size: size.unwrap_or(0),
            modified: stat.and_then(|stat| stat.modified),
        });
    }
    if options.collect_files {
        walk.files.push(PayloadFile {
            rel_path,
            kind: payload.kind,
            live: payload.live,
            size: size.unwrap_or(0),
        });
    }
}

fn count_name(walk: &mut Walk, name: &OsStr, live_payload: bool) {
    let Some(text) = name.to_str() else {
        walk.non_unicode_files = walk.non_unicode_files.saturating_add(1);
        return;
    };
    if windows_name_issue(text).is_some() {
        walk.invalid_file_names = walk.invalid_file_names.saturating_add(1);
        return;
    }
    let budget = if live_payload {
        PAYLOAD_NAME_MAX_BYTES
    } else {
        NAME_MAX_BYTES
    };
    if portable_name_issue(text, budget).is_some() {
        walk.non_portable_file_names = walk.non_portable_file_names.saturating_add(1);
    }
}

fn record_payload(walk: &mut Walk, key: &str, payload: Payload) {
    let counts = &mut walk.counts;
    let counter = match (payload.is_pak(), payload.live) {
        (true, true) => &mut counts.live_pak,
        (true, false) => &mut counts.off_pak,
        (false, true) => &mut counts.live_companion,
        (false, false) => &mut counts.off_companion,
    };
    *counter = counter.saturating_add(1);
    let display = display_escaped(key);
    let stripped = strip_off_suffix(&display).to_owned();
    insert_smallest(&mut walk.payload_names, fold_key(&stripped), stripped);
    if payload.live && payload.is_loadable() {
        let fold = fold_key(&display);
        if payload.is_pak() {
            insert_smallest(&mut walk.live_paks, fold.clone(), display.clone());
        }
        insert_smallest(&mut walk.live_loadable, fold, display);
    }
}

/// Keeps the smallest display name per fold key, so read_dir order never shows.
pub(crate) fn insert_smallest(map: &mut BTreeMap<String, String>, fold: String, display: String) {
    match map.get_mut(&fold) {
        Some(existing) if order_key(&display) < order_key(existing) => *existing = display,
        Some(_) => {}
        None => {
            map.insert(fold, display);
        }
    }
}

/// Key texts joined by `/`, each through `display_escaped` (spec §4.4 rel paths).
pub(crate) fn display_rel_path<S: AsRef<str>>(keys: &[S]) -> String {
    keys.iter()
        .map(|key| display_escaped(key.as_ref()))
        .collect::<Vec<_>>()
        .join("/")
}
