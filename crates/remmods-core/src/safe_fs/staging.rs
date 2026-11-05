//! Install staging (PLAN §4 Install): a hidden `_install_tmp_<32 hex>` folder directly in a mod
//! root, journaled before it is created, filled with inert files only, then renamed into place by
//! the commit batch or removed again. Also the creation of a missing mod root.

use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::SystemTime;

use super::FsError;
use super::aside::remove_inert_temp_tree;
use super::gate::WriteGuard;
use super::names::validate_name;
use super::scope::{MOD_ROOT_NAMES, ScopeViolation, is_link, is_within};
use crate::platform;

const STAGING_PREFIX: &str = "_install_tmp_";
const STAGING_LABEL: &str = "install-stage";
const SUFFIX_LEN: usize = 32;

/// A journaled staging folder of this session. Dropped without `finish_staging` or
/// `discard_staging` it stays journaled until the session ends; recovery removes it then.
#[derive(Debug)]
pub struct StagingDir {
    path: PathBuf,
    entry: String,
    /// Folders inside the staging folder that this value created.
    created: Mutex<HashSet<PathBuf>>,
    settled: bool,
}

/// One file being written into a staging folder.
#[derive(Debug)]
pub struct StagedFile {
    file: File,
    path: PathBuf,
}

impl WriteGuard<'_> {
    /// Creates `<paks>/<name>` (non-recursive) with `name` exactly `Mods`, `~mods` or `LogicMods`
    /// and adds it to this guard's scope. Refuses (`OutOfScope`) without `paks`, for any other
    /// name, when an ASCII-case-insensitive sibling exists (the caller uses that root instead),
    /// or when `paks` is a link. Returns the plain path.
    pub fn create_mod_root(&mut self, name: &str) -> Result<PathBuf, FsError> {
        let Some(paks) = self.scope().paks().map(Path::to_path_buf) else {
            return Err(out_of_scope(Path::new(name), ScopeViolation::OutsideRoots));
        };
        let root = paks.join(name);
        if !MOD_ROOT_NAMES.contains(&name) {
            return Err(out_of_scope(&root, ScopeViolation::OutsideRoots));
        }
        if is_link(&paks) {
            return Err(out_of_scope(&root, ScopeViolation::LinkComponent));
        }
        let entries = platform::fs_path(&paks)
            .and_then(fs::read_dir)
            .map_err(|error| FsError::io("create_mod_root", &paks, error))?;
        for entry in entries {
            let entry = entry.map_err(|error| FsError::io("create_mod_root", &paks, error))?;
            let sibling = entry
                .file_name()
                .to_str()
                .is_some_and(|other| other.eq_ignore_ascii_case(name));
            if sibling {
                return Err(out_of_scope(&root, ScopeViolation::IsRoot));
            }
        }
        platform::fs_path(&root)
            .and_then(fs::create_dir)
            .map_err(|error| FsError::io("create_mod_root", &root, error))?;
        if !self.scope_mut().add_mod_root(&root) {
            return Err(out_of_scope(&root, ScopeViolation::OutsideRoots));
        }
        Ok(root)
    }

    /// Journals `<root>/_install_tmp_<suffix>` in a new entry (label `install-stage`) first, then
    /// creates it (exclusive `create_dir`); a failed `create_dir` drops the entry again, so no
    /// staging folder ever exists unjournaled. `root` must be a mod root of the scope.
    pub fn create_staging(&self, root: &Path, suffix: &str) -> Result<StagingDir, FsError> {
        let path = root.join(format!("{STAGING_PREFIX}{suffix}"));
        let suffix_ok = suffix.len() == SUFFIX_LEN
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        if !suffix_ok || !self.scope().is_mod_root(root) {
            return Err(out_of_scope(&path, ScopeViolation::OutsideRoots));
        }
        self.scope().check(&path)?;
        let entry = self.journal().begin_staging(STAGING_LABEL, &path)?;
        let created = platform::fs_path(&path).and_then(fs::create_dir);
        if let Err(error) = created {
            if let Err(drop_error) = self.journal().remove(&entry) {
                log::error!("dropping the staging entry {entry} failed: {drop_error}");
            }
            return Err(FsError::io("create_staging", &path, error));
        }
        Ok(StagingDir {
            path,
            entry,
            created: Mutex::new(HashSet::new()),
            settled: false,
        })
    }

    /// Records the files staging made inert (paths inside the staging folder) in its entry.
    pub fn record_inert(&self, staging: &StagingDir, files: &[PathBuf]) -> Result<(), FsError> {
        if let Some(outside) = files.iter().find(|file| !is_within(file, &staging.path)) {
            return Err(out_of_scope(outside, ScopeViolation::OutsideRoots));
        }
        self.journal().add_inert(&staging.entry, files)
    }

    /// After a successful commit: the staging folder must no longer exist; drops its entry.
    pub fn finish_staging(&self, mut staging: StagingDir) -> Result<(), FsError> {
        if exists(&staging.path) {
            return Err(FsError::InvalidBatch {
                reason: "staging_left",
            });
        }
        staging.settled = true;
        self.journal().remove(&staging.entry)
    }

    /// On error or cancel: drops the entry, then removes the folder tree with the `remove_aside`
    /// checks. A refused or failed removal leaves an inert, unjournaled leftover for Repair.
    /// While another journal entry (a commit batch kept after a crash or a stuck rollback) names
    /// the folder, nothing is touched: recovery rolls that batch back first.
    pub fn discard_staging(&self, mut staging: StagingDir) -> Result<(), FsError> {
        if self.named_by_other_entry(&staging.path, &staging.entry)? {
            staging.settled = true;
            return Err(FsError::AsideJournaled {
                path: staging.path.clone(),
            });
        }
        staging.settled = true;
        self.journal().remove(&staging.entry)?;
        if !exists(&staging.path) {
            return Ok(());
        }
        remove_inert_temp_tree(self.scope(), &staging.path)
    }

    /// True when a journal record (any entry, any op kind) names `path` or something below it.
    /// An undecodable record path counts as naming it: refusing is the safe side.
    pub fn is_journaled(&self, path: &Path) -> Result<bool, FsError> {
        self.named_by_other_entry(path, "")
    }

    fn named_by_other_entry(&self, path: &Path, skip: &str) -> Result<bool, FsError> {
        let entries = self.journal().entries()?;
        let named = entries
            .iter()
            .filter(|entry| entry.id != skip)
            .flat_map(|entry| &entry.ops)
            .flat_map(|op| op.paths())
            .any(|record| record.decode().is_none_or(|named| is_within(&named, path)));
        Ok(named)
    }
}

