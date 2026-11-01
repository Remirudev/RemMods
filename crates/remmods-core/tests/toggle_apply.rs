//! `Core::mods_toggle` on a TempDir game with an injected process list (spec §5.5, §8.2).

mod mods_support;
mod support;

use std::sync::atomic::Ordering;
use std::thread;

use mods_support::toggle::{MARKER, ToggleFixture};
use mods_support::{DATA, paks_tree};
use remmods_core::CoreError;
use remmods_core::mods::{
    ModRef, ModState, ModsError, ToggleBlock, ToggleRequest, ToggleTarget, scan_paks,
};
use remmods_core::safe_fs::FsError;

fn assert_blocked(result: Result<impl std::fmt::Debug, CoreError>, expected: ToggleBlock) {
    match result {
        Err(CoreError::Mods(ModsError::Blocked { reason, .. })) => assert_eq!(reason, expected),
        other => panic!("expected Blocked({expected:?}), got {other:?}"),
    }
}

#[test]
fn off_on_round_trip_restores_every_name() {
    let fixture = ToggleFixture::new(&[
        ("Mods/Cool/Cool.pak", DATA),
        ("Mods/Cool/Cool.ucas", DATA),
        ("Mods/Cool/Sub/Cool_P.utoc", DATA),
        ("Mods/Cool/readme.txt", DATA),
        ("~mods/Old/Old.pak", DATA),
        ("LogicMods/Tool/Tool.pak", DATA),
        ("Mods/Loose.pak", DATA),
    ]);
    let keys = [
        "Standard:Cool",
        "Legacy:Old",
        "Logic:Tool",
        "Loose:Mods/Loose",
    ];
    let before = fixture.names();
    fixture.toggle(&keys, false).expect("disable");
    for key in keys {
        assert_eq!(fixture.row(key).state, ModState::Disabled, "{key}");
    }
    assert!(fixture.exists("LogicMods/.disabled_Tool/Tool.pak-x"));
    assert!(fixture.exists("Mods/Cool/readme.txt"));
    fixture.toggle(&keys, true).expect("enable");
    assert_eq!(fixture.names(), before);
}

#[test]
fn blocked_rows_change_nothing() {
    let fixture = ToggleFixture::new(&[
        ("LogicMods/X/a.pak", DATA),
        ("LogicMods/.disabled_X/a.pak-x", DATA),
        ("LogicMods/.disabled_/a.pak-x", DATA),
        ("Mods/Empty/readme.txt", DATA),
    ]);
    let before = fixture.names();
    let snapshot = fixture.snapshot();
    let ambiguous = snapshot
        .content
        .rows
        .iter()
        .find(|row| row.key == "Logic:X");
    let ambiguous = ambiguous.expect("ambiguous row");
    let request = ToggleRequest {
        targets: vec![ToggleTarget {
            mod_ref: ambiguous.mod_ref.clone(),
            key: ambiguous.key.clone(),
        }],
        enable: false,
        apply_while_running: false,
    };
    assert_blocked(fixture.core.mods_toggle(&request), ToggleBlock::Ambiguous);
    assert_blocked(
        fixture.toggle(&["Logic:"], true),
        ToggleBlock::InvalidLogicName,
    );
    assert_blocked(
        fixture.toggle(&["Standard:Empty"], false),
        ToggleBlock::NoPakFiles,
    );
    assert_eq!(fixture.names(), before);
}

#[test]
fn wrong_key_or_vanished_ref_is_stale() {
    let fixture = ToggleFixture::new(&[("Mods/A/a.pak", DATA), ("Mods/B/b.pak", DATA)]);
    let before = fixture.names();
    let mut wrong_key = fixture.target("Standard:A");
    wrong_key.key = "Standard:B".to_owned();
    let gone = ToggleTarget {
        mod_ref: ModRef::from_token("d1.0.Bogus"),
        key: "Standard:A".to_owned(),
    };
    for target in [wrong_key, gone] {
        let result = fixture.core.mods_toggle(&ToggleRequest {
            targets: vec![target],
            enable: false,
            apply_while_running: false,
        });
        assert!(
            matches!(result, Err(CoreError::Mods(ModsError::Stale))),
            "{result:?}"
        );
    }
    assert_eq!(fixture.names(), before);
}

