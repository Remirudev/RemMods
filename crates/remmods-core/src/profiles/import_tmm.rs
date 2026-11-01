//! Reading the original app's `profiles.json` (spec §5.9, PLAN §7). The file is only read: never
//! written, renamed or locked.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use serde_json::Value;

use super::error::{ImportProblem, ProfileError};
use super::model::{
    EntryState, Profile, ProfileEntry, ProfileMode, ProfileSource, ProfilesFile, unique_name,
};
use crate::mods::decode_modinfo_text;
use crate::safe_fs::FsError;

pub const IMPORT_MAX_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TmmProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled_mods: Vec<String>,
    pub is_default: bool,
}

/// Merge counts of one import.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct MergeCounts {
    pub added: u32,
    pub updated: u32,
    pub unchanged: u32,
}

/// Size check, then a plain read (no lock, no write).
pub(crate) fn read_import_file(file: &Path) -> Result<Vec<u8>, ProfileError> {
    let fs_path =
        crate::platform::fs_path(file).map_err(|error| FsError::io("read", file, error))?;
    let metadata = std::fs::metadata(&fs_path).map_err(|error| FsError::io("read", file, error))?;
    if metadata.len() > IMPORT_MAX_BYTES {
        return Err(ProfileError::Unparsable(ImportProblem::TooLarge));
    }
    let bytes = std::fs::read(&fs_path).map_err(|error| FsError::io("read", file, error))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > IMPORT_MAX_BYTES {
        return Err(ProfileError::Unparsable(ImportProblem::TooLarge));
    }
    Ok(bytes)
}

/// BOM sniffing and lossy decoding like the original, then JSON.
pub(crate) fn decode_json(bytes: &[u8]) -> Result<Value, ProfileError> {
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > IMPORT_MAX_BYTES {
        return Err(ProfileError::Unparsable(ImportProblem::TooLarge));
    }
    serde_json::from_str(&decode_modinfo_text(bytes))
        .map_err(|_| ProfileError::Unparsable(ImportProblem::NotJson))
}

/// The original's `ModProfile` list; elements without a usable `Id` are counted as skipped. Pure.
pub fn parse_tmm_profiles(bytes: &[u8]) -> Result<(Vec<TmmProfile>, u32), ProfileError> {
    let Value::Array(items) = decode_json(bytes)? else {
        return Err(ProfileError::Unparsable(ImportProblem::WrongShape));
    };
    let mut skipped = 0_u32;
    let mut profiles = Vec::new();
    for item in items {
        match tmm_profile(&item) {
            Some(profile) => profiles.push(profile),
            None => skipped += 1,
        }
    }
    Ok((profiles, skipped))
}

fn tmm_profile(item: &Value) -> Option<TmmProfile> {
    let object = item.as_object()?;
    let id = object.get("Id")?.as_str()?;
    if id.trim().is_empty() {
        return None;
    }
    let text = |field: &str| {
        object
            .get(field)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let mut seen = HashSet::new();
    let enabled_mods = object
        .get("EnabledMods")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            if entry.contains(':') {
                entry.to_owned()
            } else {
                format!("Standard:{entry}")
            }
        })
        .filter(|entry| seen.insert(entry.clone()))
        .collect();
    Some(TmmProfile {
        id: id.to_owned(),
        name: text("Name"),
        description: text("Description"),
        enabled_mods,
        is_default: object
            .get("IsDefault")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

/// Merges by the original's `Id` (spec §5.9): equal → unchanged, different → replaced in place
/// (RemMods id kept), unknown → added as Exclusive with every key Enabled.
pub(crate) fn merge_tmm(file: &mut ProfilesFile, imported: Vec<TmmProfile>) -> MergeCounts {
    let mut counts = MergeCounts::default();
    for tmm in imported {
        let mods: BTreeMap<String, ProfileEntry> = tmm
            .enabled_mods
            .iter()
            .map(|key| (key.clone(), ProfileEntry::new(EntryState::Enabled)))
            .collect();
        let existing = file.profiles.iter_mut().find(
            |profile| matches!(&profile.source, ProfileSource::Tmm { id, .. } if *id == tmm.id),
        );
        if let Some(profile) = existing {
            if profile.name == tmm.name
                && profile.description == tmm.description
                && profile.mods == mods
            {
                counts.unchanged += 1;
            } else {
                profile.name = tmm.name;
                profile.description = tmm.description;
                profile.mods = mods;
                profile.updated = super::model::now_rfc3339();
                counts.updated += 1;
            }
            continue;
        }
        let name = if tmm.name.trim().is_empty() {
            tmm.name.clone()
        } else {
            unique_name(file, &tmm.name)
        };
        let source = ProfileSource::Tmm {
            id: tmm.id,
            is_default: tmm.is_default,
        };
        file.profiles.push(Profile::fresh(
            name,
            tmm.description,
            ProfileMode::Exclusive,
            mods,
            source,
        ));
        counts.added += 1;
    }
    counts
}
