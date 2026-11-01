//! What a TEKKEN 8 folder looks like on disk, and how a picked folder leads to its root. Every
//! path returned here is plain and absolute with the on-disk spelling of each child (§4.2); the
//! verbatim form from `platform::fs_path` is used only for the std::fs call itself.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Component, Path, PathBuf};

use super::{FolderProblem, MANIFEST_FILE};
use crate::paths::display_path;
use crate::platform::{self, OsErrorClass, OsKind};

pub const MAX_PICK_LEVELS: usize = 8;

const POLARIS: &str = "Polaris";
const CONTENT: &str = "Content";
const PAKS: &str = "Paks";
const BINARIES: &str = "Binaries";
const WIN64: &str = "Win64";
const SHIPPING_EXE: &str = "Polaris-Win64-Shipping.exe";
const COMMON: &str = "common";
const STEAMAPPS: &str = "steamapps";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameLayout {
    pub root: PathBuf,
    pub paks: PathBuf,
    pub exe: PathBuf,
}

/// `appmanifest_1778820.acf` next to the `common` folder that holds the game, and the library
/// folder (parent of `steamapps`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestLocation {
    pub manifest: PathBuf,
    pub library: PathBuf,
}

/// Checks `dir` for `Polaris/Content/Paks` and `Polaris/Binaries/Win64/Polaris-Win64-Shipping.exe`
/// (child names matched ASCII-case-insensitively). `TEKKEN 8.exe` is not required.
pub fn validate_game_root(dir: &Path) -> Result<GameLayout, FolderProblem> {
    let root = plain_absolute(dir)?;
    if root.to_str().is_none() {
        return Err(FolderProblem::NotUnicode);
    }
    require_kind(&root, Kind::Dir)?;
    let polaris = child_of_kind(&root, POLARIS, Kind::Dir)?;
    let content = child_of_kind(&polaris, CONTENT, Kind::Dir)?;
    let paks = child_of_kind(&content, PAKS, Kind::Dir)?;
    let binaries = child_of_kind(&polaris, BINARIES, Kind::Dir)?;
    let win64 = child_of_kind(&binaries, WIN64, Kind::Dir)?;
    let exe = child_of_kind(&win64, SHIPPING_EXE, Kind::File)?;
    Ok(GameLayout { root, paks, exe })
}

/// The game root for any folder or file inside it: the pick itself (its parent for a file) and up
/// to seven parents are tried; an ambiguous folder stops the walk.
pub fn resolve_pick(picked: &Path) -> Result<GameLayout, FolderProblem> {
    let picked = plain_absolute(picked)?;
    let start = if is_dir(&picked) {
        picked.as_path()
    } else {
        picked.parent().ok_or(FolderProblem::NotFound)?
    };
    for folder in start.ancestors().take(MAX_PICK_LEVELS) {
        match validate_game_root(folder) {
            Ok(layout) => return Ok(layout),
            Err(FolderProblem::Ambiguous) => return Err(FolderProblem::Ambiguous),
            Err(_) => {}
        }
    }
    Err(FolderProblem::NotFound)
}

/// For `<library>/steamapps/common/<root>`: the app manifest in `steamapps`, if present.
pub fn manifest_for_root(root: &Path) -> Option<ManifestLocation> {
    let common = root.parent()?;
    let steamapps = common.parent()?;
    let named = |path: &Path, name: &str| {
        path.file_name()
            .is_some_and(|own| own.as_encoded_bytes().eq_ignore_ascii_case(name.as_bytes()))
    };
    if !named(common, COMMON) || !named(steamapps, STEAMAPPS) {
        return None;
    }
    let manifest = lookup_child(steamapps, OsStr::new(MANIFEST_FILE))
        .ok()
        .flatten()?;
    Some(ManifestLocation {
        manifest: steamapps.join(manifest),
        library: steamapps.parent()?.to_path_buf(),
    })
}

/// The Paks path as the game sees it. Linux (Proton): `S:\` is the library, `Z:` the host root.
pub fn game_view_of_paks(layout: &GameLayout, library: Option<&Path>, os: OsKind) -> String {
    match os {
        OsKind::Linux => {
            let below_library = library.and_then(|library| layout.paks.strip_prefix(library).ok());
            match below_library {
                Some(relative) => format!("S:\\{}", backslash_joined(relative)),
                None => format!("Z:\\{}", backslash_joined(&layout.paks)),
            }
        }
        OsKind::Windows | OsKind::Other => display_path(&layout.paks),
    }
}

