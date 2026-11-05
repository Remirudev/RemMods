//! The `profiles.json` model (spec §5.7). Unknown fields round-trip through `extra`.

use std::collections::BTreeMap;

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use super::error::ProfileError;
use crate::safe_fs::names::fold_key;

pub const PROFILES_FILE: &str = "profiles.json";
pub const PROFILES_VERSION: u32 = 1;
pub const SNAPSHOT_KEEP: usize = 10;
pub const PROFILE_NAME_MAX_CHARS: usize = 100;
/// In `extra`, so the file format stays as it was.
const SKIN_SET_FIELD: &str = "skinSet";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesFile {
    pub version: u32,
    pub revision: u64,
    pub profiles: Vec<Profile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_applied: Option<LastApplied>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Default for ProfilesFile {
    fn default() -> Self {
        Self {
            version: PROFILES_VERSION,
            revision: 0,
            profiles: Vec::new(),
            last_applied: None,
            extra: serde_json::Map::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    /// 32 lowercase hex.
    pub id: String,
    /// "" for snapshots.
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub mode: ProfileMode,
    pub mods: BTreeMap<String, ProfileEntry>,
    /// RFC 3339 UTC, millis.
    pub created: String,
    pub updated: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_applied: Option<String>,
    pub source: ProfileSource,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Profile {
    /// The skin set switching this group on brings along.
    pub fn skin_set(&self) -> Option<&str> {
        self.extra.get(SKIN_SET_FIELD)?.as_str()
    }

    pub fn set_skin_set(&mut self, id: Option<&str>) {
        match id {
            Some(id) => {
                self.extra.insert(SKIN_SET_FIELD.to_owned(), id.into());
            }
            None => {
                self.extra.remove(SKIN_SET_FIELD);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProfileMode {
    Additive,
    Exclusive,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProfileEntry {
    pub state: EntryState,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl ProfileEntry {
    pub fn new(state: EntryState) -> Self {
        Self {
            state,
            extra: serde_json::Map::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryState {
    Enabled,
    Disabled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ProfileSource {
    User,
    /// Auto snapshot taken before applying profile `of`.
    Snapshot {
        of: String,
    },
    /// Imported from another mod manager's profiles.json, matched by its `Id`.
    #[serde(rename_all = "camelCase")]
    OtherManager {
        id: String,
        #[serde(default)]
        is_default: bool,
    },
    /// Imported from a RemMods export.
    Import,
}

impl<'de> Deserialize<'de> for ProfileSource {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match StoredSource::deserialize(deserializer)? {
            StoredSource::Current(CurrentSource::User) => Self::User,
            StoredSource::Current(CurrentSource::Snapshot { of }) => Self::Snapshot { of },
            StoredSource::Current(CurrentSource::OtherManager { id, is_default }) => {
                Self::OtherManager { id, is_default }
            }
            StoredSource::Current(CurrentSource::Import) => Self::Import,
            StoredSource::EarlierImport(earlier) => {
                if CURRENT_KINDS.contains(&earlier.kind.as_str()) {
                    return Err(serde::de::Error::custom("malformed profile source"));
                }
                if earlier.id.trim().is_empty() {
                    return Err(serde::de::Error::custom("profile source without an id"));
                }
                Self::OtherManager {
                    id: earlier.id,
                    is_default: earlier.is_default,
                }
            }
        })
    }
}

/// Kind names `CurrentSource` reads; a source carrying one of them that fails to parse is damaged,
/// not an earlier import, and is dropped (the store backs the file up first).
const CURRENT_KINDS: [&str; 4] = ["user", "snapshot", "otherManager", "import"];

#[derive(Deserialize)]
#[serde(untagged)]
enum StoredSource {
    Current(CurrentSource),
    EarlierImport(EarlierImport),
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum CurrentSource {
    User,
    Snapshot {
        of: String,
    },
    #[serde(rename_all = "camelCase")]
    OtherManager {
        id: String,
        #[serde(default)]
        is_default: bool,
    },
    Import,
}

/// A profile imported from another mod manager by an earlier build, stored under another kind name.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EarlierImport {
    kind: String,
    id: String,
    #[serde(default)]
    is_default: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastApplied {
    pub profile_id: String,
    pub at: String,
}

impl Profile {
    pub fn is_snapshot(&self) -> bool {
        matches!(self.source, ProfileSource::Snapshot { .. })
    }

    /// A new profile with a fresh id, created and updated now.
    pub(crate) fn fresh(
        name: String,
        description: String,
        mode: ProfileMode,
        mods: BTreeMap<String, ProfileEntry>,
        source: ProfileSource,
    ) -> Self {
        let now = now_rfc3339();
        Self {
            id: new_id(),
            name,
            description,
            mode,
            mods,
            created: now.clone(),
            updated: now,
            last_applied: None,
            source,
            extra: serde_json::Map::new(),
        }
    }
}

pub(crate) fn new_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

pub(crate) fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub(crate) fn is_profile_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// Trimmed non-empty, ≤ `PROFILE_NAME_MAX_CHARS` chars, unique by fold among other non-snapshot
/// profiles (`except`: the profile being edited).
pub(crate) fn check_name(
    file: &ProfilesFile,
    name: &str,
    except: Option<&str>,
) -> Result<(), ProfileError> {
    if name.trim().is_empty() || name.chars().count() > PROFILE_NAME_MAX_CHARS {
        return Err(ProfileError::InvalidName);
    }
    let fold = fold_key(name);
    let taken = file.profiles.iter().any(|profile| {
        !profile.is_snapshot()
            && Some(profile.id.as_str()) != except
            && fold_key(&profile.name) == fold
    });
    if taken {
        Err(ProfileError::NameTaken)
    } else {
        Ok(())
    }
}

/// `name`, else `name (2)`, `name (3)`, … whose fold no non-snapshot profile has.
pub(crate) fn unique_name(file: &ProfilesFile, name: &str) -> String {
    let taken = |candidate: &str| {
        let fold = fold_key(candidate);
        file.profiles
            .iter()
            .any(|profile| !profile.is_snapshot() && fold_key(&profile.name) == fold)
    };
    if !taken(name) {
        return name.to_owned();
    }
    (2_u32..)
        .map(|n| format!("{name} ({n})"))
        .find(|candidate| !taken(candidate))
        .unwrap_or_else(|| name.to_owned())
}
