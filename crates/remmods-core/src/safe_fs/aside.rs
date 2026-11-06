//! Permanent removal of a set-aside folder or an install staging folder: the only recursive delete
//! below a mod root. A set-aside folder was renamed to an install-temp name with every live
//! payload file disabled first, and staging holds inert files only, so the game never sees a
//! half-removed mod.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};

use super::FsError;
use super::gate::WriteGuard;
use super::names::is_install_temp_name;
use super::scope::Scope;
use crate::platform;

const TEMP_MARKER: &str = "_tmp_";
/// FAT/NTFS/ext4 name limits: 255 UTF-8 bytes and 255 UTF-16 units.
const MAX_NAME: usize = 255;
const LIVE_EXTENSIONS: [&[u8]; 4] = [b"pak", b"ucas", b"utoc", b"sig"];

impl WriteGuard<'_> {
    /// Permanently removes a set-aside folder. Refuses (nothing removed) unless all hold: in scope;
    /// its parent is a mod root or an author folder; an install-temp name; a real folder (no link
    /// or junction); no live `.pak/.ucas/.utoc/.sig` at any depth; no journal record names it or
    /// anything below it.
    pub fn remove_aside(&self, aside: &Path) -> Result<(), FsError> {
        let raw = check_inert_temp_tree(self.scope(), aside)?;
        if self.is_journaled(aside)? {
            return Err(FsError::AsideJournaled {
                path: aside.to_path_buf(),
            });
        }
        remove_tree(&raw, aside)
    }
}

/// The checks of `remove_aside` except the journal, then the removal: the only recursive delete
/// below a mod root (set-aside folders, install staging folders).
pub(crate) fn remove_inert_temp_tree(scope: &Scope, path: &Path) -> Result<(), FsError> {
    let raw = check_inert_temp_tree(scope, path)?;
    remove_tree(&raw, path)
}

/// In scope, directly in a mod root or an author folder, an install-temp name, a real folder, no
/// live payload. Returns the file-system form of `path`.
pub(crate) fn check_inert_temp_tree(scope: &Scope, path: &Path) -> Result<PathBuf, FsError> {
    scope.check(path)?;
    let not_an_aside = || FsError::NotAnAside {
        path: path.to_path_buf(),
    };
    let parent_holds_mods = path.parent().is_some_and(|parent| scope.holds_mods(parent));
    let temp_named = path.file_name().is_some_and(is_install_temp_name);
    if !parent_holds_mods || !temp_named {
        return Err(not_an_aside());
    }
    let raw = platform::fs_path(path).map_err(|error| FsError::io("remove_aside", path, error))?;
    let metadata =
        fs::symlink_metadata(&raw).map_err(|error| FsError::io("remove_aside", path, error))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(not_an_aside());
    }
    if has_live_payload(&raw).map_err(|error| FsError::io("remove_aside", path, error))? {
        return Err(FsError::AsideNotInert {
            path: path.to_path_buf(),
        });
    }
    Ok(raw)
}

/// std's remove_dir_all unlinks links inside instead of following them.
pub(crate) fn remove_tree(raw: &Path, shown: &Path) -> Result<(), FsError> {
    fs::remove_dir_all(raw).map_err(|error| FsError::io("remove_aside", shown, error))
}

/// `"_" + label + "_tmp_" + suffix`, the label cut so the name stays within 255 UTF-8 bytes and
/// 255 UTF-16 units. Always an install-temp name when `suffix` is 32 hex digits. Pure.
pub fn set_aside_name(label: &OsStr, suffix: &str) -> OsString {
    let overhead = 1 + TEMP_MARKER.len() + suffix.len();
    let budget = MAX_NAME.saturating_sub(overhead);
    let label = match label.to_str() {
        Some(text) => OsString::from(cut_at_char(text, budget)),
        None => platform::os_str_prefix(label, budget, budget),
    };
    let mut name = OsString::from("_");
    name.push(label);
    name.push(TEMP_MARKER);
    name.push(suffix);
    name
}

/// UTF-16 units never exceed UTF-8 bytes, so the byte budget covers both limits.
fn cut_at_char(text: &str, max_bytes: usize) -> &str {
    let mut end = text.len().min(max_bytes);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.get(..end).unwrap_or_default()
}

/// A no-follow walk: links are never entered, but a link named like a payload still counts.
pub(crate) fn has_live_payload(dir: &Path) -> std::io::Result<bool> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if is_live_name(&entry.file_name()) {
            return Ok(true);
        }
        let kind = entry.file_type()?;
        if kind.is_dir() && !kind.is_symlink() && has_live_payload(&entry.path())? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn is_live_name(name: &OsStr) -> bool {
    let bytes = name.as_encoded_bytes();
    bytes
        .iter()
        .rposition(|&byte| byte == b'.')
        .and_then(|dot| bytes.get(dot + 1..))
        .is_some_and(|ext| {
            LIVE_EXTENSIONS
                .iter()
                .any(|live| ext.eq_ignore_ascii_case(live))
        })
}
