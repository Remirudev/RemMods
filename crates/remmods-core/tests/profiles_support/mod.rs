//! Helpers for the profile tests (spec §8.2): save, find and apply profiles on a fixture core.
#![allow(dead_code, clippy::expect_used, clippy::disallowed_methods)]

use remmods_core::CoreError;
use remmods_core::mods::ModState;
use remmods_core::profiles::{
    ApplyOutcome, ApplyRequest, Profile, ProfileMode, ProfileSave, ProfilesView,
};

use crate::mods_support::toggle::ToggleFixture;

pub fn save(fixture: &ToggleFixture, name: &str, mode: ProfileMode) -> String {
    let view = fixture
        .core
        .profiles_save(&ProfileSave::New {
            name: name.to_owned(),
            description: String::new(),
            mode,
        })
        .expect("save profile");
    by_name(&view, name).id.clone()
}

pub fn by_name<'a>(view: &'a ProfilesView, name: &str) -> &'a Profile {
    &view
        .profiles
        .iter()
        .find(|summary| summary.profile.name == name)
        .unwrap_or_else(|| panic!("no profile {name}"))
        .profile
}

pub fn snapshots(fixture: &ToggleFixture) -> Vec<Profile> {
    fixture
        .core
        .profiles_list()
        .expect("profiles")
        .profiles
        .into_iter()
        .map(|summary| summary.profile)
        .filter(Profile::is_snapshot)
        .collect()
}

/// Diff, then apply with the diff's fingerprint.
pub fn apply(fixture: &ToggleFixture, id: &str) -> Result<ApplyOutcome, CoreError> {
    let diff = fixture.core.profiles_diff(id).expect("diff");
    fixture.core.profiles_apply(&ApplyRequest {
        profile_id: id.to_owned(),
        fingerprint: diff.fingerprint,
        apply_while_running: false,
    })
}

pub fn state(fixture: &ToggleFixture, key: &str) -> ModState {
    fixture.row(key).state
}

/// Appends a profile listing exactly `entries` to the fixture's `profiles.json`; returns its id.
pub fn custom(
    fixture: &ToggleFixture,
    name: &str,
    mode: ProfileMode,
    entries: &[(&str, remmods_core::profiles::EntryState)],
) -> String {
    let store = remmods_core::profiles::ProfileStore::new(fixture.paths.profiles_file());
    let mut file = store.load().file;
    let id = format!("{:032x}", file.profiles.len() + 0xabc);
    let mods = entries
        .iter()
        .map(|(key, state)| {
            (
                (*key).to_owned(),
                remmods_core::profiles::ProfileEntry::new(*state),
            )
        })
        .collect();
    file.profiles.push(Profile {
        id: id.clone(),
        name: name.to_owned(),
        description: String::new(),
        mode,
        mods,
        created: "2026-01-01T00:00:00.000Z".to_owned(),
        updated: "2026-01-01T00:00:00.000Z".to_owned(),
        last_applied: None,
        source: remmods_core::profiles::ProfileSource::User,
        extra: serde_json::Map::new(),
    });
    let bytes = serde_json::to_vec_pretty(&file).expect("encode");
    std::fs::create_dir_all(&fixture.paths.data_dir).expect("data dir");
    std::fs::write(fixture.paths.profiles_file(), bytes).expect("write profiles");
    id
}
