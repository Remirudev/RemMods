//! The game's own file paths from the top-level base `.utoc` indexes (§4.5, §5.3 rule 5). Built
//! in the background after the first scan, kept in memory only, rebuilt when the base files change.
//! Content conflicts are limited to asset keys found here.

use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use remmods_pak::{PakError, asset_key, file_key, read_toc};

use super::model::BaseIndexState;
use crate::mods::payload::is_game_file;
use crate::platform;
use crate::safe_fs::names::key_text;

type Fingerprint = Vec<(String, u64, Option<SystemTime>)>;

#[derive(Debug)]
pub struct BaseIndex {
    paks: PathBuf,
    fingerprint: Fingerprint,
    state: BaseIndexState,
    /// `DefaultHasher` hashes of file keys.
    files: HashSet<u64>,
    /// `DefaultHasher` hashes of asset keys.
    assets: HashSet<u64>,
}

struct BaseContainer {
    name: String,
    path: PathBuf,
    size: u64,
    modified: Option<SystemTime>,
}

fn hash_key(key: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

fn is_base_utoc(name: &std::ffi::OsStr) -> bool {
    is_game_file(name)
        && name
            .as_encoded_bytes()
            .to_ascii_lowercase()
            .ends_with(b".utoc")
}

/// Top-level base `.utoc` files of Paks (plain form), sorted by name. A failed listing is none.
fn base_containers(paks: &Path) -> Vec<BaseContainer> {
    let entries = platform::fs_path(paks).and_then(fs::read_dir);
    let Ok(entries) = entries else {
        return Vec::new();
    };
    let mut found: Vec<BaseContainer> = entries
        .flatten()
        .filter(|entry| is_base_utoc(&entry.file_name()))
        .filter_map(|entry| {
            let path = entry.path();
            let metadata = fs::metadata(&path).ok().filter(fs::Metadata::is_file)?;
            Some(BaseContainer {
                name: key_text(&entry.file_name()),
                path,
                size: metadata.len(),
                modified: metadata.modified().ok(),
            })
        })
        .collect();
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

fn fingerprint_of(containers: &[BaseContainer]) -> Fingerprint {
    containers
        .iter()
        .map(|c| (c.name.clone(), c.size, c.modified))
        .collect()
}

impl BaseIndex {
    /// A fresh build for `paks`.
    pub fn load(paks: &Path) -> Arc<BaseIndex> {
        Self::current(paks, None)
    }

    pub fn state(&self) -> BaseIndexState {
        self.state
    }

    pub(crate) fn is_for(&self, paks: &Path) -> bool {
        self.paks == paks
    }

    pub(crate) fn contains(&self, file_key: &str) -> bool {
        self.files.contains(&hash_key(file_key))
    }

    pub(crate) fn contains_asset(&self, asset_key: &str) -> bool {
        self.assets.contains(&hash_key(asset_key))
    }

    /// `previous` when it was built for the same Paks and fingerprint, else a fresh build.
    pub(crate) fn current(paks: &Path, previous: Option<Arc<BaseIndex>>) -> Arc<BaseIndex> {
        let containers = base_containers(paks);
        let fingerprint = fingerprint_of(&containers);
        match previous {
            Some(p) if p.paks == paks && p.fingerprint == fingerprint => p,
            _ => Arc::new(Self::build(paks, fingerprint, &containers)),
        }
    }

    fn build(paks: &Path, fingerprint: Fingerprint, containers: &[BaseContainer]) -> BaseIndex {
        let mut files = HashSet::new();
        let mut assets = HashSet::new();
        let (mut indexed, mut failed) = (0u32, 0u32);
        for container in containers {
            match read_toc(&container.path) {
                Ok(listing) => {
                    indexed += 1;
                    for file in &listing.files {
                        files.insert(hash_key(&file_key(&file.path)));
                        assets.insert(hash_key(&asset_key(&file.path)));
                    }
                }
                Err(PakError::NoDirectoryIndex) => {}
                Err(error) => {
                    failed += 1;
                    log::info!("content: base index {} not read: {error}", container.name);
                }
            }
        }
        let state = match (indexed, failed) {
            (0, _) => BaseIndexState::Unavailable,
            (_, 0) => BaseIndexState::Complete,
            _ => BaseIndexState::Partial,
        };
        log::info!(
            "content-base\tcontainers={}\tindexed={indexed}\tfailed={failed}\tpaths={}",
            containers.len(),
            files.len()
        );
        BaseIndex {
            paks: paks.to_path_buf(),
            fingerprint,
            state,
            files,
            assets,
        }
    }
}
