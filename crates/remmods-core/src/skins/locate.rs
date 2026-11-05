//! Finding the game's skin files. Reads metadata and whole files, nothing inside them.

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::error::SkinsError;
use crate::platform;
use crate::safe_fs::{SKIN_SAVE_FILES, SkinSaves};

/// The real files are about 10 MB; this only keeps a wrong file from filling the memory.
const MAX_SAVE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SaveFolder {
    /// The folder's name, a Steam ID.
    pub account: String,
    /// Fs form.
    pub path: PathBuf,
}

fn is_account(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_digit())
}

/// The newest time among the five files; `None` unless all five are there.
fn last_change(folder: &Path) -> Option<SystemTime> {
    SKIN_SAVE_FILES
        .iter()
        .map(|name| {
            let meta = fs::metadata(folder.join(name)).ok()?;
            meta.is_file().then(|| meta.modified().ok()).flatten()
        })
        .try_fold(SystemTime::UNIX_EPOCH, |newest, time| {
            Some(newest.max(time?))
        })
}

/// The account with all five files whose files changed last: the one in use.
pub(crate) fn newest_account(root: &Path) -> Option<SaveFolder> {
    let raw = platform::fs_path(root).ok()?;
    fs::read_dir(raw)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let account = entry
                .file_name()
                .to_str()
                .filter(|name| is_account(name))?
                .to_owned();
            let path = entry.path();
            let changed = last_change(&path)?;
            Some((changed, SaveFolder { account, path }))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, folder)| folder)
}

fn read_capped(path: &Path) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(MAX_SAVE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len()).is_ok_and(|len| len > MAX_SAVE_BYTES) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "file too large"));
    }
    Ok(bytes)
}

/// The five files of `folder`, in order. A missing or unreadable one is `None`.
pub(crate) fn read_five(folder: &Path) -> Option<SkinSaves> {
    let mut saves = SkinSaves::default();
    for (name, slot) in SKIN_SAVE_FILES.iter().zip(&mut saves) {
        *slot = read_capped(&folder.join(name)).ok()?;
    }
    Some(saves)
}

pub(crate) fn read_game_saves(folder: &SaveFolder) -> Result<SkinSaves, SkinsError> {
    read_five(&folder.path).ok_or(SkinsError::NoSaves)
}
