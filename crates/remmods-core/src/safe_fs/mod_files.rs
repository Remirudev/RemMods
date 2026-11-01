//! Writes to files inside one mod folder (`modinfo.json`) and their backups.

use std::path::{Path, PathBuf};

use super::gate::WriteGuard;
use super::scope::ScopeViolation;
use super::{FsError, backup_file, write_atomic};

impl WriteGuard<'_> {
    /// `modinfo.json` writes: in scope, directly inside a mod folder (exactly one level below a
    /// mod root, no link on the way), then `write_atomic`.
    pub fn write_mod_file(&self, path: &Path, bytes: &[u8]) -> Result<(), FsError> {
        self.check_mod_file(path)?;
        write_atomic(path, bytes)
    }

    /// Copies `path` to `<name>.backup.<yyyyMMdd_HHmmss>` next to it, after the same checks.
    pub fn backup_mod_file(&self, path: &Path) -> Result<PathBuf, FsError> {
        self.check_mod_file(path)?;
        backup_file(path)
    }

    /// The scope walk already refuses link components between the root and `path`.
    fn check_mod_file(&self, path: &Path) -> Result<(), FsError> {
        self.scope().check(path)?;
        let in_mod_folder = path
            .parent()
            .and_then(Path::parent)
            .is_some_and(|root| self.scope().is_mod_root(root));
        if in_mod_folder {
            Ok(())
        } else {
            Err(FsError::OutOfScope {
                path: path.to_path_buf(),
                reason: ScopeViolation::OutsideRoots,
            })
        }
    }
}