impl StagingDir {
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Creates parents with exclusive `create_dir` (an existing parent must be a real folder this
    /// value created), then the file with `create_new`. Each component must pass
    /// `names::validate_name`. Needs no guard: the folder is journaled to this live session.
    pub fn create_file(&self, rel: &[String]) -> Result<StagedFile, FsError> {
        let Some((file_name, parents)) = rel.split_last() else {
            return Err(out_of_scope(&self.path, ScopeViolation::DotComponent));
        };
        let mut path = self.path.clone();
        for part in rel {
            if validate_name(part).is_err() {
                return Err(out_of_scope(&path.join(part), ScopeViolation::DotComponent));
            }
        }
        for part in parents {
            path.push(part);
            self.ensure_parent(&path)?;
        }
        path.push(file_name);
        let raw =
            platform::fs_path(&path).map_err(|error| FsError::io("stage_file", &path, error))?;
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(raw)
            .map_err(|error| FsError::io("stage_file", &path, error))?;
        Ok(StagedFile { file, path })
    }

    fn ensure_parent(&self, path: &Path) -> Result<(), FsError> {
        let mut created = self.created.lock().unwrap_or_else(PoisonError::into_inner);
        if created.contains(path) {
            let real_dir = platform::fs_path(path)
                .and_then(fs::symlink_metadata)
                .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink());
            if real_dir {
                return Ok(());
            }
            return Err(out_of_scope(path, ScopeViolation::LinkComponent));
        }
        platform::fs_path(path)
            .and_then(fs::create_dir)
            .map_err(|error| FsError::io("stage_dir", path, error))?;
        created.insert(path.to_path_buf());
        Ok(())
    }
}

impl Drop for StagingDir {
    fn drop(&mut self) {
        if !self.settled {
            log::warn!(
                "staging folder {} dropped unsettled; recovery removes it after this session",
                self.path.display()
            );
        }
    }
}

impl StagedFile {
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// `File::set_modified(mtime)` when given (failures logged, not fatal), then `sync_all`.
    pub fn finish(self, mtime: Option<SystemTime>) -> Result<(), FsError> {
        if let Some(mtime) = mtime
            && let Err(error) = self.file.set_modified(mtime)
        {
            log::warn!(
                "setting the mtime of {} failed: {error}",
                self.path.display()
            );
        }
        self.file
            .sync_all()
            .map_err(|error| FsError::io("stage_sync", &self.path, error))
    }
}

impl Write for StagedFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

fn exists(path: &Path) -> bool {
    platform::fs_path(path)
        .and_then(fs::symlink_metadata)
        .is_ok()
}

fn out_of_scope(path: &Path, reason: ScopeViolation) -> FsError {
    FsError::OutOfScope {
        path: path.to_path_buf(),
        reason,
    }
}
