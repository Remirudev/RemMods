//! `Core::mods_rename` and `plan_rename` (spec §5.2, §8.2 N1–N10, N12). Every refusal leaves the
//! disk unchanged.
#![allow(clippy::expect_used, clippy::disallowed_methods)]

mod mods_support;
mod support;

use std::ffi::OsString;
use std::sync::atomic::Ordering;

use mods_support::DATA;
use mods_support::manage::{HEX, core_with_trash, mods_error, restart, tree};
use remmods_core::mods::{ManageBlock, ModsError, RenameOutcome, RenameRequest, plan_rename};
use remmods_core::profiles::{ProfileMode, ProfileSave};
use remmods_core::safe_fs::FaultPlan;
use remmods_core::{CoreError, mods::ModTarget};

fn rename(
    fixture: &mods_support::manage::ToggleFixture,
    key: &str,
    new_name: &str,
) -> Result<RenameOutcome, CoreError> {
    fixture.core.mods_rename(&RenameRequest {
        target: fixture.target(key),
        new_name: new_name.to_owned(),
        apply_while_running: false,
    })
}

#[test]
fn rename_invalid_names_change_nothing() {
    let fixture = core_with_trash(&[("Mods/Cool/Cool.pak", DATA)]);
    let before = tree(&fixture);
    let long = "a".repeat(256);
    let temp = format!("_x_tmp_{HEX}");
    let cases: [(&str, &str); 9] = [
        ("CON", "reserved_device_name"),
        ("com\u{B9}", "reserved_device_name"),
        ("a.", "trailing_dot_or_space"),
        ("a ", "trailing_dot_or_space"),
        ("a/b", "forbidden_char"),
        (".disabled_a", "disabled_prefix"),
        (&temp, "temp_name"),
        (&long, "too_long"),
        ("a\u{202E}b", "bidi_control"),
    ];
    for (name, expected) in cases {
        match mods_error(rename(&fixture, "Standard:Cool", name)) {
            ModsError::InvalidName { code } => assert_eq!(code, expected, "{name:?}"),
            other => panic!("{name:?}: expected InvalidName, got {other:?}"),
        }
    }
    assert_eq!(tree(&fixture), before);
}

#[test]
fn rename_collision_folded_changes_nothing() {
    let fixture = core_with_trash(&[
        ("Mods/Cool/Cool.pak", DATA),
        ("Mods/COOL2/a.pak", DATA),
        ("LogicMods/X/X.pak", DATA),
        ("LogicMods/.disabled_Y/Y.pak-x", DATA),
        ("mods/Y/y.pak", DATA),
    ]);
    let before = tree(&fixture);
    for (key, name) in [
        ("Standard:Cool", "cool2"),
        ("Logic:X", "Y"),
        ("Standard:Cool", "Y"),
    ] {
        assert!(
            matches!(
                mods_error(rename(&fixture, key, name)),
                ModsError::TargetExists
            ),
            "{key} → {name}"
        );
    }
    assert_eq!(tree(&fixture), before);
}

#[test]
fn rename_unchanged_refused() {
    let fixture = core_with_trash(&[("Mods/Cool/Cool.pak", DATA)]);
    let before = tree(&fixture);
    assert!(matches!(
        mods_error(rename(&fixture, "Standard:Cool", "Cool")),
        ModsError::RenameUnchanged
    ));
    assert_eq!(tree(&fixture), before);
}

#[test]
fn rename_refused_for_loose_linked() {
    let fixture = core_with_trash(&[("Mods/Loose.pak", DATA)]);
    let before = tree(&fixture);
    match mods_error(rename(&fixture, "Loose:Mods/Loose", "Other")) {
        ModsError::ManageBlocked { reason, .. } => assert_eq!(reason, ManageBlock::Loose),
        other => panic!("expected ManageBlocked, got {other:?}"),
    }
    assert_eq!(tree(&fixture), before);
}

