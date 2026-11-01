//! The one rename primitive: scope-checked, never replaces, with the fallbacks for file systems
//! that do not know RENAME_NOREPLACE.

use std::fs;
use std::path::Path;

use super::FsError;
use super::journal::Journal;
use super::scope::Scope;
use crate::platform::{self, NoReplace};

/// Marks an unlink failure whose cleanup failed too: two names for one inode stay behind.
pub(crate) const STUCK_UNLINK_OP: &str = "unlink_stuck";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenameMethod {
    Atomic,
    LinkUnlink,
    Placeholder,
}

/// Scope-checked rename that never replaces. `journal` (entry id) records a placeholder folder
/// before it is created, so recovery can tell it from a mod folder.
pub(crate) fn rename_one(
    scope: &Scope,
    from: &Path,
    to: &Path,
    is_dir: bool,
    journal: Option<(&Journal, &str)>,
) -> Result<RenameMethod, FsError> {
    scope.check(from)?;
    scope.check(to)?;
    let raw_from = platform::fs_path(from).map_err(|error| FsError::io("rename", from, error))?;
    let raw_to = platform::fs_path(to).map_err(|error| FsError::io("rename", to, error))?;
    match platform::rename_noreplace(&raw_from, &raw_to) {
        Ok(NoReplace::Done) => Ok(RenameMethod::Atomic),
        Ok(NoReplace::FlagUnsupported) if is_dir => {
            if let Some((journal, entry)) = journal {
                journal.append_placeholder(entry, to)?;
            }
            placeholder_rename(&raw_from, &raw_to, from)?;
            Ok(RenameMethod::Placeholder)
        }
        Ok(NoReplace::FlagUnsupported) => {
            link_unlink(&raw_from, &raw_to, from)?;
            Ok(RenameMethod::LinkUnlink)
        }
        Err(error) => Err(FsError::io("rename", from, error)),
    }
}

/// The file fallback, forced (tests). Scope-checked like every other mutator.
#[doc(hidden)]
pub fn link_unlink_noreplace(scope: &Scope, from: &Path, to: &Path) -> Result<(), FsError> {
    scope.check(from)?;
    scope.check(to)?;
    let raw_from = platform::fs_path(from).map_err(|error| FsError::io("link", from, error))?;
    let raw_to = platform::fs_path(to).map_err(|error| FsError::io("link", to, error))?;
    link_unlink(&raw_from, &raw_to, from)
}

/// The folder fallback, forced (tests). Scope-checked like every other mutator.
#[doc(hidden)]
pub fn placeholder_noreplace(scope: &Scope, from: &Path, to: &Path) -> Result<(), FsError> {
    scope.check(from)?;
    scope.check(to)?;
    let raw_from =
        platform::fs_path(from).map_err(|error| FsError::io("placeholder", from, error))?;
    let raw_to = platform::fs_path(to).map_err(|error| FsError::io("placeholder", to, error))?;
    placeholder_rename(&raw_from, &raw_to, from)
}

/// hard_link fails when `to` exists. A crash between link and unlink leaves two names for one
/// inode, never an empty live `.pak`.
fn link_unlink(from: &Path, to: &Path, shown: &Path) -> Result<(), FsError> {
    fs::hard_link(from, to).map_err(|error| FsError::io("link", shown, error))?;
    let Err(unlink_error) = fs::remove_file(from) else {
        return Ok(());
    };
    match fs::remove_file(to) {
        Ok(()) => Err(FsError::io("unlink", shown, unlink_error)),
        Err(cleanup_error) => {
            log::error!(
                "unlink of {} failed ({unlink_error}) and removing the new link {} failed too: \
                 {cleanup_error}",
                from.display(),
                to.display()
            );
            Err(FsError::io(STUCK_UNLINK_OP, shown, unlink_error))
        }
    }
}

/// create_dir is exclusive, so an existing `to` fails; rename then replaces only the empty
/// placeholder this call just made.
fn placeholder_rename(from: &Path, to: &Path, shown: &Path) -> Result<(), FsError> {
    fs::create_dir(to).map_err(|error| FsError::io("placeholder", shown, error))?;
    if let Err(error) = fs::rename(from, to) {
        if let Err(cleanup) = fs::remove_dir(to) {
            log::warn!("removing placeholder {} failed: {cleanup}", to.display());
        }
        return Err(FsError::io("rename", shown, error));
    }
    Ok(())
}
