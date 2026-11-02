//! Single files in RemMods' own folders (the graphics marker) and a read-only look at other
//! sessions.

use std::fs;
use std::io::{self, ErrorKind};
use std::path::Path;

use super::FsError;
use super::scope::ScopeViolation;
use super::sessions::{self, Liveness, SESSIONS_DIR};
use crate::platform;

const LOCK_SUFFIX: &str = ".lock";

/// Removes one regular file that is a direct child of one of `app_dirs` (not a link, not a folder,
/// not deeper). Ok(false) when it does not exist.
pub fn remove_app_file(path: &Path, app_dirs: &[&Path]) -> Result<bool, FsError> {
    let direct_child = path
        .parent()
        .is_some_and(|parent| app_dirs.contains(&parent));
    if !direct_child || path.file_name().is_none() {
        return Err(FsError::OutOfScope {
            path: path.to_path_buf(),
            reason: ScopeViolation::OutsideRoots,
        });
    }
    let raw =
        platform::fs_path(path).map_err(|error| FsError::io("remove_app_file", path, error))?;
    match fs::symlink_metadata(&raw) {
        Ok(meta) if meta.file_type().is_file() => {}
        Ok(_) => {
            return Err(FsError::io(
                "remove_app_file",
                path,
                io::Error::new(ErrorKind::InvalidInput, "not a regular file"),
            ));
        }
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(FsError::io("remove_app_file", path, error)),
    }
    match fs::remove_file(&raw) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(FsError::io("remove_app_file", path, error)),
    }
}

/// True when another process holds its session lock in `<data_dir>/sessions`. Read-only: a dead
/// session's lock is released at once and its file stays. No sessions folder → false.
pub fn live_session_exists(data_dir: &Path) -> bool {
    let dir = data_dir.join(SESSIONS_DIR);
    let entries = match platform::fs_path(&dir).and_then(fs::read_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return false,
        Err(error) => {
            log::warn!("cannot list {}: {error}", dir.display());
            return false;
        }
    };
    entries.filter_map(Result::ok).any(|entry| {
        let name = entry.file_name();
        let Some(session) = name
            .to_str()
            .and_then(|name| name.strip_suffix(LOCK_SUFFIX))
        else {
            return false;
        };
        sessions::is_session_id(session)
            && matches!(sessions::probe(&dir, session), Liveness::Alive)
    })
}
