//! RemMods' own profile export file (spec §5.9).

use serde::Serialize;
use serde_json::Value;

use super::api::ProfilesView;
use super::error::{ImportProblem, ProfileError};
use super::import_tmm::{MergeCounts, decode_json};
use super::model::{Profile, ProfileSource, ProfilesFile, is_profile_id, new_id, unique_name};
use super::store::parse_profiles;

const EXPORT_FORMAT: &str = "remmods-profiles";
const EXPORT_VERSION: u64 = 1;

#[derive(Clone, Debug, PartialEq)]
pub struct ImportReport {
    pub added: u32,
    pub updated: u32,
    pub unchanged: u32,
    pub skipped: u32,
    pub view: ProfilesView,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportReport {
    pub count: u32,
    pub file_name: String,
}

/// Field order as written: format, version, exportedAt, profiles.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportFile<'a> {
    format: &'a str,
    version: u64,
    exported_at: &'a str,
    profiles: Vec<&'a Profile>,
}

/// Pretty JSON, 2-space indent, LF, trailing newline; snapshots are left out. Pure.
pub fn export_bytes(profiles: &[Profile], now: &str) -> Vec<u8> {
    let listed: Vec<&Profile> = profiles
        .iter()
        .filter(|profile| !profile.is_snapshot())
        .collect();
    let document = ExportFile {
        format: EXPORT_FORMAT,
        version: EXPORT_VERSION,
        exported_at: now,
        profiles: listed,
    };
    let mut bytes = serde_json::to_vec_pretty(&document).unwrap_or_default();
    bytes.push(b'\n');
    bytes
}

/// The valid, non-snapshot profiles of an export file and how many were skipped. Pure.
pub fn parse_export(bytes: &[u8]) -> Result<(Vec<Profile>, u32), ProfileError> {
    let Value::Object(mut map) = decode_json(bytes)? else {
        return Err(ProfileError::Unparsable(ImportProblem::WrongShape));
    };
    if map.get("format").and_then(Value::as_str) != Some(EXPORT_FORMAT) {
        return Err(ProfileError::Unparsable(ImportProblem::WrongFormat));
    }
    match map.get("version").and_then(Value::as_u64) {
        Some(version) if version <= EXPORT_VERSION => {}
        Some(_) => return Err(ProfileError::Unparsable(ImportProblem::NewerVersion)),
        None => return Err(ProfileError::Unparsable(ImportProblem::WrongShape)),
    }
    let Some(Value::Array(raw)) = map.remove("profiles") else {
        return Err(ProfileError::Unparsable(ImportProblem::WrongShape));
    };
    let (parsed, dropped) = parse_profiles(raw);
    let mut skipped = u32::try_from(dropped).unwrap_or(u32::MAX);
    let mut profiles = Vec::new();
    for profile in parsed {
        if profile.is_snapshot() || !is_profile_id(&profile.id) || profile.name.trim().is_empty() {
            skipped = skipped.saturating_add(1);
        } else {
            profiles.push(profile);
        }
    }
    Ok((profiles, skipped))
}

/// Same id and content → unchanged; same id, other content → a copy with a new id and name;
/// new id → added. Every added profile gets source `Import`.
pub(crate) fn merge_import(file: &mut ProfilesFile, imported: Vec<Profile>) -> MergeCounts {
    let mut counts = MergeCounts::default();
    for mut profile in imported {
        let same_id = file
            .profiles
            .iter()
            .find(|existing| existing.id == profile.id);
        match same_id {
            Some(existing) if same_content(existing, &profile) => {
                counts.unchanged += 1;
                continue;
            }
            Some(_) => profile.id = new_id(),
            None => {}
        }
        profile.name = unique_name(file, &profile.name);
        profile.source = ProfileSource::Import;
        profile.last_applied = None;
        file.profiles.push(profile);
        counts.added += 1;
    }
    counts
}

fn same_content(a: &Profile, b: &Profile) -> bool {
    a.name == b.name && a.description == b.description && a.mode == b.mode && a.mods == b.mods
}
