//! The original's `ProfileServiceTests` (PLAN §9), mapped to RemMods semantics: captures list
//! every toggleable row (Enabled and Disabled), renames rekey RemMods' profiles only.
#![allow(clippy::expect_used)]

mod mods_support;
mod profiles_support;
mod support;

use mods_support::DATA;
use mods_support::manage::{ToggleFixture, core_with_trash};
use profiles_support::{by_name, save};
use remmods_core::CoreError;
use remmods_core::mods::RenameRequest;
use remmods_core::profiles::{EntryState, ProfileError, ProfileMode, ProfileSave};

fn rename(fixture: &ToggleFixture, key: &str, new_name: &str) {
    fixture
        .core
        .mods_rename(&RenameRequest {
            target: fixture.target(key),
            new_name: new_name.to_owned(),
            apply_while_running: false,
        })
        .expect("rename");
}

fn keys(fixture: &ToggleFixture, name: &str) -> Vec<String> {
    let view = fixture.core.profiles_list().expect("profiles");
    by_name(&view, name).mods.keys().cloned().collect()
}

// ORIG: RenameModInProfiles_UpdatesMatchingEntries
#[test]
fn rename_updates_matching_entries() {
    let fixture = core_with_trash(&[("Mods/Cool/Cool.pak", DATA)]);
    save(&fixture, "P", ProfileMode::Exclusive);
    rename(&fixture, "Standard:Cool", "Hot");
    assert_eq!(keys(&fixture, "P"), ["Standard:Hot"]);
}

// ORIG: RenameModInProfiles_UpdatesAcrossMultipleProfiles
#[test]
fn rename_updates_across_multiple_profiles() {
    let fixture = core_with_trash(&[("Mods/Cool/Cool.pak", DATA)]);
    save(&fixture, "P", ProfileMode::Exclusive);
    save(&fixture, "Q", ProfileMode::Additive);
    rename(&fixture, "Standard:Cool", "Hot");
    assert_eq!(keys(&fixture, "P"), ["Standard:Hot"]);
    assert_eq!(keys(&fixture, "Q"), ["Standard:Hot"]);
}

// ORIG: RenameModInProfiles_LeavesNonMatchingEntriesUntouched
#[test]
fn rename_leaves_non_matching_entries_untouched() {
    let fixture = core_with_trash(&[("Mods/Cool/Cool.pak", DATA), ("Mods/Other/O.pak", DATA)]);
    save(&fixture, "P", ProfileMode::Exclusive);
    rename(&fixture, "Standard:Cool", "Hot");
    assert_eq!(keys(&fixture, "P"), ["Standard:Hot", "Standard:Other"]);
}

// ORIG: RenameModInProfiles_Persists
#[test]
fn rename_persists() {
    let fixture = core_with_trash(&[("Mods/Cool/Cool.pak", DATA)]);
    save(&fixture, "P", ProfileMode::Exclusive);
    rename(&fixture, "Standard:Cool", "Hot");
    let reopened = fixture.second_core();
    let view = reopened.profiles_list().expect("profiles");
    let listed: Vec<&String> = by_name(&view, "P").mods.keys().collect();
    assert_eq!(listed, ["Standard:Hot"]);
}

// ORIG: RenameModInProfiles_SameName_OnlyAffectsTargetedRoot
#[test]
fn rename_same_name_only_affects_targeted_root() {
    let fixture = core_with_trash(&[("Mods/Cool/a.pak", DATA), ("~mods/Cool/b.pak", DATA)]);
    save(&fixture, "P", ProfileMode::Exclusive);
    rename(&fixture, "Standard:Cool", "Hot");
    assert_eq!(keys(&fixture, "P"), ["Legacy:Cool", "Standard:Hot"]);
}

// ORIG: CreateFromCurrentState_OnlySavesEnabledMods. Documented difference (Appendix A10):
// RemMods lists disabled rows too, as Disabled, so additive applies can turn them off.
#[test]
fn create_from_current_state_lists_enabled_and_disabled() {
    let fixture = core_with_trash(&[("Mods/On/a.pak", DATA), ("Mods/Off/b.pak-x", DATA)]);
    let id = save(&fixture, "P", ProfileMode::Exclusive);
    let view = fixture.core.profiles_list().expect("profiles");
    let profile = view
        .profiles
        .iter()
        .find(|summary| summary.profile.id == id)
        .expect("profile");
    assert_eq!(
        profile.profile.mods["Standard:On"].state,
        EntryState::Enabled
    );
    assert_eq!(
        profile.profile.mods["Standard:Off"].state,
        EntryState::Disabled
    );
    assert_eq!((profile.enabled, profile.disabled), (1, 1));
}

// ORIG: CreateFromCurrentState_SameName_DifferentRoots_StoredSeparately
#[test]
fn same_name_different_roots_stored_separately() {
    let fixture = core_with_trash(&[("Mods/Cool/a.pak", DATA), ("LogicMods/Cool/b.pak", DATA)]);
    save(&fixture, "P", ProfileMode::Exclusive);
    assert_eq!(keys(&fixture, "P"), ["Logic:Cool", "Standard:Cool"]);
}

// ORIG: DeleteProfile_RemovesNonDefaultProfile (RemMods has no undeletable default profile)
#[test]
fn delete_profile_removes_it() {
    let fixture = core_with_trash(&[("Mods/Cool/a.pak", DATA)]);
    let id = save(&fixture, "P", ProfileMode::Exclusive);
    save(&fixture, "Q", ProfileMode::Exclusive);
    let view = fixture.core.profiles_delete(&id).expect("delete");
    assert_eq!(view.profiles.len(), 1);
    assert!(matches!(
        fixture.core.profiles_delete(&id),
        Err(CoreError::Profiles(ProfileError::NotFound))
    ));
}

// ORIG: UpdateProfile_PersistsNameChange
#[test]
fn update_profile_persists_name_change() {
    let fixture = core_with_trash(&[("Mods/Cool/a.pak", DATA)]);
    let id = save(&fixture, "P", ProfileMode::Exclusive);
    fixture
        .core
        .profiles_save(&ProfileSave::Edit {
            id: id.clone(),
            name: "Renamed".into(),
            description: "d".into(),
            mode: ProfileMode::Additive,
        })
        .expect("edit");
    let view = fixture.second_core().profiles_list().expect("profiles");
    let profile = by_name(&view, "Renamed");
    assert_eq!(profile.id, id);
    assert_eq!(profile.mode, ProfileMode::Additive);
}
