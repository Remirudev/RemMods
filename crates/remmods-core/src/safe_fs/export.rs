//! Profile export: the one write outside the mod roots and app dirs, to a file the user picked in
//! the native save dialog.

use std::path::Path;

use super::gate::WriteGuard;
use super::scope::{ScopeViolation, has_dot_component, is_within, resolve_existing};
use super::{FsError, write_atomic};

/// RemMods' own store and the original app's file share this name; an export never replaces it.
const PROFILES_FILE: &str = "profiles.json";
const EXPORT_EXT: &str = "json";

impl WriteGuard<'_> {
    /// Refuses with OutOfScope, nothing written, unless all hold: absolute, no `.`/`..`; a
    /// `.json` name other than `profiles.json` (ASCII case-insensitive); not inside a mod root,
    /// the app dirs or any of `forbidden`. Then `write_atomic` (the save dialog already asked
    /// before replacing an existing file).
    pub fn write_export(
        &self,
        path: &Path,
        bytes: &[u8],
        forbidden: &[&Path],
    ) -> Result<(), FsError> {
        let refuse = |reason| FsError::OutOfScope {
            path: path.to_path_buf(),
            reason,
        };
        if !path.is_absolute() {
            return Err(refuse(ScopeViolation::NotAbsolute));
        }
        if has_dot_component(path) {
            return Err(refuse(ScopeViolation::DotComponent));
        }
        if !is_allowed_name(path) || self.scope().contains(path) || in_forbidden(path, forbidden) {
            return Err(refuse(ScopeViolation::OutsideRoots));
        }
        write_atomic(path, bytes)
    }
}

fn is_allowed_name(path: &Path) -> bool {
    let is_json = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case(EXPORT_EXT));
    let is_store = path
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case(PROFILES_FILE));
    is_json && !is_store
}

fn in_forbidden(path: &Path, forbidden: &[&Path]) -> bool {
    let resolved = resolve_existing(path);
    forbidden.iter().any(|dir| {
        is_within(path, dir)
            || resolved.as_deref().is_some_and(|resolved| {
                resolve_existing(dir).is_some_and(|dir| is_within(resolved, &dir))
            })
    })
}
