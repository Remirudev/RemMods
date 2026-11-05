//! Profile capture (every toggleable row, Enabled and Disabled) and rename rekeying of RemMods' profiles.
#![allow(clippy::expect_used)]

mod mods_support;
mod profiles_support;
mod support;

use mods_support::DATA;
use mods_support::manage::{ToggleFixture, core_with_trash};
use profiles_support::{by_name, save};
use remmods_core::CoreError;
use remmods_core::mods::RenameRequest;
use remmods_core::profiles::{EntryState, ProfileError, ProfileMode, ProfileSave, ProfilesView};

fn rename_row(fixture: &ToggleFixture, key: &str, new_name: &str) {
    fixture
        .core
        .mods_rename(&RenameRequest {
            target: fixture.target(key),
            new_name: new_name.to_owned(),
            apply_while_running: false,
        })
        .expect("rename");
}

fn entry_keys(view: &ProfilesView, profile: &str) -> Vec<String> {
    by_name(view, profile).mods.keys().cloned().collect()
}

fn listed(fixture: &ToggleFixture, profile: &str) -> Vec<String> {
    entry_keys(&fixture.core.profiles_list().expect("profiles"), profile)
}

// Rename rekeying.

/// Three profiles with different modes and states: the rename rekeys one entry in each, keeps
/// every other entry and its state, and the result is on disk.
#[test]
fn rename_rekeys_every_profile_and_persists() {
    let fixture = core_with_trash(&[
        ("Mods/Kazuya/kazuya.pak", DATA),
        ("Mods/Jin/jin.pak-x", DATA),
        ("Mods/Lili/lili.pak", DATA),
    ]);
    save(&fixture, "Main", ProfileMode::Exclusive);
    save(&fixture, "Ranked", ProfileMode::Additive);
    fixture
        .toggle(&["Standard:Lili"], false)
        .expect("disable Lili");
    save(&fixture, "Casual", ProfileMode::Exclusive);
    rename_row(&fixture, "Standard:Kazuya", "Kazuya Alt");
    let view = fixture
        .second_core()
        .profiles_list()
        .expect("profiles after reopen");
    for (profile, mode, lili) in [
        ("Main", ProfileMode::Exclusive, EntryState::Enabled),
        ("Ranked", ProfileMode::Additive, EntryState::Enabled),
        ("Casual", ProfileMode::Exclusive, EntryState::Disabled),
    ] {
        assert_eq!(
            entry_keys(&view, profile),
            ["Standard:Jin", "Standard:Kazuya Alt", "Standard:Lili"],
            "{profile}"
        );
        let saved = by_name(&view, profile);
        assert_eq!(saved.mode, mode, "{profile}");
        let state = |key: &str| saved.mods[key].state;
        assert_eq!(
            state("Standard:Kazuya Alt"),
            EntryState::Enabled,
            "{profile}"
        );
        assert_eq!(state("Standard:Jin"), EntryState::Disabled, "{profile}");
        assert_eq!(state("Standard:Lili"), lili, "{profile}");
    }
}

#[test]
fn rename_touches_only_the_renamed_root() {
    let fixture = core_with_trash(&[("Mods/Jin/jin.pak", DATA), ("~mods/Jin/jin_old.pak", DATA)]);
    save(&fixture, "Main", ProfileMode::Exclusive);
    rename_row(&fixture, "Standard:Jin", "Jin Alt");
    assert_eq!(listed(&fixture, "Main"), ["Legacy:Jin", "Standard:Jin Alt"]);
}

// Capture.

/// Disabled rows are stored as Disabled, so an additive apply can turn them off too.
#[test]
fn capture_lists_enabled_and_disabled_rows() {
    let fixture = core_with_trash(&[
        ("Mods/Kazuya/kazuya.pak", DATA),
        ("Mods/Jin/jin.pak-x", DATA),
        ("Mods/Lili/lili.pak", DATA),
    ]);
    let id = save(&fixture, "Main", ProfileMode::Additive);
    let view = fixture.core.profiles_list().expect("profiles");
    let summary = view
        .profiles
        .iter()
        .find(|summary| summary.profile.id == id)
        .expect("saved profile");
    let state = |key: &str| summary.profile.mods[key].state;
    assert_eq!(state("Standard:Kazuya"), EntryState::Enabled);
    assert_eq!(state("Standard:Lili"), EntryState::Enabled);
    assert_eq!(state("Standard:Jin"), EntryState::Disabled);
    assert_eq!((summary.enabled, summary.disabled), (2, 1));
}

#[test]
fn capture_keeps_same_name_in_different_roots_apart() {
    let fixture = core_with_trash(&[
        ("Mods/Jin/jin.pak", DATA),
        ("LogicMods/Jin/jin_menu.pak", DATA),
    ]);
    save(&fixture, "Main", ProfileMode::Exclusive);
    assert_eq!(listed(&fixture, "Main"), ["Logic:Jin", "Standard:Jin"]);
}

// Deleting and editing.

#[test]
fn deleting_a_profile_removes_only_it() {
    let fixture = core_with_trash(&[("Mods/Kazuya/kazuya.pak", DATA)]);
    let main = save(&fixture, "Main", ProfileMode::Exclusive);
    let ranked = save(&fixture, "Ranked", ProfileMode::Exclusive);
    let view = fixture.core.profiles_delete(&main).expect("delete");
    let ids: Vec<&str> = view
        .profiles
        .iter()
        .map(|summary| summary.profile.id.as_str())
        .collect();
    assert_eq!(ids, [ranked.as_str()]);
    assert!(matches!(
        fixture.core.profiles_delete(&main),
        Err(CoreError::Profiles(ProfileError::NotFound))
    ));
}

#[test]
fn edited_name_and_mode_survive_reopen() {
    let fixture = core_with_trash(&[("Mods/Kazuya/kazuya.pak", DATA)]);
    let id = save(&fixture, "Ranked", ProfileMode::Exclusive);
    fixture
        .core
        .profiles_save(&ProfileSave::Edit {
            id: id.clone(),
            name: "Ranked Set".into(),
            description: "season picks".into(),
            mode: ProfileMode::Additive,
        })
        .expect("edit");
    let view = fixture.second_core().profiles_list().expect("profiles");
    let profile = by_name(&view, "Ranked Set");
    assert_eq!(profile.id, id);
    assert_eq!(profile.mode, ProfileMode::Additive);
    assert_eq!(profile.description, "season picks");
}