/// Best effort: each component re-spelled as on disk (exact name first, else the single
/// ASCII-case-insensitive match); a failed or ambiguous lookup keeps the component as given. A
/// drive letter is upper-cased.
pub fn with_disk_casing(path: &Path) -> PathBuf {
    let mut cased = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => cased.push(upper_drive_letter(prefix.as_os_str())),
            Component::Normal(name) => {
                let on_disk = lookup_child(&cased, name).ok().flatten();
                cased.push(on_disk.as_deref().unwrap_or(name));
            }
            other => cased.push(other.as_os_str()),
        }
    }
    cased
}

/// Looks `name` up in `dir`: the exact name wins; else the single ASCII-case-insensitive match;
/// several such matches without an exact one are `Ambiguous`. `Ok(None)` = no such entry.
pub(crate) fn lookup_child(dir: &Path, name: &OsStr) -> Result<Option<OsString>, FolderProblem> {
    let target = platform::fs_path(dir).map_err(|_| FolderProblem::NotFound)?;
    let entries = fs::read_dir(target).map_err(|error| problem_of(&error))?;
    let wanted = name.as_encoded_bytes();
    let mut folded = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| problem_of(&error))?;
        let own = entry.file_name();
        if own == name {
            return Ok(Some(own));
        }
        if own.as_encoded_bytes().eq_ignore_ascii_case(wanted) {
            folded.push(own);
        }
    }
    match folded.len() {
        0 => Ok(None),
        1 => Ok(folded.pop()),
        _ => Err(FolderProblem::Ambiguous),
    }
}

/// `dir/<name>` with the on-disk spelling, if that entry is a directory.
pub(crate) fn child_dir(dir: &Path, name: &str) -> Result<PathBuf, FolderProblem> {
    child_of_kind(dir, name, Kind::Dir)
}

/// `std::path::absolute` with `.`, `..` and trailing separators resolved lexically (no symlink
/// resolution), as Windows' GetFullPathNameW does, so both OSes store the same form.
pub(crate) fn plain_absolute(path: &Path) -> Result<PathBuf, FolderProblem> {
    let absolute = std::path::absolute(path).map_err(|_| FolderProblem::NotFound)?;
    let mut plain = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                plain.pop();
            }
            other => plain.push(other.as_os_str()),
        }
    }
    Ok(plain)
}

pub(crate) fn is_dir(path: &Path) -> bool {
    require_kind(path, Kind::Dir).is_ok()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Dir,
    File,
}

fn child_of_kind(dir: &Path, name: &str, kind: Kind) -> Result<PathBuf, FolderProblem> {
    let on_disk = lookup_child(dir, OsStr::new(name))?.ok_or(FolderProblem::NotFound)?;
    let child = dir.join(on_disk);
    require_kind(&child, kind)?;
    Ok(child)
}

/// Follows links: a link to the exe counts as the exe.
fn require_kind(path: &Path, kind: Kind) -> Result<(), FolderProblem> {
    let target = platform::fs_path(path).map_err(|_| FolderProblem::NotFound)?;
    let metadata = fs::metadata(target).map_err(|error| problem_of(&error))?;
    let fits = match kind {
        Kind::Dir => metadata.is_dir(),
        Kind::File => metadata.is_file(),
    };
    if fits {
        Ok(())
    } else {
        Err(FolderProblem::NotFound)
    }
}

fn problem_of(error: &std::io::Error) -> FolderProblem {
    match platform::classify(error) {
        OsErrorClass::NotFound => FolderProblem::NotFound,
        _ => FolderProblem::Unreadable,
    }
}

fn backslash_joined(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\\")
}

fn upper_drive_letter(prefix: &OsStr) -> OsString {
    let Some(text) = prefix.to_str() else {
        return prefix.to_os_string();
    };
    let bytes = text.as_bytes();
    match bytes {
        [.., letter, b':'] if letter.is_ascii_alphabetic() => {
            let split = bytes.len() - 2;
            let mut upper = text[..split].to_owned();
            upper.push(char::from(letter.to_ascii_uppercase()));
            upper.push(':');
            OsString::from(upper)
        }
        _ => prefix.to_os_string(),
    }
}
