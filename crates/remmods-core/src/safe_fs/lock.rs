use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};

use super::{FsError, ensure_dir};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockMode {
    Exclusive,
    Shared,
}

/// Cross-process lock on `remmods.lock` (flock on Linux, LockFileEx on Windows).
/// Released when the guard is dropped.
#[derive(Debug)]
pub struct AppLock {
    file: File,
    path: PathBuf,
    mode: LockMode,
}

impl AppLock {
    pub fn try_exclusive(path: &Path) -> Result<Self, FsError> {
        Self::try_acquire(path, LockMode::Exclusive)
    }

    pub fn try_shared(path: &Path) -> Result<Self, FsError> {
        Self::try_acquire(path, LockMode::Shared)
    }

    pub fn mode(&self) -> LockMode {
        self.mode
    }

    fn try_acquire(path: &Path, mode: LockMode) -> Result<Self, FsError> {
        let file = open_lock_file(path)?;
        let attempt = match mode {
            LockMode::Exclusive => file.try_lock(),
            LockMode::Shared => file.try_lock_shared(),
        };
        match attempt {
            Ok(()) => Ok(Self {
                file,
                path: path.to_path_buf(),
                mode,
            }),
            Err(TryLockError::WouldBlock) => Err(FsError::Busy),
            Err(TryLockError::Error(source)) => Err(FsError::io("lock", path, source)),
        }
    }
}

impl Drop for AppLock {
    fn drop(&mut self) {
        if let Err(error) = self.file.unlock() {
            log::warn!("unlock {} failed: {error}", self.path.display());
        }
    }
}

fn open_lock_file(path: &Path) -> Result<File, FsError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        ensure_dir(parent)?;
    }
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|source| FsError::io("open_lock", path, source))
}
