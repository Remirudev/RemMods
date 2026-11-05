//! Groups in the Mods dashboard: profiles saved from a picked mod list, and switching a group off
//! (only its "on" mods turn off, through the same snapshot and batch as an apply).
#![allow(clippy::expect_used, clippy::disallowed_methods)]

mod mods_support;
mod profiles_support;
mod support;

use mods_support::DATA;
use mods_support::manage::{ToggleFixture, core_with_trash, tree};
use profiles_support::{apply, by_name, custom, snapshots, state, switch};
use remmods_core::CoreError;
use remmods_core::mods::ModState;
use remmods_core::profiles::{
    ApplyDirection::{Off, On},
    ApplyRequest, ChangeReason,
    EntryState::{self, Disabled, Enabled},
    ProfileError, ProfileMode,
    ProfileMode::{Additive, Exclusive},
    ProfileSave, ProfilesView,
};

const BASE: &[(&str, &[u8])] = &[
    ("Mods/A/A.pak", DATA),
    ("Mods/B/B.pak", DATA),
    ("Mods/C/C.pak-x", DATA),
    ("LogicMods/L/L.pak", DATA),
];

fn picked(
    fixture: &ToggleFixture,
    id: Option<&str>,
    name: &str,
    mode: ProfileMode,
    mods: &[(&str, EntryState)],
) -> Result<ProfilesView, CoreError> {
    fixture.core.profiles_save(&ProfileSave::Picked {
        id: id.map(str::to_owned),
        name: name.to_owned(),
        description: String::new(),
        mode,
        mods: mods
            .iter()
            .map(|(key, state)| ((*key).to_owned(), *state))
            .collect(),
        skins: None,
    })
}

fn listed(view: &ProfilesView, name: &str) -> Vec<(String, EntryState)> {
    by_name(view, name)
        .mods
        .iter()
        .map(|(key, entry)| (key.clone(), entry.state))
        .collect()
}

fn stored_json(fixture: &ToggleFixture) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(fixture.paths.profiles_file()).expect("read store"))
        .expect("store json")
}

fn last_applied_id(fixture: &ToggleFixture) -> Option<String> {
    fixture
        .core
        .profiles_list()
        .expect("profiles")
        .last_applied
        .map(|last| last.profile_id)
}

#[test]
fn picked_new_saves_exactly_the_list_and_touches_no_mod() {
    let fixture = core_with_trash(BASE);
    let before = tree(&fixture);
    let view = picked(
        &fixture,
        None,
        "Fine",
        Additive,
        &[
            ("Standard:B", Disabled),
            ("Standard:A", Enabled),
            ("Standard:Gone", Enabled),
        ],
    )
    .expect("save picked");
    assert_eq!(
        listed(&view, "Fine"),
        [
            ("Standard:A".to_owned(), Enabled),
            ("Standard:B".to_owned(), Disabled),
            ("Standard:Gone".to_owned(), Enabled),
        ]
    );
    let summary = view
        .profiles
        .iter()
        .find(|summary| summary.profile.name == "Fine")
        .expect("summary");
    assert_eq!(
        (summary.enabled, summary.disabled, summary.missing),
        (2, 1, 1)
    );
    assert_eq!(tree(&fixture), before);
}

#[test]
fn picked_rejects_repeated_empty_and_control_keys_and_saves_nothing() {
    let fixture = core_with_trash(BASE);
    let bad: [&[(&str, EntryState)]; 3] = [
        &[("Standard:A", Enabled), ("Standard:A", Disabled)],
        &[("", Enabled)],
        &[("Standard:A\nB", Enabled)],
    ];
    for mods in bad {
        let refused = picked(&fixture, None, "Bad", Additive, mods);
        assert!(
            matches!(
                refused,
                Err(CoreError::Profiles(ProfileError::InvalidEntries))
            ),
            "{mods:?}: {refused:?}"
        );
    }
    let view = fixture.core.profiles_list().expect("profiles");
    assert!(view.profiles.is_empty());
}

#[test]
fn picked_follows_the_name_rules() {
    let fixture = core_with_trash(BASE);
    picked(&fixture, None, "Fine", Additive, &[]).expect("first");
    let taken = picked(&fixture, None, "FINE", Additive, &[]);
    assert!(matches!(
        taken,
        Err(CoreError::Profiles(ProfileError::NameTaken))
    ));
    let empty = picked(&fixture, None, "  ", Additive, &[]);
    assert!(matches!(
        empty,
        Err(CoreError::Profiles(ProfileError::InvalidName))
    ));
}

#[test]
fn picked_edit_replaces_the_list_and_keeps_entry_extras() {
    let fixture = core_with_trash(BASE);
    let id = custom(
        &fixture,
        "G",
        Additive,
        &[("Standard:A", Enabled), ("Standard:B", Enabled)],
    );
    // An entry field from a newer RemMods must survive an edit that keeps the entry.
    let mut json = stored_json(&fixture);
    let entry = json["profiles"][0]["mods"]["Standard:A"]
        .as_object_mut()
        .expect("entry");
    entry.insert("variant".to_owned(), serde_json::json!("blue"));
    std::fs::write(
        fixture.paths.profiles_file(),
        serde_json::to_vec_pretty(&json).expect("encode"),
    )
    .expect("write store");

    let view = picked(
        &fixture,
        Some(&id),
        "Renamed",
        Exclusive,
        &[("Standard:A", Disabled), ("Logic:L", Enabled)],
    )
    .expect("edit picked");
    assert_eq!(
        listed(&view, "Renamed"),
        [
            ("Logic:L".to_owned(), Enabled),
            ("Standard:A".to_owned(), Disabled),
        ]
    );
    assert_eq!(by_name(&view, "Renamed").mode, Exclusive);
    let json = stored_json(&fixture);
    let mods = &json["profiles"][0]["mods"];
    assert_eq!(mods["Standard:A"]["variant"], "blue");
    assert_eq!(mods["Standard:A"]["state"], "disabled");
    assert!(mods.get("Standard:B").is_none());
}

