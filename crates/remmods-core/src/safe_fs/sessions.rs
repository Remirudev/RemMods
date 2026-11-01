//! Per-process session lock files (`data_dir/sessions/<id>.lock`). A session is alive while its
//! process holds the exclusive lock on its file; a missing or lockable file means dead. A file is
//! only removed by whoever holds its lock, so a live session's file never vanishes.

use std::collections::HashSet;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io;
use std::path::{Path, PathBuf};

use super::FsError;
use crate::platform;

pub(crate) const SESSIONS_DIR: &str = "sessions";
const LOCK_SUFFIX: &str = ".lock";
const SESSION_ID_LEN: usize = 32;
const CREATE_ATTEMPTS: u32 = 3;

pub(crate) enum Liveness {
    Alive,
    /// Dead; holds the session file's lock while the file still exists.
    Dead(Option<File>),
}

pub(crate) fn new_session_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// Journal content is data: only ids RemMods itself generates may become file names.
pub(crate) fn is_session_id(text: &str) -> bool {
    text.len() == SESSION_ID_LEN
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Creates and locks this process's session file. A cleaner may remove a fresh file between
/// create and lock; then the file id read after locking differs or is gone, and a new id is used.
pub(crate) fn create_own(dir: &Path) -> Result<(String, File), FsError> {
    for _ in 0..CREATE_ATTEMPTS {
        let id = new_session_id();
        let path = lock_path(dir, &id);
        let file = platform::fs_path(&path)
            .and_then(|raw| {
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create_new(true)
                    .open(raw)
            })
            .map_err(|error| FsError::io("session", &path, error))?;
        let created = platform::file_id(&path).ok();
        match file.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => continue,
            Err(TryLockError::Error(error)) => return Err(FsError::io("session", &path, error)),
        }
        let locked = platform::file_id(&path).ok();
        if created.is_some() && created == locked {
            return Ok((id, file));
        }
    }
    Err(FsError::io(
        "session",
        dir,
        io::Error::other("the session lock file vanished three times"),
    ))
}

pub(crate) fn probe(dir: &Path, session: &str) -> Liveness {
    if !is_session_id(session) {
        return Liveness::Dead(None);
    }
    let path = lock_path(dir, session);
    let file = match platform::fs_path(&path).and_then(File::open) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Liveness::Dead(None),
        Err(error) => {
            log::warn!("cannot open session file {}: {error}", path.display());
            return Liveness::Alive;
        }
    };
    match file.try_lock() {
        Ok(()) => Liveness::Dead(Some(file)),
        Err(_) => Liveness::Alive,
    }
}

/// Removes a dead session's file while holding its lock (`lock` from [`probe`]).
pub(crate) fn remove_dead(dir: &Path, session: &str, lock: Option<File>) {
    let Some(lock) = lock else {
        return;
    };
    let path = lock_path(dir, session);
    if let Err(error) = platform::fs_path(&path).and_then(fs::remove_file) {
        log::warn!("removing session file {} failed: {error}", path.display());
    }
    drop(lock);
}

/// Removes the files of dead sessions that own no journal entry.
pub(crate) fn clean_dead(dir: &Path, own: &str, owners: &HashSet<String>) {
    let entries = match platform::fs_path(dir).and_then(fs::read_dir) {
        Ok(entries) => entries,
        Err(error) => {
            log::warn!("cannot list {}: {error}", dir.display());
            return;
        }
    };
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        let Some(session) = name
            .to_str()
            .and_then(|name| name.strip_suffix(LOCK_SUFFIX))
        else {
            continue;
        };
        if session == own || !is_session_id(session) || owners.contains(session) {
            continue;
        }
        if let Liveness::Dead(lock) = probe(dir, session) {
            log::debug!("removing dead session file {session}");
            remove_dead(dir, session, lock);
        }
    }
}

fn lock_path(dir: &Path, session: &str) -> PathBuf {
    dir.join(format!("{session}{LOCK_SUFFIX}"))
}
