//! Per-mod app data in `data_dir/mod-meta.json`; P2 keeps the disable record there (spec §5.4).

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::error::ModsError;
use crate::safe_fs::{FsError, WriteGuard, backup_file};

pub const MOD_META_VERSION: u32 = 1;
pub const MOD_META_FILE: &str = "mod-meta.json";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModMeta {
    pub version: u32,
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub mods: BTreeMap<String, ModMetaEntry>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl ModMeta {
    /// An empty file of the current version.
    pub fn fresh() -> Self {
        Self {
            version: MOD_META_VERSION,
            ..Self::default()
        }
    }

    pub fn record(&self, key: &str) -> Option<&DisableRecord> {
        self.mods.get(key)?.disabled.as_ref()
    }

    /// Sets or clears the disable record of `key`; an entry left with no fields is removed.
    pub fn set_record(&mut self, key: &str, record: Option<DisableRecord>) {
        let entry = self.mods.entry(key.to_owned()).or_default();
        entry.disabled = record;
        if entry.is_empty() {
            self.mods.remove(key);
        }
    }

    /// Moves the whole entry of `old` to `new`; an existing entry under `new` is replaced.
    /// `true` when `old` had an entry.
    pub fn rekey(&mut self, old: &str, new: &str) -> bool {
        if old == new {
            return self.mods.contains_key(old);
        }
        let Some(entry) = self.mods.remove(old) else {
            return false;
        };
        if self.mods.insert(new.to_owned(), entry).is_some() {
            log::warn!("mod-meta.json: the entry of the renamed key replaced an existing one");
        }
        true
    }

    pub fn remove(&mut self, key: &str) {
        self.mods.remove(key);
    }

    pub fn picture(&self, key: &str) -> Option<&str> {
        self.mods.get(key)?.picture.as_deref()
    }

    /// Sets or clears the mod's own picture; an entry left with no fields is removed.
    pub fn set_picture(&mut self, key: &str, picture: Option<String>) {
        let entry = self.mods.entry(key.to_owned()).or_default();
        entry.picture = picture;
        if entry.is_empty() {
            self.mods.remove(key);
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModMetaEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<DisableRecord>,
    /// RFC 3339, written by P4 installs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub added: Option<String>,
    /// e.g. the archive file name (P4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// The owner's own picture: an id in the assets folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub picture: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl ModMetaEntry {
    fn is_empty(&self) -> bool {
        self.disabled.is_none()
            && self.added.is_none()
            && self.source.is_none()
            && self.picture.is_none()
            && self.extra.is_empty()
    }
}

/// The files a disable renamed (rel paths below the mod folder) and when.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisableRecord {
    pub files: Vec<String>,
    pub at: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoadedMeta {
    pub meta: ModMeta,
    /// A newer or unreadable file: its records are ignored and it is never written.
    pub read_only: bool,
}

enum Disk {
    Missing,
    Parsed(ModMeta),
    Corrupt,
}

#[derive(Clone, Debug)]
pub struct ModMetaStore {
    path: PathBuf,
}

impl ModMetaStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Missing → empty. Unparsable → backed up, empty. Newer version or unreadable → read-only.
    pub fn load(&self) -> LoadedMeta {
        match read_disk(&self.path) {
            Ok(Disk::Missing) => writable(ModMeta::fresh()),
            Ok(Disk::Corrupt) => {
                self.back_up_corrupt();
                writable(ModMeta::fresh())
            }
            Ok(Disk::Parsed(meta)) if meta.version > MOD_META_VERSION => {
                log::warn!(
                    "mod-meta.json has version {}: records ignored, never written",
                    meta.version
                );
                LoadedMeta {
                    meta: ModMeta::fresh(),
                    read_only: true,
                }
            }
            Ok(Disk::Parsed(meta)) => writable(meta),
            Err(error) => {
                log::warn!("mod-meta.json unreadable, records ignored: {error}");
                LoadedMeta {
                    meta: ModMeta::fresh(),
                    read_only: true,
                }
            }
        }
    }

    /// Under the write guard: re-read → `change` → revision + 1 → atomic write. A newer file is
    /// left alone (Ok, logged).
    pub fn update(
        &self,
        guard: &WriteGuard<'_>,
        change: impl FnOnce(&mut ModMeta),
    ) -> Result<(), ModsError> {
        let mut meta = match read_disk(&self.path)? {
            Disk::Missing => ModMeta::fresh(),
            Disk::Corrupt => {
                self.back_up_corrupt();
                ModMeta::fresh()
            }
            Disk::Parsed(meta) if meta.version > MOD_META_VERSION => {
                log::warn!("mod-meta.json is newer: the disable record is not written");
                return Ok(());
            }
            Disk::Parsed(meta) => meta,
        };
        change(&mut meta);
        meta.mods.retain(|_, entry| !entry.is_empty());
        meta.revision = meta.revision.saturating_add(1);
        let bytes = serde_json::to_vec_pretty(&meta)
            .map_err(|error| FsError::io("encode", &self.path, io::Error::other(error)))?;
        guard.write_json(&self.path, &bytes)?;
        Ok(())
    }

    fn back_up_corrupt(&self) {
        match backup_file(&self.path) {
            Ok(backup) => log::error!(
                "mod-meta.json is corrupt; backed up to {}",
                backup.display()
            ),
            Err(error) => log::error!("mod-meta.json is corrupt and its backup failed: {error}"),
        }
    }
}

fn writable(meta: ModMeta) -> LoadedMeta {
    LoadedMeta {
        meta,
        read_only: false,
    }
}

fn read_disk(path: &Path) -> Result<Disk, FsError> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Disk::Missing),
        Err(error) => return Err(FsError::io("read", path, error)),
    };
    Ok(serde_json::from_slice(&bytes).map_or(Disk::Corrupt, Disk::Parsed))
}
