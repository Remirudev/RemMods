//! Removals for recovery: a link-fallback leftover and an empty placeholder folder.

use std::fs;
use std::path::Path;

use super::FsError;
use super::scope::Scope;
use crate::platform;

pub(crate) fn remove_file_in(scope: &Scope, path: &Path) -> Result<(), FsError> {
    scope.check(path)?;
    platform::fs_path(path)
        .and_then(fs::remove_file)
        .map_err(|error| FsError::io("remove_file", path, error))
}

/// remove_dir only succeeds on an empty folder, so a mod folder is never removed here.
pub(crate) fn remove_empty_dir_in(scope: &Scope, path: &Path) -> Result<(), FsError> {
    scope.check(path)?;
    platform::fs_path(path)
        .and_then(fs::remove_dir)
        .map_err(|error| FsError::io("remove_dir", path, error))
}
