//! Profile diff and apply (spec §5.8, §8.2 P1, P3–P15). Data safety first; P2 is in
//! `platform_linux_mods.rs`.
#![allow(clippy::expect_used, clippy::disallowed_methods)]

mod mods_support;
mod profiles_support;
mod support;

use std::sync::atomic::Ordering;

use mods_support::DATA;
use mods_support::manage::{core_with_trash, restart, tree};
use profiles_support::{apply, by_name, custom, save, snapshots, state};
use remmods_core::CoreError;
use remmods_core::mods::{ModState, ModsError};
use remmods_core::profiles::{
    ApplyDirection, ApplyRequest, ChangeReason,
    EntryState::{Disabled, Enabled},
    Profile, ProfileError,
    ProfileMode::{Additive, Exclusive},
    SkipReason, capture_states, diff_profile,
};
use remmods_core::safe_fs::FaultPlan;

const BASE: &[(&str, &[u8])] = &[
    ("Mods/A/A.pak", DATA),
    ("Mods/B/B.pak", DATA),
    ("Mods/C/C.pak-x", DATA),
    ("LogicMods/L/L.pak", DATA),
];

/// A fixture with an Exclusive profile "Off" (A and B off) saved while the disk is BASE.
fn with_off_profile() -> (mods_support::manage::ToggleFixture, String) {
    let fixture = core_with_trash(BASE);
    fixture
        .toggle(&["Standard:A", "Standard:B"], false)
        .expect("disable");
    let id = save(&fixture, "Off", Exclusive);
    fixture
        .toggle(&["Standard:A", "Standard:B"], true)
        .expect("enable");
    (fixture, id)
}

#[test]
fn apply_stale_fingerprint_changes_nothing() {
    let (fixture, id) = with_off_profile();
    let diff = fixture
        .core
        .profiles_diff(&id, ApplyDirection::On)
        .expect("diff");
    fixture
        .toggle(&["Standard:A"], false)
        .expect("disk changes");
    let before = tree(&fixture);
    let result = fixture.core.profiles_apply(&ApplyRequest {
        profile_id: id,
        fingerprint: diff.fingerprint,
        apply_while_running: false,
        direction: ApplyDirection::On,
    });
    assert!(matches!(
        result,
        Err(CoreError::Profiles(ProfileError::DiffChanged))
    ));
    assert_eq!(tree(&fixture), before);
    assert!(snapshots(&fixture).is_empty());
}

#[test]
fn apply_batch_failure_rolls_back_all() {
    let (fixture, id) = with_off_profile();
    let before = tree(&fixture);
    fixture.core.inject_batch_fault(FaultPlan {
        fail_before: Some(1),
        ..FaultPlan::default()
    });
    assert!(apply(&fixture, &id).is_err());
    assert_eq!(tree(&fixture), before);
    assert_eq!(snapshots(&fixture).len(), 1, "the snapshot is kept");
    let view = fixture.core.profiles_list().expect("profiles");
    assert_eq!(view.last_applied, None);
    assert_eq!(by_name(&view, "Off").last_applied, None);
}

#[test]
fn apply_crash_recovers() {
    let (mut fixture, id) = with_off_profile();
    let before = tree(&fixture);
    fixture.core.inject_batch_fault(FaultPlan {
        crash_after: Some(1),
        ..FaultPlan::default()
    });
    assert!(apply(&fixture, &id).is_err());
    restart(&mut fixture);
    assert_eq!(tree(&fixture), before);
}

#[test]
fn apply_while_running_needs_flag() {
    let (fixture, id) = with_off_profile();
    let before = tree(&fixture);
    fixture.running.store(true, Ordering::SeqCst);
    let diff = fixture
        .core
        .profiles_diff(&id, ApplyDirection::On)
        .expect("diff");
    let mut request = ApplyRequest {
        profile_id: id,
        fingerprint: diff.fingerprint,
        apply_while_running: false,
        direction: ApplyDirection::On,
    };
    let refused = fixture.core.profiles_apply(&request);
    assert!(matches!(
        refused,
        Err(CoreError::Mods(ModsError::GameRunning))
    ));
    assert_eq!(tree(&fixture), before);
    request.apply_while_running = true;
    fixture.core.profiles_apply(&request).expect("forced apply");
    assert_eq!(state(&fixture, "Standard:A"), ModState::Disabled);
}

#[test]
fn apply_additive_touches_only_listed() {
    let fixture = core_with_trash(BASE);
    let id = custom(
        &fixture,
        "P",
        Additive,
        &[
            ("Standard:A", Disabled),
            ("Standard:C", Enabled),
            ("Standard:Gone", Enabled),
        ],
    );
    let outcome = apply(&fixture, &id).expect("apply");
    assert_eq!(outcome.changed, 2);
    assert_eq!(state(&fixture, "Standard:A"), ModState::Disabled);
    assert_eq!(state(&fixture, "Standard:C"), ModState::Enabled);
    assert_eq!(state(&fixture, "Standard:B"), ModState::Enabled);
    assert_eq!(state(&fixture, "Logic:L"), ModState::Enabled);
}