#[test]
fn an_existing_target_stops_the_whole_toggle() {
    let fixture = ToggleFixture::new(&[
        ("Mods/A/a.pak", DATA),
        ("Mods/Foo/Foo.pak", DATA),
        ("Mods/Foo/Foo.pak-x", DATA),
    ]);
    let before = fixture.names();
    let result = fixture.toggle(&["Standard:A", "Standard:Foo"], false);
    assert!(
        matches!(result, Err(CoreError::Fs(FsError::TargetExists { .. }))),
        "{result:?}"
    );
    assert_eq!(fixture.names(), before);
}

#[test]
fn a_running_game_needs_consent() {
    let fixture = ToggleFixture::new(&[("Mods/A/a.pak", DATA)]);
    let before = fixture.names();
    fixture.running.store(true, Ordering::SeqCst);
    let result = fixture.toggle(&["Standard:A"], false);
    assert!(
        matches!(result, Err(CoreError::Mods(ModsError::GameRunning))),
        "{result:?}"
    );
    assert_eq!(fixture.names(), before);
    let request = ToggleRequest {
        targets: vec![fixture.target("Standard:A")],
        enable: false,
        apply_while_running: true,
    };
    fixture.core.mods_toggle(&request).expect("applied anyway");
    assert!(fixture.exists("Mods/A/a.pak-x"));
}

#[test]
fn a_smoke_core_needs_the_marker() {
    let fixture = ToggleFixture::requiring_marker(&[("Mods/A/a.pak", DATA)]);
    let before = fixture.names();
    let result = fixture.toggle(&["Standard:A"], false);
    assert!(
        matches!(result, Err(CoreError::Mods(ModsError::NotAFixture))),
        "{result:?}"
    );
    assert_eq!(fixture.names(), before);
    paks_tree(&fixture.paks, &[(MARKER, b"{}")]);
    fixture
        .toggle(&["Standard:A"], false)
        .expect("marker present");
    assert!(fixture.exists("Mods/A/a.pak-x"));
}

#[test]
fn a_failed_record_write_falls_back_to_strip_all() {
    let fixture = ToggleFixture::new(&[
        ("Mods/Cool/Cool.pak", DATA),
        ("Mods/Cool/Cool_alt.pak-x", DATA),
    ]);
    let meta = fixture.paths.data_dir.join("mod-meta.json");
    paks_tree(&meta, &[("blocker/", b"")]);
    fixture.toggle(&["Standard:Cool"], false).expect("disable");
    assert!(fixture.exists("Mods/Cool/Cool.pak-x"));
    fixture.toggle(&["Standard:Cool"], true).expect("enable");
    assert!(fixture.exists("Mods/Cool/Cool.pak"));
    assert!(fixture.exists("Mods/Cool/Cool_alt.pak"), "strip-all rule");
}

#[test]
fn a_hand_disabled_variant_survives_a_round_trip() {
    let fixture = ToggleFixture::new(&[
        ("Mods/Cool/Cool.pak", DATA),
        ("Mods/Cool/Cool_alt.pak-x", DATA),
    ]);
    let before = fixture.names();
    fixture.toggle(&["Standard:Cool"], false).expect("disable");
    assert_eq!(fixture.row("Standard:Cool").state, ModState::Disabled);
    fixture.toggle(&["Standard:Cool"], true).expect("enable");
    assert_eq!(fixture.row("Standard:Cool").state, ModState::Partial);
    assert!(fixture.exists("Mods/Cool/Cool_alt.pak-x"));
    assert_eq!(fixture.names(), before);
}