#[test]
fn picked_never_replaces_a_snapshot() {
    let fixture = core_with_trash(BASE);
    let id = custom(&fixture, "G", Additive, &[("Standard:A", Disabled)]);
    let snapshot = apply(&fixture, &id)
        .expect("apply")
        .snapshot_id
        .expect("snapshot");
    let refused = picked(&fixture, Some(&snapshot), "S", Additive, &[]);
    assert!(matches!(
        refused,
        Err(CoreError::Profiles(ProfileError::NotFound))
    ));
}

#[test]
fn switching_off_turns_off_only_the_listed_on_mods() {
    let fixture = core_with_trash(BASE);
    // Exclusive, yet switching it off must leave unlisted L alone, and its "off" entry B too.
    let id = custom(
        &fixture,
        "G",
        Exclusive,
        &[
            ("Standard:A", Enabled),
            ("Standard:C", Enabled),
            ("Standard:B", Disabled),
        ],
    );
    let diff = fixture.core.profiles_diff(&id, Off).expect("diff off");
    let changes: Vec<(&str, bool, ChangeReason)> = diff
        .changes
        .iter()
        .map(|change| (change.key.as_str(), change.enable, change.reason))
        .collect();
    assert_eq!(changes, [("Standard:A", false, ChangeReason::Listed)]);
    assert_eq!(diff.unchanged, 1, "C is off already");

    let outcome = switch(&fixture, &id, Off).expect("switch off");
    assert_eq!(outcome.changed, 1);
    assert!(outcome.snapshot_id.is_some());
    assert_eq!(state(&fixture, "Standard:A"), ModState::Disabled);
    assert_eq!(state(&fixture, "Standard:B"), ModState::Enabled);
    assert_eq!(state(&fixture, "Standard:C"), ModState::Disabled);
    assert_eq!(state(&fixture, "Logic:L"), ModState::Enabled);
    assert_eq!(
        last_applied_id(&fixture),
        None,
        "off never counts as applied"
    );
}

#[test]
fn switching_off_with_nothing_on_changes_nothing() {
    let fixture = core_with_trash(BASE);
    let id = custom(&fixture, "G", Additive, &[("Standard:C", Enabled)]);
    let before = tree(&fixture);
    let outcome = switch(&fixture, &id, Off).expect("switch off");
    assert_eq!(outcome.changed, 0);
    assert_eq!(outcome.snapshot_id, None);
    assert_eq!(tree(&fixture), before);
    assert!(snapshots(&fixture).is_empty());
    assert_eq!(last_applied_id(&fixture), None);
}

#[test]
fn an_off_fingerprint_never_applies_on() {
    let fixture = core_with_trash(BASE);
    let id = custom(&fixture, "G", Additive, &[("Standard:A", Enabled)]);
    let off = fixture.core.profiles_diff(&id, Off).expect("diff off");
    assert_eq!(off.fingerprint, "-Standard:A\n");
    let before = tree(&fixture);
    let refused = fixture.core.profiles_apply(&ApplyRequest {
        profile_id: id,
        fingerprint: off.fingerprint,
        apply_while_running: false,
        direction: On,
    });
    assert!(matches!(
        refused,
        Err(CoreError::Profiles(ProfileError::DiffChanged))
    ));
    assert_eq!(tree(&fixture), before);
}

#[test]
fn on_then_off_then_undo_restores_the_tree() {
    let fixture = core_with_trash(BASE);
    let id = custom(
        &fixture,
        "G",
        Exclusive,
        &[("Standard:C", Enabled), ("Standard:A", Disabled)],
    );
    let original = tree(&fixture);
    switch(&fixture, &id, On).expect("on");
    let summary = |fixture: &ToggleFixture| {
        fixture
            .core
            .profiles_list()
            .expect("profiles")
            .profiles
            .into_iter()
            .find(|summary| summary.profile.id == id)
            .expect("summary")
    };
    assert!(summary(&fixture).matches_current);
    assert_eq!(state(&fixture, "Standard:C"), ModState::Enabled);
    assert_eq!(state(&fixture, "Standard:B"), ModState::Disabled);

    let on_state = tree(&fixture);
    let off = switch(&fixture, &id, Off).expect("off");
    assert!(!summary(&fixture).matches_current);
    assert_eq!(state(&fixture, "Standard:C"), ModState::Disabled);
    assert_eq!(
        state(&fixture, "Standard:B"),
        ModState::Disabled,
        "B stays off"
    );

    // Undo of the off switch: apply its snapshot.
    apply(&fixture, &off.snapshot_id.expect("snapshot")).expect("undo");
    assert_eq!(tree(&fixture), on_state);
    assert_ne!(tree(&fixture), original);
}
