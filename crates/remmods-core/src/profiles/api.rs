//! The profiles half of the `Core` facade (spec §5.8): list, save, delete, diff. Apply lives in
//! `apply.rs`, import/export in `api_transfer.rs`.

use std::collections::BTreeMap;

use super::capture::capture_states;
use super::diff::{
    ApplyDirection, DiffSkip, ProfileDiff, diff_profile, diff_switch, switched_together,
};
use super::error::ProfileError;
use super::model::{
    EntryState, LastApplied, Profile, ProfileEntry, ProfileMode, ProfileSource, ProfilesFile,
    check_name, now_rfc3339,
};
use super::store::{LoadedProfiles, StoreProblem};
use crate::mods::{Refreshed, ScanContent};
use crate::safe_fs::Scope;
use crate::skins::LinkedSkins;
use crate::{Core, CoreError};

/// Longest key a picked list may hold, in bytes.
const PICKED_KEY_MAX_BYTES: usize = 4096;
/// Most entries a picked list may hold.
const PICKED_MAX_ENTRIES: usize = 100_000;

/// New: captures the current states; Overwrite: replaces `mods` with the current states, keeps
/// the rest; Edit: name, description, mode only; Picked: a new profile (`id` None) or an edit of
/// a user profile with exactly the listed mods, no scan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProfileSave {
    New {
        name: String,
        description: String,
        mode: ProfileMode,
    },
    Overwrite {
        id: String,
    },
    Edit {
        id: String,
        name: String,
        description: String,
        mode: ProfileMode,
    },
    Picked {
        id: Option<String>,
        name: String,
        description: String,
        mode: ProfileMode,
        mods: Vec<(String, EntryState)>,
        /// The skin set this group brings along.
        skins: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProfileSummary {
    pub profile: Profile,
    pub enabled: u32,
    pub disabled: u32,
    pub missing: u32,
    pub matches_current: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProfilesView {
    pub profiles: Vec<ProfileSummary>,
    pub last_applied: Option<LastApplied>,
    pub read_only: bool,
    pub problem: Option<StoreProblem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplyRequest {
    /// Switched together in one batch, in this order.
    pub profile_ids: Vec<String>,
    pub fingerprint: String,
    pub apply_while_running: bool,
    pub direction: ApplyDirection,
}

#[derive(Clone, Debug)]
pub struct ApplyOutcome {
    pub refreshed: Refreshed,
    pub changed: u32,
    pub snapshot_id: Option<String>,
    pub skipped: Vec<DiffSkip>,
    /// What the group's linked skin set did; `None` without a link or with nothing to do.
    pub skins: Option<LinkedSkins>,
}

impl Core {
    pub fn profiles_list(&self) -> Result<ProfilesView, CoreError> {
        let loaded = self.profiles.load();
        let snapshot = match self.mods_list() {
            Ok(snapshot) => Some(snapshot),
            Err(error) => {
                log::debug!("profiles listed without a scan: {error}");
                None
            }
        };
        Ok(build_view(
            loaded,
            snapshot.as_ref().map(|snapshot| &snapshot.content),
        ))
    }

    pub fn profiles_save(&self, save: &ProfileSave) -> Result<ProfilesView, CoreError> {
        let states = match save {
            ProfileSave::Edit { .. } | ProfileSave::Picked { .. } => None,
            ProfileSave::New { .. } | ProfileSave::Overwrite { .. } => {
                Some(capture_states(&self.mods_refresh()?.snapshot.content))
            }
        };
        {
            let guard = self.ops.write(self.app_scope()?)?;
            self.profiles
                .update(&guard, |file| apply_save(file, save, states))?;
        }
        self.profiles_list()
    }

    pub fn profiles_delete(&self, id: &str) -> Result<ProfilesView, CoreError> {
        {
            let guard = self.ops.write(self.app_scope()?)?;
            self.profiles.update(&guard, |file| {
                let index = position(file, id)?;
                file.profiles.remove(index);
                Ok(())
            })?;
        }
        self.profiles_list()
    }

    pub fn profiles_diff(
        &self,
        ids: &[String],
        direction: ApplyDirection,
    ) -> Result<ProfileDiff, CoreError> {
        let fresh = self.mods_refresh()?;
        let loaded = self.profiles.load();
        let profile = switched_profile(&loaded.file, ids)?;
        Ok(diff_switch(&profile, &fresh.snapshot.content, direction))
    }

    /// RemMods' own folders only (store writes that need no game).
    pub(crate) fn app_scope(&self) -> Result<Scope, CoreError> {
        let app_dirs = [self.paths.data_dir.clone(), self.paths.config_dir.clone()];
        Ok(Scope::from_roots(&[], &app_dirs)?)
    }
}

pub(crate) fn position(file: &ProfilesFile, id: &str) -> Result<usize, ProfileError> {
    file.profiles
        .iter()
        .position(|profile| profile.id == id)
        .ok_or(ProfileError::NotFound)
}

/// The profile a switch of `ids` applies: one or several, never none.
pub(crate) fn switched_profile(
    file: &ProfilesFile,
    ids: &[String],
) -> Result<Profile, ProfileError> {
    let picked = ids
        .iter()
        .map(|id| position(file, id).map(|index| &file.profiles[index]))
        .collect::<Result<Vec<_>, _>>()?;
    switched_together(&picked).ok_or(ProfileError::NotFound)
}

fn apply_save(
    file: &mut ProfilesFile,
    save: &ProfileSave,
    states: Option<BTreeMap<String, ProfileEntry>>,
) -> Result<(), ProfileError> {
    let states = states.unwrap_or_default();
    match save {
        ProfileSave::New {
            name,
            description,
            mode,
        } => {
            check_name(file, name, None)?;
            let profile = Profile::fresh(
                name.clone(),
                description.clone(),
                *mode,
                states,
                ProfileSource::User,
            );
            file.profiles.push(profile);
        }
        ProfileSave::Overwrite { id } => {
            let index = position(file, id)?;
            let profile = &mut file.profiles[index];
            profile.mods = states;
            profile.updated = now_rfc3339();
        }
        ProfileSave::Edit {
            id,
            name,
            description,
            mode,
        } => {
            let index = position(file, id)?;
            if !file.profiles[index].is_snapshot() {
                check_name(file, name, Some(id))?;
            }
            let profile = &mut file.profiles[index];
            profile.name.clone_from(name);
            profile.description.clone_from(description);
            profile.mode = *mode;
            profile.updated = now_rfc3339();
        }
        ProfileSave::Picked {
            id: None,
            name,
            description,
            mode,
            mods,
            skins,
        } => {
            check_name(file, name, None)?;
            let mut profile = Profile::fresh(
                name.clone(),
                description.clone(),
                *mode,
                picked_mods(mods, None)?,
                ProfileSource::User,
            );
            profile.set_skin_set(skins.as_deref());
            file.profiles.push(profile);
        }
        ProfileSave::Picked {
            id: Some(id),
            name,
            description,
            mode,
            mods,
            skins,
        } => {
            let index = position(file, id)?;
            // Snapshots only ever hold a captured state, so a picked list never replaces one.
            if file.profiles[index].is_snapshot() {
                return Err(ProfileError::NotFound);
            }
            check_name(file, name, Some(id))?;
            let mods = picked_mods(mods, Some(&file.profiles[index].mods))?;
            let profile = &mut file.profiles[index];
            profile.set_skin_set(skins.as_deref());
            profile.name.clone_from(name);
            profile.description.clone_from(description);
            profile.mode = *mode;
            profile.mods = mods;
            profile.updated = now_rfc3339();
        }
    }
    Ok(())
}

/// The map of a picked list; unknown fields of entries the profile already had survive. Empty,
/// overlong, control-character or repeated keys refuse the whole list.
fn picked_mods(
    entries: &[(String, EntryState)],
    previous: Option<&BTreeMap<String, ProfileEntry>>,
) -> Result<BTreeMap<String, ProfileEntry>, ProfileError> {
    if entries.len() > PICKED_MAX_ENTRIES {
        return Err(ProfileError::InvalidEntries);
    }
    let mut mods = BTreeMap::new();
    for (key, state) in entries {
        if key.is_empty() || key.len() > PICKED_KEY_MAX_BYTES || key.chars().any(char::is_control) {
            return Err(ProfileError::InvalidEntries);
        }
        let mut entry = previous
            .and_then(|map| map.get(key))
            .cloned()
            .unwrap_or_else(|| ProfileEntry::new(*state));
        entry.state = *state;
        if mods.insert(key.clone(), entry).is_some() {
            return Err(ProfileError::InvalidEntries);
        }
    }
    Ok(mods)
}

pub(crate) fn build_view(loaded: LoadedProfiles, content: Option<&ScanContent>) -> ProfilesView {
    let profiles = loaded
        .file
        .profiles
        .into_iter()
        .map(|profile| summarize(profile, content))
        .collect();
    ProfilesView {
        profiles,
        last_applied: loaded.file.last_applied,
        read_only: loaded.read_only,
        problem: loaded.problem,
    }
}

fn summarize(profile: Profile, content: Option<&ScanContent>) -> ProfileSummary {
    let count = |state: EntryState| {
        let n = profile
            .mods
            .values()
            .filter(|entry| entry.state == state)
            .count();
        u32::try_from(n).unwrap_or(u32::MAX)
    };
    let enabled = count(EntryState::Enabled);
    let disabled = count(EntryState::Disabled);
    let (missing, matches_current) = match content {
        Some(content) => {
            let diff = diff_profile(&profile, content);
            let missing = u32::try_from(diff.missing.len()).unwrap_or(u32::MAX);
            (missing, diff.changes.is_empty() && diff.skipped.is_empty())
        }
        None => (0, false),
    };
    ProfileSummary {
        profile,
        enabled,
        disabled,
        missing,
        matches_current,
    }
}