#[test]
fn apply_exclusive_disables_unlisted_on_rows() {
    let fixture = core_with_trash(&[
        ("Mods/A/A.pak", DATA),
        ("Mods/A/A2.pak-x", DATA),
        ("LogicMods/.disabled_L/L.pak", DATA),
        ("Mods/B/B.pak", DATA),
        ("Mods/C/C.pak-x", DATA),
    ]);
    assert_eq!(state(&fixture, "Standard:A"), ModState::Partial);
    let id = custom(&fixture, "OnlyB", Exclusive, &[("Standard:B", Enabled)]);
    let diff = fixture
        .core
        .profiles_diff(&id, ApplyDirection::On)
        .expect("diff");
    assert_eq!(diff.unchanged, 1);
    let keys: Vec<(&str, bool, ChangeReason)> = diff
        .changes
        .iter()
        .map(|change| (change.key.as_str(), change.enable, change.reason))
        .collect();
    assert_eq!(
        keys,
        [
            ("Logic:L", false, ChangeReason::Unlisted),
            ("Standard:A", false, ChangeReason::Unlisted),
        ]
    );
    apply(&fixture, &id).expect("apply");
    assert_eq!(state(&fixture, "Standard:A"), ModState::Disabled);
    assert_eq!(state(&fixture, "Logic:L"), ModState::Disabled);
    assert!(fixture.exists("LogicMods/.disabled_L/L.pak-x"));
    assert_eq!(state(&fixture, "Standard:B"), ModState::Enabled);
    assert_eq!(state(&fixture, "Standard:C"), ModState::Disabled);
}

#[test]
fn apply_skips_ambiguous_and_blocked() {
    let fixture = core_with_trash(&[
        ("LogicMods/X/X.pak", DATA),
        ("LogicMods/.disabled_X/X.pak-x", DATA),
        ("Mods/Empty/readme.txt", DATA),
        ("Mods/B/B.pak-x", DATA),
    ]);
    let before = tree(&fixture);
    let id = custom(
        &fixture,
        "P",
        Additive,
        &[("Logic:X", Disabled), ("Standard:Empty", Enabled)],
    );
    let outcome = apply(&fixture, &id).expect("apply");
    assert_eq!(outcome.changed, 0);
    let reasons: Vec<(&str, SkipReason)> = outcome
        .skipped
        .iter()
        .map(|skip| (skip.key.as_str(), skip.reason))
        .collect();
    assert_eq!(
        reasons,
        [
            ("Logic:X", SkipReason::Ambiguous),
            ("Standard:Empty", SkipReason::NoPakFiles)
        ]
    );
    assert_eq!(tree(&fixture), before);
}

#[test]
fn imported_empty_default_disables_everything() {
    let fixture = core_with_trash(BASE);
    let source = fixture.root.join("other-profiles.json");
    std::fs::write(
        &source,
        br#"[{"Id":"7c9e","Name":"Default","Description":null,"EnabledMods":[],"Created":"2025-01-01T00:00:00","LastUsed":null,"IsDefault":true}]"#,
    )
    .expect("write source");
    let report = fixture
        .core
        .profiles_import_other_manager(&source)
        .expect("import");
    assert_eq!(report.added, 1);
    let id = by_name(&report.view, "Default").id.clone();
    let outcome = apply(&fixture, &id).expect("apply");
    assert_eq!(outcome.changed, 3);
    for key in ["Standard:A", "Standard:B", "Standard:C", "Logic:L"] {
        assert_eq!(state(&fixture, key), ModState::Disabled, "{key}");
    }
}

#[test]
fn apply_back_and_forth_restores_exact_tree() {
    let fixture = core_with_trash(BASE);
    let a = save(&fixture, "A", Exclusive);
    let b = custom(
        &fixture,
        "B",
        Exclusive,
        &[
            ("Standard:C", Enabled),
            ("Standard:A", Disabled),
            ("Logic:L", Disabled),
        ],
    );
    apply(&fixture, &a).expect("apply A");
    let first = tree(&fixture);
    apply(&fixture, &b).expect("apply B");
    assert_ne!(tree(&fixture), first);
    assert_eq!(state(&fixture, "Standard:B"), ModState::Disabled);
    apply(&fixture, &a).expect("apply A again");
    assert_eq!(tree(&fixture), first);
    // Records match the disk: only C (disabled again by RemMods) has one, naming C.pak.
    let meta: serde_json::Value = serde_json::from_slice(
        &std::fs::read(fixture.paths.data_dir.join("mod-meta.json")).expect("meta"),
    )
    .expect("meta json");
    let mods = meta["mods"].as_object().expect("mods");
    let keys: Vec<&str> = mods.keys().map(String::as_str).collect();
    assert_eq!(keys, ["Standard:C"]);
    assert_eq!(
        mods["Standard:C"]["disabled"]["files"],
        serde_json::json!(["C.pak"])
    );
}

