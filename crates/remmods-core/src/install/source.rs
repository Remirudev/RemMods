//! What a dropped or picked path is (spec §5.3 step 1). Reads metadata only.

use std::fs;
use std::path::{Path, PathBuf};

use super::model::{SourceKind, SourceReject};
use crate::platform;
use crate::safe_fs::{is_within, resolve_existing};

/// Missing → `Missing`; link → `Link`; folder → `Folder` unless it is, contains or lies inside
/// `paks` (`InsidePaks`); `.zip`/`.7z`/`.rar` (any ASCII case) → that kind; else
/// `UnsupportedType`. Archive files inside Paks are fine (they are only read).
pub fn classify_source(path: &Path, paks: Option<&Path>) -> Result<SourceKind, SourceReject> {
    let metadata = platform::fs_path(path)
        .and_then(fs::symlink_metadata)
        .map_err(|_| SourceReject::Missing)?;
    if metadata.file_type().is_symlink() {
        return Err(SourceReject::Link);
    }
    if metadata.is_dir() {
        if paks.is_some_and(|paks| overlaps(path, paks)) {
            return Err(SourceReject::InsidePaks);
        }
        return Ok(SourceKind::Folder);
    }
    if !metadata.is_file() {
        return Err(SourceReject::UnsupportedType);
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "zip" => Ok(SourceKind::Zip),
        "7z" => Ok(SourceKind::SevenZip),
        "rar" => Ok(SourceKind::Rar),
        _ => Err(SourceReject::UnsupportedType),
    }
}

/// The folder is Paks, lies below it or contains it; judged lexically and on resolved paths
/// (component-wise, ASCII case folded on Windows).
fn overlaps(folder: &Path, paks: &Path) -> bool {
    let pairs = [
        (Some(folder.to_path_buf()), Some(paks.to_path_buf())),
        (resolved(folder), resolved(paks)),
    ];
    pairs.iter().any(|pair| match pair {
        (Some(folder), Some(paks)) => is_within(folder, paks) || is_within(paks, folder),
        _ => false,
    })
}

fn resolved(path: &Path) -> Option<PathBuf> {
    resolve_existing(path)
}

/// The display name of a source and its stem (archive file name without its extension).
pub(crate) fn source_names(path: &Path, folder: bool) -> (String, String) {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    if folder {
        return (name.clone(), name);
    }
    let stem = match name.rfind('.') {
        Some(dot) if dot > 0 => name.get(..dot).unwrap_or_default().to_owned(),
        _ => name.clone(),
    };
    (name, stem)
}