#[test]
fn rename_while_running_needs_flag() {
    let fixture = core_with_trash(&[("Mods/Cool/Cool.pak", DATA)]);
    let before = tree(&fixture);
    fixture.running.store(true, Ordering::SeqCst);
    assert!(matches!(
        mods_error(rename(&fixture, "Standard:Cool", "Hot")),
        ModsError::GameRunning
    ));
    assert_eq!(tree(&fixture), before);
    let outcome = fixture
        .core
        .mods_rename(&RenameRequest {
            target: fixture.target("Standard:Cool"),
            new_name: "Hot".to_owned(),
            apply_while_running: true,
        })
        .expect("forced rename");
    assert_eq!(outcome.new_key, "Standard:Hot");
    assert!(fixture.exists("Mods/Hot/Cool.pak"));
}

#[test]
fn rename_batch_failure_rolls_back() {
    let fixture = core_with_trash(&[("Mods/Cool/Cool.pak", DATA)]);
    let before = tree(&fixture);
    fixture.core.inject_batch_fault(FaultPlan {
        fail_before: Some(0),
        ..FaultPlan::default()
    });
    assert!(rename(&fixture, "Standard:Cool", "Hot").is_err());
    assert_eq!(tree(&fixture), before);
    assert!(fixture.exists("Mods/Cool"));
}

#[test]
fn rename_case_only_goes_through_case_tmp() {
    let mut fixture = core_with_trash(&[("Mods/cool/Cool.pak", DATA)]);
    let before = tree(&fixture);
    let outcome = rename(&fixture, "Standard:cool", "Cool").expect("case-only rename");
    assert_eq!(outcome.new_key, "Standard:Cool");
    assert!(outcome.new_ref.is_some());
    assert_eq!(fixture.row("Standard:Cool").name, "Cool");
    rename(&fixture, "Standard:Cool", "cool").expect("back");
    assert_eq!(tree(&fixture), before);
    fixture.core.inject_batch_fault(FaultPlan {
        crash_after: Some(1),
        ..FaultPlan::default()
    });
    assert!(rename(&fixture, "Standard:cool", "Cool").is_err());
    restart(&mut fixture);
    assert_eq!(tree(&fixture), before);
}

#[test]
fn rename_logic_prefixed_keeps_prefix() {
    let fixture = core_with_trash(&[("LogicMods/.disabled_X/X.pak-x", DATA)]);
    let outcome = rename(&fixture, "Logic:X", "Y").expect("rename");
    assert_eq!(outcome.old_key, "Logic:X");
    assert_eq!(outcome.new_key, "Logic:Y");
    assert!(fixture.exists("LogicMods/.disabled_Y/X.pak-x"));
    assert!(!fixture.exists("LogicMods/.disabled_X"));
}

fn save_profile(fixture: &mods_support::manage::ToggleFixture, name: &str) {
    let save = ProfileSave::New {
        name: name.to_owned(),
        description: String::new(),
        mode: ProfileMode::Additive,
    };
    fixture.core.profiles_save(&save).expect("save profile");
}

#[test]
fn rename_rekeys_profiles_and_meta() {
    let fixture = core_with_trash(&[("Mods/Cool/Cool.pak", DATA), ("Mods/Keep/Keep.pak", DATA)]);
    fixture.toggle(&["Standard:Cool"], false).expect("disable");
    save_profile(&fixture, "One");
    save_profile(&fixture, "Two");
    let outcome = rename(&fixture, "Standard:Cool", "Hot").expect("rename");
    assert_eq!(outcome.rekey.profiles, 2);
    assert!(outcome.rekey.meta);
    assert!(!outcome.rekey.failed && !outcome.rekey.skipped_ambiguous);
    let view = fixture.core.profiles_list().expect("profiles");
    for summary in &view.profiles {
        let keys: Vec<&str> = summary.profile.mods.keys().map(String::as_str).collect();
        assert_eq!(keys, ["Standard:Hot", "Standard:Keep"]);
    }
    let meta = std::fs::read_to_string(fixture.paths.data_dir.join("mod-meta.json")).expect("meta");
    assert!(
        meta.contains("Standard:Hot") && !meta.contains("Standard:Cool"),
        "{meta}"
    );
    fixture
        .toggle(&["Standard:Hot"], true)
        .expect("enable keeps the record");
    assert!(fixture.exists("Mods/Hot/Cool.pak"));
}

