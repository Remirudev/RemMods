//! RemMods' `profiles.json` store (spec §5.7, §8.2 S1–S5).
#![allow(clippy::expect_used, clippy::disallowed_methods)]

mod mods_support;
mod support;

use mods_support::DATA;
use mods_support::manage::{ToggleFixture, core_with_trash};
use remmods_core::CoreError;
use remmods_core::profiles::{
    PROFILE_NAME_MAX_CHARS, ProfileError, ProfileMode, ProfileSave, ProfileStore, ProfilesView,
    StoreProblem,
};
use remmods_core::safe_fs::Scope;

fn new_profile(fixture: &ToggleFixture, name: &str) -> Result<ProfilesView, CoreError> {
    fixture.core.profiles_save(&ProfileSave::New {
        name: name.to_owned(),
        description: String::new(),
        mode: ProfileMode::Exclusive,
    })
}

fn store_file(fixture: &ToggleFixture) -> std::path::PathBuf {
    fixture.paths.profiles_file()
}

fn fixture() -> ToggleFixture {
    core_with_trash(&[("Mods/A/A.pak", DATA)])
}

#[test]
fn corrupt_store_backed_up_before_write() {
    let fixture = fixture();
    std::fs::write(store_file(&fixture), b"{ bad").expect("write");
    let loaded = fixture.core.profiles.load();
    assert_eq!(loaded.problem, Some(StoreProblem::Corrupt));
    assert!(!loaded.read_only);
    new_profile(&fixture, "P").expect("save");
    let backups: Vec<String> = support::names_in(&fixture.paths.data_dir)
        .into_iter()
        .filter(|name| name.starts_with("profiles.json.backup."))
        .collect();
    assert_eq!(backups.len(), 1);
    let backup = std::fs::read(fixture.paths.data_dir.join(&backups[0])).expect("backup");
    assert_eq!(backup, b"{ bad");
    let loaded = fixture.core.profiles.load();
    assert_eq!(loaded.problem, None);
    assert_eq!(loaded.file.profiles.len(), 1);
}

#[test]
fn newer_version_read_only_refuses_writes() {
    let fixture = fixture();
    let bytes = br#"{"version":2,"revision":7,"profiles":[],"novel":true}"#;
    std::fs::write(store_file(&fixture), bytes).expect("write");
    let view = fixture.core.profiles_list().expect("list");
    assert!(view.read_only);
    assert_eq!(view.problem, Some(StoreProblem::NewerVersion));
    let refused = new_profile(&fixture, "P");
    assert!(matches!(
        refused,
        Err(CoreError::Profiles(ProfileError::ReadOnly))
    ));
    assert_eq!(std::fs::read(store_file(&fixture)).expect("read"), bytes);
}

#[test]
fn unknown_fields_round_trip() {
    let fixture = fixture();
    let text = r#"{"version":1,"revision":1,"future":{"a":1},"profiles":[{"id":"0123456789abcdef0123456789abcdef","name":"Old","mode":"additive","mods":{"Standard:A":{"state":"enabled","pin":true}},"created":"2026-01-01T00:00:00.000Z","updated":"2026-01-01T00:00:00.000Z","source":{"kind":"user"},"color":"red"}]}"#;
    std::fs::write(store_file(&fixture), text).expect("write");
    new_profile(&fixture, "New").expect("save");
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(store_file(&fixture)).expect("read")).expect("json");
    assert_eq!(value["future"]["a"], 1);
    assert_eq!(value["revision"], 2);
    let old = &value["profiles"][0];
    assert_eq!(old["color"], "red");
    assert_eq!(old["mods"]["Standard:A"]["pin"], true);
    assert_eq!(value["profiles"].as_array().expect("profiles").len(), 2);
}

#[test]
fn name_rules() {
    let fixture = fixture();
    for name in ["", "   ", &"x".repeat(PROFILE_NAME_MAX_CHARS + 1)] {
        let refused = new_profile(&fixture, name);
        assert!(
            matches!(refused, Err(CoreError::Profiles(ProfileError::InvalidName))),
            "{name:?}"
        );
    }
    new_profile(&fixture, &"\u{e9}".repeat(PROFILE_NAME_MAX_CHARS)).expect("100 chars");
    new_profile(&fixture, "Stra\u{df}e").expect("first");
    let taken = new_profile(&fixture, "STRASSE");
    assert!(matches!(
        taken,
        Err(CoreError::Profiles(ProfileError::NameTaken))
    ));
}

#[test]
fn revision_increments_and_rereads() {
    let fixture = fixture();
    let first = ProfileStore::new(store_file(&fixture));
    let second = ProfileStore::new(store_file(&fixture));
    let app_dirs = [
        fixture.paths.data_dir.clone(),
        fixture.paths.config_dir.clone(),
    ];
    let scope = || Scope::from_roots(&[], &app_dirs).expect("scope");
    for (store, key) in [(&first, "one"), (&second, "two")] {
        let guard = fixture.core.ops.write(scope()).expect("guard");
        store
            .update(&guard, |file| {
                file.extra.insert(key.to_owned(), serde_json::json!(true));
                Ok(())
            })
            .expect("update");
    }
    let loaded = first.load();
    assert_eq!(loaded.file.revision, 2);
    assert!(loaded.file.extra.contains_key("one") && loaded.file.extra.contains_key("two"));
}