#[test]
fn snapshot_restores_pre_apply_state() {
    let (fixture, id) = with_off_profile();
    let original = tree(&fixture);
    let outcome = apply(&fixture, &id).expect("apply");
    let snapshot_id = outcome.snapshot_id.expect("a snapshot");
    assert_ne!(tree(&fixture), original);
    apply(&fixture, &snapshot_id).expect("apply the snapshot");
    assert_eq!(tree(&fixture), original);
}

#[test]
fn snapshots_pruned_to_ten() {
    let (fixture, off) = with_off_profile();
    let on = save(&fixture, "On", Exclusive);
    for round in 0..12 {
        let id = if round % 2 == 0 { &off } else { &on };
        apply(&fixture, id).expect("apply");
    }
    assert_eq!(snapshots(&fixture).len(), 10);
}

fn profile(
    mode: remmods_core::profiles::ProfileMode,
    entries: &[(&str, remmods_core::profiles::EntryState)],
) -> Profile {
    Profile {
        id: "0".repeat(32),
        name: "T".to_owned(),
        description: String::new(),
        mode,
        mods: entries
            .iter()
            .map(|(key, state)| {
                (
                    (*key).to_owned(),
                    remmods_core::profiles::ProfileEntry::new(*state),
                )
            })
            .collect(),
        created: String::new(),
        updated: String::new(),
        last_applied: None,
        source: remmods_core::profiles::ProfileSource::User,
        extra: serde_json::Map::new(),
    }
}

#[test]
fn diff_table() {
    let fixture = core_with_trash(&[
        ("Mods/P/P.pak", DATA),
        ("Mods/P/Q.pak-x", DATA),
        ("Mods/Off/O.pak-x", DATA),
        ("Mods/Loose.pak", DATA),
        ("Mods/On/On.pak", DATA),
    ]);
    let content = fixture.snapshot().content.clone();
    let additive = profile(
        Additive,
        &[
            ("Standard:P", Enabled),
            ("Standard:Off", Enabled),
            ("Loose:Mods/Loose", Disabled),
            ("Standard:Missing", Enabled),
            ("Legacy:Also", Disabled),
        ],
    );
    let diff = diff_profile(&additive, &content);
    assert_eq!(diff.unchanged, 1, "Partial wanted Enabled is unchanged");
    assert_eq!(diff.missing, ["Legacy:Also", "Standard:Missing"]);
    assert_eq!(diff.fingerprint, "+Standard:Off\n-Loose:Mods/Loose\n");
    assert!(diff.skipped.is_empty());
    let exclusive = profile(Exclusive, &[("Standard:Off", Disabled)]);
    let diff = diff_profile(&exclusive, &content);
    assert_eq!(diff.unchanged, 1);
    assert_eq!(
        diff.fingerprint,
        "-Loose:Mods/Loose\n-Standard:On\n-Standard:P\n"
    );
}

#[test]
fn capture_states_excludes_ambiguous_and_blocked() {
    let fixture = core_with_trash(&[
        ("LogicMods/X/X.pak", DATA),
        ("LogicMods/.disabled_X/X.pak-x", DATA),
        ("Mods/Empty/readme.txt", DATA),
        ("Mods/A/A.pak", DATA),
        ("Mods/C/C.pak-x", DATA),
        ("Mods/Loose.pak", DATA),
    ]);
    let states = capture_states(&fixture.snapshot().content);
    let listed: Vec<(&str, remmods_core::profiles::EntryState)> = states
        .iter()
        .map(|(key, entry)| (key.as_str(), entry.state))
        .collect();
    assert_eq!(
        listed,
        [
            ("Loose:Mods/Loose", Enabled),
            ("Standard:A", Enabled),
            ("Standard:C", Disabled),
        ]
    );
}

#[test]
fn no_change_apply_records_last_applied_only() {
    let fixture = core_with_trash(BASE);
    let id = save(&fixture, "Now", Exclusive);
    let before = tree(&fixture);
    let outcome = apply(&fixture, &id).expect("apply");
    assert_eq!(outcome.changed, 0);
    assert_eq!(outcome.snapshot_id, None);
    assert_eq!(tree(&fixture), before);
    assert!(snapshots(&fixture).is_empty());
    let view = fixture.core.profiles_list().expect("profiles");
    assert_eq!(view.last_applied.expect("last applied").profile_id, id);
    let summary = view
        .profiles
        .iter()
        .find(|summary| summary.profile.id == id)
        .expect("profile");
    assert!(summary.profile.last_applied.is_some());
    assert!(summary.matches_current);
}