#[test]
fn a_refresh_during_toggles_finishes_and_ends_on_the_disk_state() {
    let fixture = ToggleFixture::new(&[("Mods/A/a.pak", DATA), ("LogicMods/L/l.pak", DATA)]);
    thread::scope(|scope| {
        let refresher = scope.spawn(|| {
            for _ in 0..20 {
                fixture.core.mods_refresh().expect("refresh");
            }
        });
        for round in 0..6 {
            let enable = round % 2 == 1;
            fixture
                .toggle(&["Standard:A", "Logic:L"], enable)
                .expect("toggle");
        }
        refresher.join().expect("refresher thread");
    });
    let last = fixture.core.mods_last().expect("a snapshot");
    let disk = scan_paks(&mods_support::input(&fixture.paks)).expect("scan");
    assert_eq!(last.content.rows.len(), disk.content.rows.len());
    for (cached, fresh) in last.content.rows.iter().zip(&disk.content.rows) {
        assert_eq!((&cached.key, cached.state), (&fresh.key, fresh.state));
    }
}

#[test]
fn unmanaged_and_text_files_are_never_renamed() {
    let fixture = ToggleFixture::new(&[
        ("Stray.pak", DATA),
        ("Other/x.pak", DATA),
        ("Mods/Loose.pak", DATA),
        ("Mods/Loose.txt", DATA),
        ("Mods/A/a.pak", DATA),
        ("Mods/A/a.txt", DATA),
    ]);
    fixture
        .toggle(&["Loose:Mods/Loose", "Standard:A"], false)
        .expect("disable");
    fixture.core.mods_disable_all(false).expect("disable all");
    for rel in ["Stray.pak", "Other/x.pak", "Mods/Loose.txt", "Mods/A/a.txt"] {
        assert!(fixture.exists(rel), "{rel}");
    }
}

#[test]
fn each_mutation_publishes_a_new_revision() {
    let fixture = ToggleFixture::new(&[("Mods/A/a.pak", DATA)]);
    let first = fixture.snapshot().revision;
    let off = fixture.toggle(&["Standard:A"], false).expect("disable");
    assert!(off.refreshed.changed);
    assert!(off.refreshed.snapshot.revision > first);
    let on = fixture.toggle(&["Standard:A"], true).expect("enable");
    assert!(on.refreshed.snapshot.revision > off.refreshed.snapshot.revision);
    let again = fixture.toggle(&["Standard:A"], true).expect("no-op");
    assert!(!again.refreshed.changed);
}

#[test]
fn a_failed_toggle_still_publishes_the_disk_state() {
    let fixture = ToggleFixture::new(&[("Mods/Foo/Foo.pak", DATA), ("Mods/Foo/Foo.pak-x", DATA)]);
    let target = fixture.target("Standard:Foo");
    paks_tree(&fixture.paks, &[("Mods/New/new.pak", DATA)]);
    let result = fixture.core.mods_toggle(&ToggleRequest {
        targets: vec![target],
        enable: false,
        apply_while_running: false,
    });
    assert!(result.is_err());
    let last = fixture.core.mods_last().expect("a snapshot");
    assert!(
        last.content
            .rows
            .iter()
            .any(|row| row.key == "Standard:New")
    );
}

#[test]
fn bulk_targets_run_in_one_batch() {
    let fixture = ToggleFixture::new(&[
        ("Mods/A/a.pak", DATA),
        ("Mods/B/b.pak", DATA),
        ("LogicMods/L/l.pak", DATA),
    ]);
    let outcome = fixture
        .toggle(
            &["Standard:A", "Standard:B", "Logic:L", "Standard:A"],
            false,
        )
        .expect("disable");
    let rows = &outcome.refreshed.snapshot.content.rows;
    assert!(rows.iter().all(|row| row.state == ModState::Disabled));
    assert!(fixture.exists("LogicMods/.disabled_L/l.pak-x"));
    assert!(!fixture.paths.data_dir.join("journal.json").exists() || journal_is_empty(&fixture));
}

fn journal_is_empty(fixture: &ToggleFixture) -> bool {
    let text =
        std::fs::read_to_string(fixture.paths.data_dir.join("journal.json")).unwrap_or_default();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
    value["entries"].as_array().is_none_or(Vec::is_empty)
}
