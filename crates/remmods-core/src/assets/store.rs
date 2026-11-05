//! The folder `<data_dir>/assets`: `pictures/<id>.png` and `names.<language>.json`.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use serde::de::DeserializeOwned;

use super::mod_names::{MOD_NAMES_VERSION, ModNames};
use super::model::AssetsStatus;
use super::names::{ItemNames, NAMES_VERSION};
use crate::paths::AppPaths;
use crate::safe_fs::{self, AppLock, FsError};

const ASSETS_DIR: &str = "assets";
const PICTURES_DIR: &str = "pictures";
const PICTURE_EXTENSION: &str = "png";
const LOCK_WAIT: Duration = Duration::from_secs(2);
const ID_HEX_LEN: usize = 32;

#[derive(Debug)]
pub struct AssetStore {
    root: PathBuf,
    pictures: PathBuf,
    lock_file: PathBuf,
    /// Ids on disk; `None` until the folder was listed.
    known: Mutex<Option<HashSet<String>>>,
    /// Ids that failed to decode this session: not tried again on every scan.
    failed: Mutex<HashSet<String>>,
    names: Mutex<HashMap<String, Arc<ItemNames>>>,
    mod_names: Mutex<HashMap<String, Arc<ModNames>>>,
    /// One refresh at a time; the second waits and then finds nothing to do.
    refresh: Mutex<()>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `<edge>-<32 hex>`: ids reach this store from the UI, and they become file names.
fn is_picture_id(id: &str) -> bool {
    id.split_once('-').is_some_and(|(edge, hex)| {
        !edge.is_empty()
            && edge.bytes().all(|byte| byte.is_ascii_digit())
            && hex.len() == ID_HEX_LEN
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn stored_ids(pictures: &Path) -> HashSet<String> {
    let Ok(entries) = fs::read_dir(pictures) else {
        return HashSet::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let id = path.file_stem()?.to_str()?;
            is_picture_id(id).then(|| id.to_owned())
        })
        .collect()
}

impl AssetStore {
    pub fn new(paths: &AppPaths) -> Self {
        let root = paths.data_dir.join(ASSETS_DIR);
        Self {
            pictures: root.join(PICTURES_DIR),
            root,
            lock_file: paths.lock_file.clone(),
            known: Mutex::new(None),
            failed: Mutex::new(HashSet::new()),
            names: Mutex::new(HashMap::new()),
            mod_names: Mutex::new(HashMap::new()),
            refresh: Mutex::new(()),
        }
    }

    pub(crate) fn refresh_turn(&self) -> MutexGuard<'_, ()> {
        lock(&self.refresh)
    }

    fn with_known<T>(&self, use_ids: impl FnOnce(&mut HashSet<String>) -> T) -> T {
        use_ids(lock(&self.known).get_or_insert_with(|| stored_ids(&self.pictures)))
    }

    pub(crate) fn has_picture(&self, id: &str) -> bool {
        self.with_known(|known| known.contains(id))
    }

    pub(crate) fn needs_picture(&self, id: &str) -> bool {
        !self.has_picture(id) && !lock(&self.failed).contains(id)
    }

    fn picture_path(&self, id: &str) -> PathBuf {
        self.pictures.join(format!("{id}.{PICTURE_EXTENSION}"))
    }

    /// The name holds the content, so no lock: the same id always gets the same bytes.
    pub(crate) fn add_picture(&self, id: &str, png: &[u8]) -> Result<(), FsError> {
        safe_fs::write_atomic(&self.picture_path(id), png)?;
        self.with_known(|known| known.insert(id.to_owned()));
        Ok(())
    }

    pub(crate) fn mark_failed(&self, id: &str) {
        lock(&self.failed).insert(id.to_owned());
    }

    /// Removes every stored picture that `wanted` does not name; `true` when one went.
    pub(crate) fn remove_others<T>(&self, wanted: &BTreeMap<String, T>) -> bool {
        let unused: Vec<String> = self.with_known(|known| {
            let unused = known.iter().filter(|id| !wanted.contains_key(*id));
            unused.cloned().collect()
        });
        let mut changed = false;
        for id in unused {
            let removed = safe_fs::remove_app_file(&self.picture_path(&id), &[&self.pictures]);
            if let Err(error) = &removed {
                log::info!("assets: picture not removed: {error}");
                continue;
            }
            self.with_known(|known| known.remove(&id));
            changed = true;
        }
        changed
    }

    /// PNG bytes; `None` for an unknown id or a file that went away.
    pub(crate) fn read_picture(&self, id: &str) -> Option<Vec<u8>> {
        if !is_picture_id(id) {
            return None;
        }
        match fs::read(self.picture_path(id)) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == ErrorKind::NotFound => None,
            Err(error) => {
                log::info!("assets: picture not read: {error}");
                None
            }
        }
    }

    fn names_path(&self, language: &str) -> PathBuf {
        self.root.join(format!("names.{language}.json"))
    }

    fn mod_names_path(&self, language: &str) -> PathBuf {
        self.root.join(format!("mod-names.{language}.json"))
    }

    /// From memory, else from its file. A file of another format reads as missing.
    pub(crate) fn names(&self, language: &str) -> Option<Arc<ItemNames>> {
        let mut names = lock(&self.names);
        if let Some(loaded) = names.get(language) {
            return Some(Arc::clone(loaded));
        }
        let parsed = read_json::<ItemNames>(&self.names_path(language))
            .filter(|parsed| parsed.version == NAMES_VERSION)?;
        let parsed = Arc::new(parsed);
        names.insert(language.to_owned(), Arc::clone(&parsed));
        Some(parsed)
    }

    pub(crate) fn store_names(&self, language: &str, table: ItemNames) -> Result<(), FsError> {
        self.write_json(&self.names_path(language), &table)?;
        lock(&self.names).insert(language.to_owned(), Arc::new(table));
        Ok(())
    }

    /// From memory, else from its file; empty when there is none.
    pub(crate) fn mod_names(&self, language: &str) -> Arc<ModNames> {
        let mut names = lock(&self.mod_names);
        if let Some(loaded) = names.get(language) {
            return Arc::clone(loaded);
        }
        let parsed = read_json::<ModNames>(&self.mod_names_path(language))
            .filter(|parsed| parsed.version == MOD_NAMES_VERSION)
            .unwrap_or_default();
        let parsed = Arc::new(parsed);
        names.insert(language.to_owned(), Arc::clone(&parsed));
        parsed
    }

    pub(crate) fn store_mod_names(&self, language: &str, table: ModNames) -> Result<(), FsError> {
        self.write_json(&self.mod_names_path(language), &table)?;
        lock(&self.mod_names).insert(language.to_owned(), Arc::new(table));
        Ok(())
    }

    fn write_json<T: serde::Serialize>(&self, path: &Path, value: &T) -> Result<(), FsError> {
        let bytes = serde_json::to_vec(value)
            .map_err(|error| FsError::io("assets_names", path, error.into()))?;
        let _app_lock = AppLock::exclusive_within(&self.lock_file, LOCK_WAIT)?;
        safe_fs::write_atomic(path, &bytes)
    }

    /// Reads the size of every stored picture: meant for the Settings page, not for a loop.
    pub(crate) fn status(&self, language: &str) -> AssetsStatus {
        let ids: Vec<String> = self.with_known(|known| known.iter().cloned().collect());
        let size = |id: &String| fs::metadata(self.picture_path(id)).map_or(0, |meta| meta.len());
        let count = |value: usize| u32::try_from(value).unwrap_or(u32::MAX);
        AssetsStatus {
            pictures: count(ids.len()),
            bytes: ids.iter().map(size).sum(),
            names: self.names(language).map_or(0, |names| count(names.len())),
            failed: count(lock(&self.failed).len()),
        }
    }
}