#[test]
fn rename_ambiguous_does_not_rekey() {
    let fixture = core_with_trash(&[
        ("LogicMods/X/X.pak", DATA),
        ("LogicMods/.disabled_X/X.pak-x", DATA),
        ("Mods/Keep/Keep.pak", DATA),
    ]);
    save_profile(&fixture, "One");
    let profiles = std::fs::read(fixture.paths.data_dir.join("profiles.json")).expect("profiles");
    let snapshot = fixture.snapshot();
    let row = snapshot
        .content
        .rows
        .iter()
        .find(|row| row.key == "Logic:X" && row.logic_prefixed)
        .expect("prefixed row");
    let outcome = fixture
        .core
        .mods_rename(&RenameRequest {
            target: ModTarget {
                mod_ref: row.mod_ref.clone(),
                key: row.key.clone(),
            },
            new_name: "Z".to_owned(),
            apply_while_running: false,
        })
        .expect("rename");
    assert!(outcome.rekey.skipped_ambiguous);
    assert_eq!(outcome.rekey.profiles, 0);
    assert_eq!(
        std::fs::read(fixture.paths.data_dir.join("profiles.json")).expect("profiles"),
        profiles
    );
    assert!(fixture.exists("LogicMods/.disabled_Z/X.pak-x"));
}

#[test]
fn plan_rename_table() {
    let fixture = core_with_trash(&[
        ("Mods/Cool/Cool.pak", DATA),
        ("Mods/Taken/a.pak", DATA),
        ("LogicMods/.disabled_L/L.pak-x", DATA),
    ]);
    let cool = fixture.row("Standard:Cool");
    let entries: Vec<OsString> = ["Cool", "Taken"].iter().map(OsString::from).collect();
    let op = plan_rename(&cool, &entries, &[], "New").expect("plan");
    assert_eq!(op.to, cool.path.with_file_name("New"));
    assert!(op.is_dir);
    assert_eq!(op.from, cool.path);
    assert!(
        plan_rename(&cool, &entries, &[], "cOOL").is_ok(),
        "case-only"
    );
    assert!(matches!(
        plan_rename(&cool, &entries, &[], "taken"),
        Err(ModsError::TargetExists)
    ));
    assert!(matches!(
        plan_rename(&cool, &entries, &["Standard:NEW"], "new"),
        Err(ModsError::TargetExists)
    ));
    assert!(matches!(
        plan_rename(&cool, &entries, &[], "Cool"),
        Err(ModsError::RenameUnchanged)
    ));
    let logic = fixture.row("Logic:L");
    let logic_entries = vec![OsString::from(".disabled_L"), OsString::from("M")];
    let op = plan_rename(&logic, &logic_entries, &[], "N").expect("plan");
    assert_eq!(op.to, logic.path.with_file_name(".disabled_N"));
    assert!(matches!(
        plan_rename(&logic, &logic_entries, &[], "m"),
        Err(ModsError::TargetExists)
    ));
    let long = "a".repeat(250);
    assert!(matches!(
        plan_rename(&logic, &logic_entries, &[], &long),
        Err(ModsError::InvalidName { code: "too_long" })
    ));
}

#[test]
fn rename_rekey_failure_keeps_rename() {
    let fixture = core_with_trash(&[("Mods/Cool/Cool.pak", DATA)]);
    // A newer profiles.json is read-only: the rekey fails, the rename stands.
    let newer = br#"{"version":2,"revision":1,"profiles":[{"id":"0123456789abcdef0123456789abcdef","name":"P","mode":"exclusive","mods":{"Standard:Cool":{"state":"enabled"}},"created":"x","updated":"x","source":{"kind":"user"}}]}"#;
    std::fs::create_dir_all(&fixture.paths.data_dir).expect("data dir");
    std::fs::write(fixture.paths.profiles_file(), newer).expect("write");
    let outcome = rename(&fixture, "Standard:Cool", "Hot").expect("rename");
    assert!(outcome.rekey.failed);
    assert!(fixture.exists("Mods/Hot/Cool.pak"));
    assert_eq!(
        std::fs::read(fixture.paths.profiles_file()).expect("read"),
        newer
    );
}
