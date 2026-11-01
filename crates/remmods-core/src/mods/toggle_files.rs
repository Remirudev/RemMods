//! The files a toggle may rename: the same walk and link rule as the scan, so after an apply the
//! row is exactly Enabled or Disabled (spec §5.2).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use super::error::ModsError;
use super::listing::{Entry, EntryKind, Lister};
use super::model::{ModKind, ModRow, ScanInput};
use super::payload::classify_payload;
use super::strategy::{LooseFile, group_loose_files};
use crate::platform;
use crate::safe_fs::FsError;
use crate::safe_fs::names::fold_key;

const OFF_TEXT_SUFFIX: &[u8] = b".txt-x";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileClass {
    LivePayload,
    OffPayload,
    OffText,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModFile {
    /// Built from `ScanInput.paks` and the on-disk names (plain form).
    pub path: PathBuf,
    /// `/`-joined key texts below the mod folder (loose rows: the file's key text).
    pub rel: String,
    pub class: FileClass,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModFiles {
    /// The mod folder; loose rows: their root folder.
    pub base: PathBuf,
    pub files: Vec<ModFile>,
}

pub fn list_toggle_files(row: &ModRow, input: &ScanInput) -> Result<ModFiles, ModsError> {
    let fs_paks =
        platform::fs_path(&input.paks).map_err(|error| FsError::io("list", &input.paks, error))?;
    let below = row
        .path
        .strip_prefix(&fs_paks)
        .map_err(|_| ModsError::Stale)?;
    let base = input.paks.join(below);
    let mut lister = Lister::default();
    let files = if row.kind == ModKind::Loose {
        loose_files(&mut lister, row, &base)?
    } else {
        folder_files(&mut lister, &row.path, &base)?
    };
    Ok(ModFiles { base, files })
}

pub(crate) fn file_class(name: &OsStr) -> Option<FileClass> {
    if let Some(payload) = classify_payload(name) {
        return Some(if payload.live {
            FileClass::LivePayload
        } else {
            FileClass::OffPayload
        });
    }
    let bytes = name.as_encoded_bytes();
    let cut = bytes.len().checked_sub(OFF_TEXT_SUFFIX.len())?;
    bytes[cut..]
        .eq_ignore_ascii_case(OFF_TEXT_SUFFIX)
        .then_some(FileClass::OffText)
}

/// Every file at any depth; links to folders are not followed, links to files count (as in
/// the scan's walk).
fn folder_files(
    lister: &mut Lister,
    fs_root: &Path,
    base: &Path,
) -> Result<Vec<ModFile>, ModsError> {
    let mut files = Vec::new();
    let mut stack = vec![(fs_root.to_path_buf(), base.to_path_buf(), String::new())];
    while let Some((fs_dir, plain_dir, rel)) = stack.pop() {
        let entries = lister
            .list(&fs_dir)
            .map_err(|error| FsError::io("list", &plain_dir, error))?;
        for entry in entries {
            if entry.problem.is_some() {
                continue;
            }
            let child_rel = join_rel(&rel, &entry.key);
            let plain = plain_dir.join(&entry.name);
            match entry.kind {
                EntryKind::Dir => stack.push((entry.path, plain, child_rel)),
                EntryKind::File | EntryKind::LinkToFile => {
                    if let Some(class) = file_class(&entry.name) {
                        files.push(ModFile {
                            path: plain,
                            rel: child_rel,
                            class,
                        });
                    }
                }
                EntryKind::LinkToDir | EntryKind::Other => {}
            }
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// The payload files of the row's loose group (same grouping as the scan).
fn loose_files(lister: &mut Lister, row: &ModRow, base: &Path) -> Result<Vec<ModFile>, ModsError> {
    let stem = row
        .key
        .split_once('/')
        .map(|(_, stem)| stem)
        .ok_or(ModsError::Stale)?;
    let entries = lister
        .list(&row.path)
        .map_err(|error| FsError::io("list", base, error))?;
    let loose = entries.into_iter().filter_map(loose_file).collect();
    let wanted = fold_key(stem);
    let group = group_loose_files(loose)
        .into_iter()
        .find(|group| fold_key(group.stem_key()) == wanted)
        .ok_or(ModsError::Stale)?;
    Ok(group
        .files
        .into_iter()
        .map(|file| ModFile {
            path: base.join(&file.name),
            class: if file.payload.live {
                FileClass::LivePayload
            } else {
                FileClass::OffPayload
            },
            rel: file.key,
        })
        .collect())
}

fn loose_file(entry: Entry) -> Option<LooseFile> {
    if entry.problem.is_some() || !entry.kind.is_file_like() {
        return None;
    }
    let payload = classify_payload(&entry.name)?;
    Some(LooseFile {
        size: Ok(0),
        modified: None,
        name: entry.name,
        key: entry.key,
        path: entry.path,
        payload,
    })
}

fn join_rel(parent: &str, key: &str) -> String {
    if parent.is_empty() {
        key.to_owned()
    } else {
        format!("{parent}/{key}")
    }
}
