//! `<data_dir>/skin-sets/<account>`: `sets.json`, one folder per set, `backups/<stamp>` (the game's
//! files before a switch) and `replaced/<set id>/<stamp>` (a set's stored copy from before the
//! game's files took its place). Per set, so a busy set never pushes out another set's copies.

use std::collections::HashSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use chrono::{NaiveDateTime, SecondsFormat, Utc};

use super::error::SkinsError;
use super::locate::read_five;
use super::model::{SKIN_BACKUPS_KEPT, SKIN_SETS_VERSION, SkinSetsFile};
use crate::safe_fs::{
    FsError, SKIN_SAVE_FILES, SkinSaves, WriteGuard, backup_file, remove_app_dir, remove_app_file,
};

pub(crate) const SKIN_SETS_DIR: &str = "skin-sets";
const SETS_FILE: &str = "sets.json";
const BACKUPS_DIR: &str = "backups";
const REPLACED_DIR: &str = "replaced";
const BACKUP_STAMP: &str = "%Y%m%d-%H%M%S-%9f";
const SET_ID_HEX_LEN: usize = 32;
/// Stands in for a set id in the id of a copy of the game's files.
const GAME_COPY: &str = "game";

/// Ids become folder names: a hand-edited list must not lead anywhere else.
pub(crate) fn is_set_id(id: &str) -> bool {
    id.len() == SET_ID_HEX_LEN
        && id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// One complete automatic copy on disk.
#[derive(Clone, Debug)]
pub(crate) struct StoredCopy {
    pub id: String,
    pub taken: String,
    /// The set it was replaced in; none for the game's files from before a switch.
    pub set: Option<String>,
    stamp: String,
    path: PathBuf,
}

#[derive(Debug)]
pub(crate) struct LoadedSets {
    pub file: SkinSetsFile,
    /// A newer file: shown, never written.
    pub read_only: bool,
}

fn parse(bytes: &[u8]) -> Option<SkinSetsFile> {
    let mut file: SkinSetsFile = serde_json::from_slice(bytes).ok()?;
    file.sets.retain(|set| is_set_id(&set.id));
    let known = |id: &String| file.sets.iter().any(|set| set.id == *id);
    file.active = file.active.take().filter(known);
    Some(file)
}

/// One Steam account's sets.
#[derive(Clone, Debug)]
pub(crate) struct SkinStore {
    dir: PathBuf,
}

impl SkinStore {
    pub(crate) fn new(data_dir: &Path, account: &str) -> Self {
        Self {
            dir: data_dir.join(SKIN_SETS_DIR).join(account),
        }
    }

    fn sets_file(&self) -> PathBuf {
        self.dir.join(SETS_FILE)
    }

    fn set_dir(&self, id: &str) -> PathBuf {
        self.dir.join(id)
    }

    fn replaced_dir(&self, id: &str) -> PathBuf {
        self.dir.join(REPLACED_DIR).join(id)
    }

    /// A corrupt list reads as empty and stays on disk until `save` replaces it.
    pub(crate) fn load(&self) -> LoadedSets {
        let path = self.sets_file();
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                if error.kind() != ErrorKind::NotFound {
                    log::warn!("skin set list not read: {error}");
                }
                return LoadedSets {
                    file: SkinSetsFile::default(),
                    read_only: error.kind() != ErrorKind::NotFound,
                };
            }
        };
        let Some(file) = parse(&bytes) else {
            log::error!("skin set list corrupt");
            return LoadedSets {
                file: SkinSetsFile::default(),
                read_only: false,
            };
        };
        let read_only = file.version > SKIN_SETS_VERSION;
        LoadedSets { file, read_only }
    }

    pub(crate) fn save(
        &self,
        guard: &WriteGuard<'_>,
        file: &SkinSetsFile,
    ) -> Result<(), SkinsError> {
        let path = self.sets_file();
        if fs::read(&path).is_ok_and(|bytes| parse(&bytes).is_none()) {
            let backup = backup_file(&path)?;
            log::error!("corrupt skin set list kept as {}", backup.display());
        }
        let bytes = serde_json::to_vec_pretty(file)?;
        guard.write_app_file(&path, &bytes)?;
        Ok(())
    }

    fn write_five(
        guard: &WriteGuard<'_>,
        folder: &Path,
        saves: &SkinSaves,
    ) -> Result<(), SkinsError> {
        for (name, bytes) in SKIN_SAVE_FILES.iter().zip(saves) {
            guard.write_app_file(&folder.join(name), bytes)?;
        }
        Ok(())
    }

    pub(crate) fn write_set(
        &self,
        guard: &WriteGuard<'_>,
        id: &str,
        saves: &SkinSaves,
    ) -> Result<(), SkinsError> {
        Self::write_five(guard, &self.set_dir(id), saves)
    }

    /// The set takes the game's files. Its old copy is kept when they differ: they may be a fresh
    /// install, not a newer state of this set.
    pub(crate) fn refresh_set(
        &self,
        guard: &WriteGuard<'_>,
        id: &str,
        saves: &SkinSaves,
    ) -> Result<(), SkinsError> {
        if let Some(old) = read_five(&self.set_dir(id)).filter(|old| old != saves) {
            Self::add_copy(guard, &self.replaced_dir(id), &old)?;
        }
        self.write_set(guard, id, saves)
    }

    pub(crate) fn read_set(&self, id: &str) -> Result<SkinSaves, SkinsError> {
        read_five(&self.set_dir(id)).ok_or(SkinsError::BrokenSet)
    }

    pub(crate) fn set_bytes(&self, id: &str) -> u64 {
        let folder = self.set_dir(id);
        let size = |name: &&str| fs::metadata(folder.join(name)).map_or(0, |meta| meta.len());
        SKIN_SAVE_FILES.iter().map(size).sum()
    }

    fn remove_five(folder: &Path, parent: &Path) -> Result<(), FsError> {
        for name in SKIN_SAVE_FILES {
            remove_app_file(&folder.join(name), &[folder])?;
        }
        remove_app_dir(folder, parent)?;
        Ok(())
    }

    pub(crate) fn delete_set(&self, id: &str) -> Result<(), SkinsError> {
        Self::remove_five(&self.set_dir(id), &self.dir)?;
        if let Err(error) = self.remove_replaced(id) {
            log::warn!("copies of a deleted skin set stay: {error}");
        }
        Ok(())
    }

    fn remove_replaced(&self, id: &str) -> Result<(), FsError> {
        let copies = self.replaced_dir(id);
        for stamp in stored_folders(&copies) {
            Self::remove_five(&copies.join(stamp), &copies)?;
        }
        remove_app_dir(&copies, &self.dir.join(REPLACED_DIR))?;
        Ok(())
    }

    /// A copy of the game's files from before a switch.
    pub(crate) fn add_backup(
        &self,
        guard: &WriteGuard<'_>,
        saves: &SkinSaves,
    ) -> Result<(), SkinsError> {
        Self::add_copy(guard, &self.dir.join(BACKUPS_DIR), saves)
    }

    /// One more stamped copy in `copies`. Only the newest complete copies stay, so a copy cut
    /// short by a full disk never takes the place of a good one.
    fn add_copy(
        guard: &WriteGuard<'_>,
        copies: &Path,
        saves: &SkinSaves,
    ) -> Result<(), SkinsError> {
        let stamp = Utc::now().format(BACKUP_STAMP).to_string();
        Self::write_five(guard, &copies.join(stamp), saves)?;
        let mut stamps = stored_folders(copies);
        stamps.sort_unstable();
        let mut kept = 0;
        for stamp in stamps.iter().rev() {
            let copy = copies.join(stamp);
            if kept < SKIN_BACKUPS_KEPT && has_five(&copy) {
                kept += 1;
            } else if let Err(error) = Self::remove_five(&copy, copies) {
                log::warn!("old skin backup stays: {error}");
            }
        }
        Ok(())
    }

    /// Every complete automatic copy, newest first.
    pub(crate) fn copies(&self) -> Vec<StoredCopy> {
        let replaced = self.dir.join(REPLACED_DIR);
        let mut copies = complete_copies(&self.dir.join(BACKUPS_DIR), None);
        for set in stored_folders(&replaced).iter().filter(|id| is_set_id(id)) {
            copies.extend(complete_copies(&replaced.join(set), Some(set)));
        }
        copies.sort_by(|a, b| b.stamp.cmp(&a.stamp));
        copies
    }

    pub(crate) fn read_copy(&self, id: &str) -> Result<SkinSaves, SkinsError> {
        let copy = self
            .copies()
            .into_iter()
            .find(|copy| copy.id == id)
            .ok_or(SkinsError::UnknownSet)?;
        read_five(&copy.path).ok_or(SkinsError::BrokenSet)
    }
}

