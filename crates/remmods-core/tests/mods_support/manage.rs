//! Builders for the P3 manage tests (spec §8.2): fixtures with a TempDir trash, targets for rows
//! and entries, and the "unchanged" listing.

use std::path::{Path, PathBuf};

use remmods_core::mods::{ModTarget, ModsError, ScanContent};
use remmods_core::{CoreError, CoreOptions};

pub use super::toggle::{Extras, ToggleFixture};

/// A set-aside suffix (32 lowercase hex).
pub const HEX: &str = "0123456789abcdef0123456789abcdef";

/// A fixture whose cores trash into `<root>/xdg-data/Trash` (never a real trash).
pub fn core_with_trash(entries: &[(&str, &[u8])]) -> ToggleFixture {
    ToggleFixture::build(entries, |root| Extras {
        trash: Some(crate::support::trash_env(root)),
        ..Extras::default()
    })
}

/// A fixture with `CoreOptions::default()`'s trash env, which refuses every trash.
pub fn core_without_trash(entries: &[(&str, &[u8])]) -> ToggleFixture {
    ToggleFixture::build(entries, |_| Extras {
        trash: Some(CoreOptions::default().trash),
        ..Extras::default()
    })
}

pub fn trash_dir(fixture: &ToggleFixture) -> PathBuf {
    fixture.root.join("xdg-data").join("Trash")
}

/// Paks and the trash folder, byte for byte.
pub fn tree(fixture: &ToggleFixture) -> Vec<String> {
    let mut listing = crate::support::tree_listing(&fixture.paks);
    let trash = trash_dir(fixture);
    if trash.exists() {
        listing.extend(
            crate::support::tree_listing(&trash)
                .into_iter()
                .map(|line| format!("trash {line}")),
        );
    }
    listing
}

/// The target of an archive, leftover or unmanaged entry by its rel path.
pub fn entry_target(content: &ScanContent, rel_path: &str) -> ModTarget {
    let mod_ref = content
        .archives
        .iter()
        .map(|entry| (&entry.mod_ref, &entry.rel_path))
        .chain(
            content
                .leftovers
                .iter()
                .map(|entry| (&entry.mod_ref, &entry.rel_path)),
        )
        .chain(
            content
                .unmanaged
                .iter()
                .map(|entry| (&entry.mod_ref, &entry.rel_path)),
        )
        .find(|(_, rel)| rel.as_str() == rel_path)
        .map(|(mod_ref, _)| mod_ref.clone())
        .unwrap_or_else(|| panic!("no entry {rel_path}"));
    ModTarget {
        mod_ref,
        key: rel_path.to_owned(),
    }
}

pub fn path_of(root: &Path, rel: &str) -> PathBuf {
    crate::support::join_rel(root, rel)
}

/// The `ModsError` inside a core error; panics on anything else.
pub fn mods_error<T: std::fmt::Debug>(result: Result<T, CoreError>) -> ModsError {
    match result {
        Err(CoreError::Mods(error)) => error,
        other => panic!("expected a ModsError, got {other:?}"),
    }
}

/// Simulates an app restart: the old core (and its session) goes away, a new one opens and takes
/// one write guard, so journal recovery runs for the dead session.
pub fn restart(fixture: &mut ToggleFixture) {
    let fresh = fixture.second_core();
    drop(std::mem::replace(&mut fixture.core, fresh));
    let app_dirs = [
        fixture.paths.data_dir.as_path(),
        fixture.paths.config_dir.as_path(),
    ];
    let scope = remmods_core::safe_fs::Scope::for_paks(&fixture.paks, &app_dirs).expect("scope");
    drop(fixture.core.ops.write(scope).expect("write guard"));
}
