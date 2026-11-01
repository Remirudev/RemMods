//! Resolving a ModRef back to the disk, the details drawer's file list and the path handed to the
//! file manager (spec §4.7, §4.8).

use std::path::{Path, PathBuf};

use super::error::ModsError;
use super::listing::{Entry, Lister};
use super::model::{DETAILS_MAX_FILES, ModDetails, PayloadFile, ScanInput};
use super::modref::{ModRef, ParsedRef, RefKind, format_file_id};
use super::payload::classify_payload;
use super::strategy::{LooseFile, group_loose_files};
use super::walk::{WalkOptions, walk};
use crate::platform::{self, OsKind};
use crate::safe_fs::FsError;
use crate::safe_fs::names::{display_escaped, fold_key, order_key, windows_name_issue};

pub fn read_details(input: &ScanInput, mod_ref: &ModRef) -> Result<ModDetails, ModsError> {
    let parsed = ModRef::parse(mod_ref.as_str())?;
    let resolved = resolve_parsed(input, &parsed)?;
    let (location, mut files, other_files) = match parsed.kind {
        RefKind::File => return Err(ModsError::BadRef),
        RefKind::Dir => {
            let options = WalkOptions {
                collect_files: true,
                ..WalkOptions::default()
            };
            let walked = walk(&mut Lister::default(), &resolved.path, options);
            if walked.root_unreadable {
                log::debug!("details: cannot list {}", resolved.path.display());
            }
            (resolved.path, walked.files, walked.other_files)
        }
        RefKind::Loose => (resolved.container, loose_files(&resolved.group), 0),
    };
    files.sort_by_cached_key(|file| order_key(&file.rel_path));
    let truncated = files.len() > DETAILS_MAX_FILES;
    files.truncate(DETAILS_MAX_FILES);
    Ok(ModDetails {
        mod_ref: mod_ref.clone(),
        location,
        files,
        truncated,
        other_files,
    })
}

/// The entry a token names, in the mod-side form; `Stale` when it no longer matches the disk.
pub fn resolve_ref(input: &ScanInput, mod_ref: &ModRef) -> Result<PathBuf, ModsError> {
    let parsed = ModRef::parse(mod_ref.as_str())?;
    Ok(resolve_parsed(input, &parsed)?.path)
}

/// `resolve_ref` for the file manager. On Windows every component must be a name Win32 keeps as
/// is; the path is then simplified (no `\\?\`), because Explorer does not take verbatim paths.
pub fn shell_form(input: &ScanInput, mod_ref: &ModRef, os: OsKind) -> Result<PathBuf, ModsError> {
    let parsed = ModRef::parse(mod_ref.as_str())?;
    let resolved = resolve_parsed(input, &parsed)?;
    if os != OsKind::Windows {
        return Ok(resolved.path);
    }
    let shell_safe = |key: &str| !key.contains('\\') && windows_name_issue(key).is_none();
    let components_safe = parsed.components.iter().all(|key| shell_safe(key));
    let file_safe = resolved
        .group
        .first()
        .is_none_or(|file| shell_safe(&file.key));
    if !(components_safe && file_safe) {
        return Err(ModsError::NotShellSafe);
    }
    Ok(dunce::simplified(&resolved.path).to_path_buf())
}

struct Resolved {
    /// The entry itself (loose: the group's first file).
    path: PathBuf,
    /// The folder holding it (loose: the root).
    container: PathBuf,
    /// Loose tokens: the group's files in `order_key` order.
    group: Vec<LooseFile>,
}

fn resolve_parsed(input: &ScanInput, parsed: &ParsedRef) -> Result<Resolved, ModsError> {
    if parsed.paks_id != input.paks_id {
        return Err(ModsError::Stale);
    }
    let mut container = platform::fs_path(&input.paks)
        .map_err(|error| FsError::io("resolve", &input.paks, error))?;
    let (last, parents) = parsed.components.split_last().ok_or(ModsError::BadRef)?;
    let mut lister = Lister::default();
    for key in parents {
        let entry = find_entry(&mut lister, &container, key)?;
        if !entry.kind.is_dir_like() {
            return Err(ModsError::Stale);
        }
        container = entry.path;
    }
    if parsed.kind == RefKind::Loose {
        let group = loose_group(&mut lister, &container, last)?;
        let path = group
            .first()
            .map(|file| file.path.clone())
            .ok_or(ModsError::Stale)?;
        return Ok(Resolved {
            path,
            container,
            group,
        });
    }
    let entry = find_entry(&mut lister, &container, last)?;
    let kind_matches = match parsed.kind {
        RefKind::Dir => entry.kind.is_dir_like(),
        _ => entry.kind.is_file_like(),
    };
    if !kind_matches {
        return Err(ModsError::Stale);
    }
    if let Some(expected) = &parsed.file_id {
        let actual = platform::file_id(&entry.path).map(format_file_id);
        if actual.as_ref().ok() != Some(expected) {
            return Err(ModsError::Stale);
        }
    }
    Ok(Resolved {
        path: entry.path,
        container,
        group: Vec::new(),
    })
}

fn find_entry(lister: &mut Lister, container: &Path, key: &str) -> Result<Entry, ModsError> {
    list(lister, container)?
        .into_iter()
        .find(|entry| entry.key == key)
        .ok_or(ModsError::Stale)
}

fn list(lister: &mut Lister, container: &Path) -> Result<Vec<Entry>, ModsError> {
    lister.list(container).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            ModsError::Stale
        } else {
            ModsError::Fs(FsError::io("resolve", container, error))
        }
    })
}

/// The payload files in `root` whose stem folds like `stem_key`.
fn loose_group(
    lister: &mut Lister,
    root: &Path,
    stem_key: &str,
) -> Result<Vec<LooseFile>, ModsError> {
    let wanted = fold_key(stem_key);
    let files: Vec<LooseFile> = list(lister, root)?
        .into_iter()
        .filter(|entry| entry.problem.is_none() && entry.kind.is_file_like())
        .filter_map(|entry| {
            let payload = classify_payload(&entry.name)?;
            Some(LooseFile {
                size: entry.size(),
                name: entry.name,
                key: entry.key,
                path: entry.path,
                payload,
            })
        })
        .filter(|file| fold_key(file.stem_key()) == wanted)
        .collect();
    Ok(group_loose_files(files)
        .into_iter()
        .next()
        .map(|group| group.files)
        .unwrap_or_default())
}

fn loose_files(group: &[LooseFile]) -> Vec<PayloadFile> {
    group
        .iter()
        .map(|file| PayloadFile {
            rel_path: display_escaped(&file.key),
            kind: file.payload.kind,
            live: file.payload.live,
            size: file.size.unwrap_or(0),
        })
        .collect()
}