fn has_five(folder: &Path) -> bool {
    SKIN_SAVE_FILES
        .iter()
        .all(|name| folder.join(name).is_file())
}

/// The moment in a copy's folder name; `None` for a name RemMods did not write.
fn stamp_time(stamp: &str) -> Option<String> {
    let time = NaiveDateTime::parse_from_str(stamp, BACKUP_STAMP).ok()?;
    Some(time.and_utc().to_rfc3339_opts(SecondsFormat::Millis, true))
}

fn complete_copies(folder: &Path, set: Option<&str>) -> Vec<StoredCopy> {
    let owner = set.unwrap_or(GAME_COPY);
    stored_folders(folder)
        .into_iter()
        .filter_map(|stamp| {
            let path = folder.join(&stamp);
            let taken = stamp_time(&stamp).filter(|_| has_five(&path))?;
            Some(StoredCopy {
                id: format!("{owner}-{stamp}"),
                taken,
                set: set.map(str::to_owned),
                stamp,
                path,
            })
        })
        .collect()
}

fn stored_folders(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect()
}

/// Every screenshot any account's sets name; `None` when a list can't be read, so nobody decides
/// from half the facts what is unused.
pub(crate) fn all_screenshots(data_dir: &Path) -> Option<HashSet<String>> {
    let root = data_dir.join(SKIN_SETS_DIR);
    let mut used = HashSet::new();
    for account in stored_folders(&root) {
        let path = root.join(account).join(SETS_FILE);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(_) => return None,
        };
        let file = parse(&bytes)?;
        used.extend(file.sets.into_iter().flat_map(|set| set.screenshots));
    }
    Some(used)
}
