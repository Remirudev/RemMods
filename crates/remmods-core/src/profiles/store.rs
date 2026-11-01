//! `data_dir/profiles.json`: lenient reads without a lock, writes under the write guard
//! (spec §5.7). A corrupt file or one with dropped profiles is backed up before it is replaced.

use std::io;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::error::ProfileError;
use super::model::{LastApplied, PROFILES_VERSION, Profile, ProfilesFile};
use crate::safe_fs::{FsError, WriteGuard, backup_file};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreProblem {
    Corrupt,
    NewerVersion,
    Unreadable,
}

#[derive(Clone, Debug)]
pub struct LoadedProfiles {
    pub file: ProfilesFile,
    pub read_only: bool,
    pub problem: Option<StoreProblem>,
}

#[derive(Clone, Debug)]
pub struct ProfileStore {
    path: PathBuf,
}

enum Disk {
    Missing,
    /// The parsed file and how many profiles were dropped as unparseable.
    Parsed(ProfilesFile, usize),
    Corrupt,
}

impl ProfileStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// No lock; never writes. Missing → empty file (revision 0). Corrupt → empty + problem Corrupt
    /// (backed up on the next write). Newer version → its profiles parsed leniently if possible,
    /// read_only = true.
    pub fn load(&self) -> LoadedProfiles {
        match read_disk(&self.path) {
            Ok(Disk::Missing) => loaded(ProfilesFile::default(), false, None),
            Ok(Disk::Corrupt) => {
                loaded(ProfilesFile::default(), false, Some(StoreProblem::Corrupt))
            }
            Ok(Disk::Parsed(file, _)) if file.version > PROFILES_VERSION => {
                loaded(file, true, Some(StoreProblem::NewerVersion))
            }
            Ok(Disk::Parsed(file, _)) => loaded(file, false, None),
            Err(error) => {
                log::warn!("profiles.json unreadable: {error}");
                loaded(
                    ProfilesFile::default(),
                    true,
                    Some(StoreProblem::Unreadable),
                )
            }
        }
    }

    /// Under the write guard: re-read → change → revision + 1 → `guard.write_json`. Corrupt →
    /// `backup_file` first, then start empty. Newer version → Err(ProfileError::ReadOnly).
    pub fn update<T>(
        &self,
        guard: &WriteGuard<'_>,
        change: impl FnOnce(&mut ProfilesFile) -> Result<T, ProfileError>,
    ) -> Result<T, ProfileError> {
        let mut file = match read_disk(&self.path)? {
            Disk::Missing => ProfilesFile::default(),
            Disk::Corrupt => {
                let backup = backup_file(&self.path)?;
                log::error!(
                    "profiles.json is corrupt; backed up to {}",
                    backup.display()
                );
                ProfilesFile::default()
            }
            Disk::Parsed(file, _) if file.version > PROFILES_VERSION => {
                return Err(ProfileError::ReadOnly);
            }
            Disk::Parsed(file, dropped) => {
                if dropped > 0 {
                    let backup = backup_file(&self.path)?;
                    log::warn!(
                        "profiles.json: {dropped} unreadable profiles dropped; backed up to {}",
                        backup.display()
                    );
                }
                file
            }
        };
        let value = change(&mut file)?;
        file.version = PROFILES_VERSION;
        file.revision = file.revision.saturating_add(1);
        let bytes = serde_json::to_vec_pretty(&file)
            .map_err(|error| FsError::io("encode", &self.path, io::Error::other(error)))?;
        guard.write_json(&self.path, &bytes)?;
        Ok(value)
    }
}

fn loaded(file: ProfilesFile, read_only: bool, problem: Option<StoreProblem>) -> LoadedProfiles {
    LoadedProfiles {
        file,
        read_only,
        problem,
    }
}

fn read_disk(path: &Path) -> Result<Disk, FsError> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Disk::Missing),
        Err(error) => return Err(FsError::io("read", path, error)),
    };
    Ok(serde_json::from_slice::<Value>(&bytes)
        .ok()
        .and_then(lenient)
        .map_or(Disk::Corrupt, |(file, dropped)| Disk::Parsed(file, dropped)))
}

/// The file with every profile that parses; None when the top level is not a profiles file.
fn lenient(value: Value) -> Option<(ProfilesFile, usize)> {
    let Value::Object(mut map) = value else {
        return None;
    };
    let version = u32::try_from(map.remove("version")?.as_u64()?).ok()?;
    let revision = map.remove("revision").and_then(|v| v.as_u64()).unwrap_or(0);
    let Value::Array(raw) = map.remove("profiles").unwrap_or(Value::Array(Vec::new())) else {
        return None;
    };
    let last_applied = map
        .remove("lastApplied")
        .and_then(|value| serde_json::from_value::<LastApplied>(value).ok());
    let (profiles, dropped) = parse_profiles(raw);
    Some((
        ProfilesFile {
            version,
            revision,
            profiles,
            last_applied,
            extra: map,
        },
        dropped,
    ))
}

/// Profiles that parse (unknown `mode`, `source` or entry `state` drop the profile); logged.
pub(crate) fn parse_profiles(raw: Vec<Value>) -> (Vec<Profile>, usize) {
    let mut dropped = 0;
    let profiles = raw
        .into_iter()
        .filter_map(|value| match serde_json::from_value::<Profile>(value) {
            Ok(profile) => Some(profile),
            Err(error) => {
                log::warn!("profiles.json: a profile was skipped: {error}");
                dropped += 1;
                None
            }
        })
        .collect();
    (profiles, dropped)
}
