//! Per-container cache: a `.utoc` index is read once and kept while its size and mtime hold.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::SystemTime;

use remmods_pak::hints::hint_for;
use remmods_pak::{ContentHash, PakError, TocListing, asset_key, file_key, read_toc};

use super::model::{ContainerStatus, UtocRef};
use crate::platform;

/// The (file key, content hash) of each file of one asset, sorted; `None` when a hash is unknown.
pub(crate) type Fingerprint = Option<Vec<(String, ContentHash)>>;

/// A listing plus what every scan derives from it, computed once per read.
#[derive(Debug)]
pub(crate) struct Container {
    pub listing: TocListing,
    /// (asset key, index of its smallest display path in `listing.files`), sorted, unique.
    pub assets: Vec<(String, usize)>,
    /// Parallel to `assets`.
    pub prints: Vec<Fingerprint>,
    pub characters: BTreeSet<String>,
    pub slots: BTreeSet<String>,
}

impl Container {
    fn new(listing: TocListing) -> Self {
        let mut assets: BTreeMap<String, (usize, Fingerprint)> = BTreeMap::new();
        let mut characters = BTreeSet::new();
        let mut slots = BTreeSet::new();
        for (index, file) in listing.files.iter().enumerate() {
            // Files are sorted by display path, so the first index per key is the smallest path.
            let (_, print) = assets
                .entry(asset_key(&file.path))
                .or_insert_with(|| (index, Some(Vec::new())));
            match (print.as_mut(), file.content_hash) {
                (Some(files), Some(hash)) => files.push((file_key(&file.path), hash)),
                _ => *print = None,
            }
            let hint = hint_for(&file.path);
            characters.extend(hint.character);
            slots.extend(hint.slot);
        }
        let (assets, prints) = assets
            .into_iter()
            .map(|(key, (index, mut print))| {
                if let Some(files) = print.as_mut() {
                    files.sort_unstable();
                }
                ((key, index), print)
            })
            .unzip();
        Self {
            listing,
            assets,
            prints,
            characters,
            slots,
        }
    }

    pub(crate) fn path_of(&self, index: usize) -> &str {
        self.listing
            .files
            .get(index)
            .map_or("", |file| file.path.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Failure {
    pub status: ContainerStatus,
    pub version: Option<u8>,
}

pub(crate) type ContainerResult = Arc<Result<Container, Failure>>;

struct Cached {
    size: u64,
    modified: SystemTime,
    result: ContainerResult,
}

/// Keyed by fs path; an entry is valid while (size, mtime) match. Also remembers the last scan's
/// containers per row, so `Core::mods_contents` finds them by `mod_ref`.
#[derive(Default)]
pub struct ContentCache {
    entries: Mutex<HashMap<PathBuf, Cached>>,
    rows: Mutex<HashMap<String, Vec<UtocRef>>>,
}

impl fmt::Debug for ContentCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContentCache")
            .field("len", &self.len())
            .finish_non_exhaustive()
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl ContentCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        lock(&self.entries).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The cached result when path, size and mtime match (a `None` mtime never hits), else a
    /// fresh read, cached either way. The flag is `true` when the file was read.
    pub(crate) fn get_or_read(&self, utoc: &UtocRef) -> (ContainerResult, bool) {
        if let Some(modified) = utoc.modified
            && let Some(cached) = lock(&self.entries).get(&utoc.path)
            && cached.size == utoc.size
            && cached.modified == modified
        {
            return (Arc::clone(&cached.result), false);
        }
        let result = Arc::new(read_container(&utoc.path));
        if let Some(modified) = utoc.modified {
            lock(&self.entries).insert(
                utoc.path.clone(),
                Cached {
                    size: utoc.size,
                    modified,
                    result: Arc::clone(&result),
                },
            );
        }
        (result, true)
    }

    /// Eviction after a scan: only containers that scan saw stay.
    pub(crate) fn retain(&self, seen: &HashSet<PathBuf>) {
        lock(&self.entries).retain(|path, _| seen.contains(path));
    }

    pub(crate) fn set_rows(&self, rows: HashMap<String, Vec<UtocRef>>) {
        *lock(&self.rows) = rows;
    }

    pub(crate) fn row_containers(&self, mod_ref: &str) -> Option<Vec<UtocRef>> {
        lock(&self.rows).get(mod_ref).cloned()
    }
}

fn read_container(path: &std::path::Path) -> Result<Container, Failure> {
    match read_toc(path) {
        Ok(listing) => Ok(Container::new(listing)),
        Err(error) => {
            let failure = failure(&error);
            if let PakError::Io(io) = &error {
                log::debug!(
                    "content: cannot read {}: {:?}",
                    path.display(),
                    platform::classify(io)
                );
            }
            Err(failure)
        }
    }
}

fn failure(error: &PakError) -> Failure {
    let (status, version) = match error {
        PakError::Io(_) => (ContainerStatus::Unreadable, None),
        PakError::NotIoStore => (ContainerStatus::NotIoStore, None),
        PakError::UnsupportedVersion(version) => {
            (ContainerStatus::UnsupportedVersion, Some(*version))
        }
        PakError::Encrypted => (ContainerStatus::Encrypted, None),
        PakError::NoDirectoryIndex => (ContainerStatus::NoIndex, None),
        PakError::Malformed(_) => (ContainerStatus::Malformed, None),
        PakError::TooLarge => (ContainerStatus::TooLarge, None),
    };
    Failure { status, version }
}
