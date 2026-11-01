//! The profiles half of the `Core` facade (spec §5.8): list, save, delete, diff. Apply lives in
//! `apply.rs`, import/export in `api_transfer.rs`.

use std::collections::BTreeMap;

use super::capture::capture_states;
use super::diff::{DiffSkip, ProfileDiff, diff_profile};
use super::error::ProfileError;
use super::model::{
    EntryState, LastApplied, Profile, ProfileEntry, ProfileMode, ProfileSource, ProfilesFile,
    check_name, now_rfc3339,
};
use super::store::{LoadedProfiles, StoreProblem};
use crate::mods::{Refreshed, ScanContent};
use crate::safe_fs::Scope;
use crate::{Core, CoreError};

/// New: captures the current states; Overwrite: replaces `mods` with the current states, keeps
/// the rest; Edit: name, description, mode only.
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
    pub profile_id: String,
    pub fingerprint: String,
    pub apply_while_running: bool,
}

#[derive(Clone, Debug)]
pub struct ApplyOutcome {
    pub refreshed: Refreshed,
    pub changed: u32,
    pub snapshot_id: Option<String>,
    pub skipped: Vec<DiffSkip>,
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
            ProfileSave::Edit { .. } => None,
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

    pub fn profiles_diff(&self, id: &str) -> Result<ProfileDiff, CoreError> {
        let fresh = self.mods_refresh()?;
        let loaded = self.profiles.load();
        let profile = loaded
            .file
            .profiles
            .iter()
            .find(|profile| profile.id == id)
            .ok_or(ProfileError::NotFound)?;
        Ok(diff_profile(profile, &fresh.snapshot.content))
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
    }
    Ok(())
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
