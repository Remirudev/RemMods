//! Installed mods a candidate would replace (spec §5.3 step 10).

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use super::model::{ExistingMatch, ExistingRelation};
use crate::mods::manage_target::plain_path;
use crate::mods::{ModKind, ModRow, RootKind, ScanContent, ScanInput};
use crate::platform;
use crate::safe_fs::names::{fold_key, key_text};

/// The row of `kind` named `name` (bytes, fold, or in Logic `.disabled_<name>`) and the other
/// roots holding the same fold name. Several matching rows make the match ambiguous. A loose
/// `<name>.pak` is not a match, nor is a mod in an author folder: an install lands directly in the
/// root, so it would not replace that folder.
pub(crate) fn find_existing(
    content: &ScanContent,
    kind: RootKind,
    name: &str,
) -> (Option<ExistingMatch>, Vec<RootKind>) {
    let wanted = fold_key(name);
    let mut matches: Vec<(&ModRow, ExistingRelation)> = content
        .rows
        .iter()
        .filter(|row| row_kind(row) == Some(kind) && !row.in_author_folder())
        .filter_map(|row| relation(row, name, &wanted).map(|relation| (row, relation)))
        .collect();
    matches.sort_by_key(|(_, relation)| relation_rank(*relation));
    let several = matches.len() > 1;
    let existing = matches.first().map(|(row, relation)| ExistingMatch {
        mod_ref: row.mod_ref.clone(),
        key: row.key.clone(),
        rel: format!("{}/{}", row.root, row.folder_name),
        relation: *relation,
        ambiguous: row.ambiguous || several,
    });
    let mut also_in: Vec<RootKind> = content
        .rows
        .iter()
        .filter_map(|row| row_kind(row).map(|other| (row, other)))
        .filter(|(row, other)| *other != kind && fold_key(&row.name) == wanted)
        .map(|(_, other)| other)
        .collect();
    also_in.sort_by_key(|kind| RootKind::ALL.iter().position(|each| each == kind));
    also_in.dedup();
    (existing, also_in)
}

fn relation(row: &ModRow, name: &str, wanted: &str) -> Option<ExistingRelation> {
    if row.kind == ModKind::Logic && row.logic_prefixed {
        return (fold_key(&row.name) == wanted).then_some(ExistingRelation::DisabledPrefix);
    }
    if row.folder_name == name {
        Some(ExistingRelation::SameName)
    } else if fold_key(&row.folder_name) == wanted {
        Some(ExistingRelation::FoldEqual)
    } else {
        None
    }
}

fn relation_rank(relation: ExistingRelation) -> u8 {
    match relation {
        ExistingRelation::SameName => 0,
        ExistingRelation::FoldEqual => 1,
        ExistingRelation::DisabledPrefix => 2,
    }
}

fn row_kind(row: &ModRow) -> Option<RootKind> {
    match row.kind {
        ModKind::Standard => Some(RootKind::Standard),
        ModKind::Legacy => Some(RootKind::Legacy),
        ModKind::Logic => Some(RootKind::Logic),
        ModKind::Loose => None,
    }
}

/// The root folder of `kind` (plain form): the canonical spelling when several case variants
/// exist, else the first. `None` when Paks has none.
pub(crate) fn root_folder(
    content: &ScanContent,
    input: &ScanInput,
    kind: RootKind,
) -> Option<PathBuf> {
    let roots: Vec<_> = content
        .roots
        .iter()
        .filter(|root| root.kind == kind)
        .collect();
    let root = roots
        .iter()
        .find(|root| root.name == kind.folder())
        .or_else(|| roots.first())?;
    plain_path(&root.path, input).ok()
}

/// Names in `root` fold-equal to `name`, other than `except` (the existing match's folder).
pub(crate) fn foreign_entries(root: &Path, name: &str, except: Option<&str>) -> bool {
    let Ok(entries) = platform::fs_path(root).and_then(fs::read_dir) else {
        return false;
    };
    let wanted = fold_key(name);
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .filter(|other: &OsString| fold_key(&key_text(other)) == wanted)
        .any(|other| except.is_none_or(|except| key_text(&other) != except))
}
