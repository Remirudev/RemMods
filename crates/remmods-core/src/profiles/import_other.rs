//! Reading another mod manager's `profiles.json` (spec §5.9): a list of
//! `Id`/`Name`/`Description`/`EnabledMods`/`IsDefault` objects. The file is only read.

use std::collections::{BTreeMap, HashSet};

use serde_json::Value;

use super::error::{ImportProblem, ProfileError};
use super::import_file::{MergeCounts, decode_json};
use super::model::{
    EntryState, Profile, ProfileEntry, ProfileMode, ProfileSource, ProfilesFile, unique_name,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OtherManagerProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled_mods: Vec<String>,
    pub is_default: bool,
}

/// Elements without a usable `Id` are counted as skipped. Pure.
pub fn parse_other_manager_profiles(
    bytes: &[u8],
) -> Result<(Vec<OtherManagerProfile>, u32), ProfileError> {
    let Value::Array(items) = decode_json(bytes)? else {
        return Err(ProfileError::Unparsable(ImportProblem::WrongShape));
    };
    let mut skipped = 0_u32;
    let mut profiles = Vec::new();
    for item in items {
        match other_manager_profile(&item) {
            Some(profile) => profiles.push(profile),
            None => skipped += 1,
        }
    }
    Ok((profiles, skipped))
}

fn other_manager_profile(item: &Value) -> Option<OtherManagerProfile> {
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
    Some(OtherManagerProfile {
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

/// Merges by `Id` (spec §5.9): equal → unchanged, different → replaced in place
/// (RemMods id kept), unknown → added as Exclusive with every key Enabled.
pub(crate) fn merge_other_manager(
    file: &mut ProfilesFile,
    imported: Vec<OtherManagerProfile>,
) -> MergeCounts {
    let mut counts = MergeCounts::default();
    for entry in imported {
        let mods: BTreeMap<String, ProfileEntry> = entry
            .enabled_mods
            .iter()
            .map(|key| (key.clone(), ProfileEntry::new(EntryState::Enabled)))
            .collect();
        let existing = file.profiles.iter_mut().find(
            |profile| matches!(&profile.source, ProfileSource::OtherManager { id, .. } if *id == entry.id),
        );
        if let Some(profile) = existing {
            if profile.name == entry.name
                && profile.description == entry.description
                && profile.mods == mods
            {
                counts.unchanged += 1;
            } else {
                profile.name = entry.name;
                profile.description = entry.description;
                profile.mods = mods;
                profile.updated = super::model::now_rfc3339();
                counts.updated += 1;
            }
            continue;
        }
        let name = if entry.name.trim().is_empty() {
            entry.name.clone()
        } else {
            unique_name(file, &entry.name)
        };
        let source = ProfileSource::OtherManager {
            id: entry.id,
            is_default: entry.is_default,
        };
        file.profiles.push(Profile::fresh(
            name,
            entry.description,
            ProfileMode::Exclusive,
            mods,
            source,
        ));
        counts.added += 1;
    }
    counts
}
