#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod support;

use std::fs;
use std::path::Path;

use remmods_core::platform::{self, RawName};
use remmods_core::safe_fs::names::{escape_bytes, escape_wide};
use remmods_core::safe_fs::{FsError, JournalProblem, RecoveryReport};
use serde_json::{Value, json};

use support::{FsFixture, crash_after, options};

const DEAD_SESSION: &str = "0123456789abcdef0123456789abcdef";

fn record(path: &Path) -> Value {
    match platform::raw_name(path.as_os_str()) {
        RawName::Bytes(bytes) => json!({ "form": "bytes", "text": escape_bytes(&bytes) }),
        RawName::Wide(units) => json!({ "form": "wide", "text": escape_wide(&units) }),
    }
}

fn rename(fixture: &FsFixture, from: &str, to: &str, is_dir: bool) -> Value {
    json!({
        "kind": "rename",
        "isDir": is_dir,
        "from": record(&fixture.path(from)),
        "to": record(&fixture.path(to)),
    })
}

fn placeholder(fixture: &FsFixture, path: &str) -> Value {
    json!({ "kind": "placeholder", "path": record(&fixture.path(path)) })
}

/// Writes a journal with one entry of a dead session.
fn craft(fixture: &FsFixture, ops: Vec<Value>) {
    let journal = json!({
        "version": 1,
        "revision": 3,
        "entries": [{
            "id": "fedcba9876543210fedcba9876543210",
            "label": "toggle",
            "owner": { "pid": 1, "session": DEAD_SESSION },
            "createdAt": "2026-11-01T12:00:00.000Z",
            "stuck": false,
            "ops": ops,
        }],
    });
    let bytes = serde_json::to_vec_pretty(&journal).expect("encode");
    fs::write(fixture.journal_path(), bytes).expect("write journal");
}

fn recover(fixture: &FsFixture) -> RecoveryReport {
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");
    guard.recovery().clone()
}

fn logic_mod() -> FsFixture {
    let fixture = support::fs_fixture();
    support::write_tree(
        &fixture.paks,
        &[
            ("LogicMods/X/X.pak", b"xpak"),
            ("LogicMods/X/X.ucas", b"xucas"),
        ],
    );
    fixture
}

fn pak_mod() -> FsFixture {
    let fixture = support::fs_fixture();
    support::write_tree(
        &fixture.paks,
        &[
            ("Mods/Cool/Cool.pak", b"pak"),
            ("Mods/Cool/Cool.ucas", b"ucas"),
        ],
    );
    fixture
}

fn disable_cool(fixture: &FsFixture) -> Vec<remmods_core::safe_fs::RenameOp> {
    vec![
        fixture.op("Mods/Cool/Cool.pak", "Mods/Cool/Cool.pak-x", false),
        fixture.op("Mods/Cool/Cool.ucas", "Mods/Cool/Cool.ucas-x", false),
    ]
}

#[test]
fn an_entry_has_the_documented_format() {
    let fixture = pak_mod();
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");

    let result = guard.rename_batch(&disable_cool(&fixture), &options(crash_after(1)));

    assert!(matches!(result, Err(FsError::Batch(_))));
    let json = support::journal_json(&fixture.journal_path()).expect("journal");
    assert_eq!(json["version"], 1);
    assert!(
        json["revision"]
            .as_u64()
            .is_some_and(|revision| revision >= 1)
    );
    let entry = &json["entries"][0];
    let id = entry["id"].as_str().expect("id");
    assert_eq!(id.len(), 32);
    assert!(
        id.bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    );
    assert_eq!(entry["label"], "test");
    assert_eq!(entry["owner"]["pid"], std::process::id());
    assert_eq!(entry["owner"]["session"], gate.session());
    assert!(
        entry["createdAt"]
            .as_str()
            .is_some_and(|at| at.ends_with('Z'))
    );
    assert_eq!(entry["stuck"], false);
    assert_eq!(
        entry["ops"][0],
        rename(
            &fixture,
            "Mods/Cool/Cool.pak",
            "Mods/Cool/Cool.pak-x",
            false
        )
    );
    assert_eq!(entry["ops"].as_array().map(Vec::len), Some(2));
}

#[test]
fn a_missing_journal_means_no_entries() {
    let fixture = pak_mod();

    let report = recover(&fixture);

    assert_eq!(report, RecoveryReport::default());
    assert!(!fixture.journal_path().exists());
}

#[test]
fn a_newer_journal_refuses_every_mutation() {
    let fixture = pak_mod();
    let newer = br#"{"version":2,"revision":1,"entries":[],"future":true}"#;
    fs::write(fixture.journal_path(), newer).expect("write journal");
    let gate = fixture.gate();

    let result = gate.write(fixture.scope());

    assert!(matches!(
        result,
        Err(FsError::Journal {
            reason: JournalProblem::NewerVersion
        })
    ));
    assert_eq!(fs::read(fixture.journal_path()).expect("read"), newer);
}

#[test]
fn a_corrupt_journal_is_backed_up_and_treated_as_empty() {
    let fixture = pak_mod();
    fs::write(fixture.journal_path(), b"{ not json").expect("write journal");

    let report = recover(&fixture);

    assert_eq!(report, RecoveryReport::default());
    let backups = support::names_in(&fixture.app.data_dir)
        .into_iter()
        .filter(|name| name.starts_with("journal.json.backup."))
        .count();
    assert_eq!(backups, 1);
    assert_eq!(support::journal_entries(&fixture.journal_path()), 0);
}

#[test]
fn a_crashed_batch_is_rolled_back_by_the_next_session() {
    let fixture = pak_mod();
    let before = fixture.paks_snapshot();
    {
        let gate = fixture.gate();
        let guard = gate.write(fixture.scope()).expect("write guard");
        let result = guard.rename_batch(&disable_cool(&fixture), &options(crash_after(2)));
        assert!(matches!(result, Err(FsError::Batch(_))));
    }
    assert_ne!(fixture.paks_snapshot(), before);

    let report = recover(&fixture);

    assert_eq!(report.entries, 1);
    assert_eq!(report.rolled_back, 2);
    assert_eq!(fixture.paks_snapshot(), before);
    assert_eq!(support::journal_entries(&fixture.journal_path()), 0);
}

#[test]
fn a_live_owners_entry_is_not_touched() {
    let fixture = pak_mod();
    let first = fixture.gate();
    {
        let guard = first.write(fixture.scope()).expect("write guard");
        let result = guard.rename_batch(&disable_cool(&fixture), &options(crash_after(1)));
        assert!(result.is_err());
    }
    let after_crash = fixture.paks_snapshot();

    let report = recover(&fixture);

    assert_eq!(report, RecoveryReport::default());
    assert_eq!(fixture.paks_snapshot(), after_crash);
    assert_eq!(support::journal_entries(&fixture.journal_path()), 1);
    drop(first);
}

#[test]
fn an_interrupted_link_fallback_loses_the_second_name() {
    let fixture = pak_mod();
    fs::hard_link(
        fixture.path("Mods/Cool/Cool.pak"),
        fixture.path("Mods/Cool/Cool.pak-x"),
    )
    .expect("hard link");
    craft(
        &fixture,
        vec![rename(
            &fixture,
            "Mods/Cool/Cool.pak",
            "Mods/Cool/Cool.pak-x",
            false,
        )],
    );

    let report = recover(&fixture);

    assert_eq!(report.rolled_back, 1);
    assert_eq!(
        support::names_in(&fixture.path("Mods/Cool")),
        ["Cool.pak", "Cool.ucas"]
    );
    assert_eq!(
        fs::read(fixture.path("Mods/Cool/Cool.pak")).expect("read"),
        b"pak"
    );
    assert_eq!(support::journal_entries(&fixture.journal_path()), 0);
}

#[test]
fn an_entry_outside_the_scope_is_foreign_and_kept() {
    let fixture = pak_mod();
    support::write_tree(&fixture.paks, &[("base.pak-x", b"base")]);
    craft(
        &fixture,
        vec![rename(&fixture, "base.pak", "base.pak-x", false)],
    );

    let report = recover(&fixture);

    assert_eq!(report.foreign, 1);
    assert!(fixture.path("base.pak-x").exists());
    assert_eq!(support::journal_entries(&fixture.journal_path()), 1);
}

#[test]
fn a_logic_batch_crashed_midway_is_undone() {
    let fixture = logic_mod();
    let before = fixture.paks_snapshot();
    fs::rename(
        fixture.path("LogicMods/X"),
        fixture.path("LogicMods/.disabled_X"),
    )
    .expect("mv");
    fs::rename(
        fixture.path("LogicMods/.disabled_X/X.pak"),
        fixture.path("LogicMods/.disabled_X/X.pak-x"),
    )
    .expect("mv");
    craft(
        &fixture,
        vec![
            rename(&fixture, "LogicMods/X", "LogicMods/.disabled_X", true),
            rename(
                &fixture,
                "LogicMods/.disabled_X/X.pak",
                "LogicMods/.disabled_X/X.pak-x",
                false,
            ),
            rename(
                &fixture,
                "LogicMods/.disabled_X/X.ucas",
                "LogicMods/.disabled_X/X.ucas-x",
                false,
            ),
        ],
    );

    let report = recover(&fixture);

    assert_eq!(report.rolled_back, 2);
    assert_eq!(fixture.paks_snapshot(), before);
    assert_eq!(support::journal_entries(&fixture.journal_path()), 0);
}

#[test]
fn an_op_with_neither_path_is_not_done_and_the_entry_goes() {
    let fixture = pak_mod();
    craft(
        &fixture,
        vec![rename(
            &fixture,
            "Mods/Gone/a.pak",
            "Mods/Gone/a.pak-x",
            false,
        )],
    );

    let report = recover(&fixture);

    assert_eq!(report.entries, 1);
    assert_eq!(report.stuck, 0);
    assert_eq!(support::journal_entries(&fixture.journal_path()), 0);
}

#[test]
fn two_different_files_at_both_paths_are_stuck_and_untouched() {
    let fixture = pak_mod();
    support::write_tree(&fixture.paks, &[("Mods/Cool/Cool.pak-x", b"other")]);
    let before = fixture.paks_snapshot();
    craft(
        &fixture,
        vec![rename(
            &fixture,
            "Mods/Cool/Cool.pak",
            "Mods/Cool/Cool.pak-x",
            false,
        )],
    );

    let report = recover(&fixture);

    assert_eq!(report.stuck, 1);
    assert_eq!(fixture.paks_snapshot(), before);
    let json = support::journal_json(&fixture.journal_path()).expect("journal");
    assert_eq!(json["entries"][0]["stuck"], true);
}

#[test]
fn a_placeholder_made_before_the_crash_is_removed() {
    let fixture = logic_mod();
    let before = fixture.paks_snapshot();
    support::write_tree(&fixture.paks, &[("LogicMods/.disabled_X/", b"")]);
    craft(
        &fixture,
        vec![
            rename(&fixture, "LogicMods/X", "LogicMods/.disabled_X", true),
            placeholder(&fixture, "LogicMods/.disabled_X"),
        ],
    );

    let report = recover(&fixture);

    assert_eq!(report.stuck, 0);
    assert_eq!(fixture.paks_snapshot(), before);
}

#[test]
fn an_empty_mod_folder_at_the_placeholder_path_is_renamed_back() {
    let fixture = support::fs_fixture();
    support::write_tree(&fixture.paks, &[("LogicMods/.disabled_X/", b"")]);
    craft(
        &fixture,
        vec![
            rename(&fixture, "LogicMods/X", "LogicMods/.disabled_X", true),
            placeholder(&fixture, "LogicMods/.disabled_X"),
        ],
    );

    let report = recover(&fixture);

    assert_eq!(report.rolled_back, 1);
    assert_eq!(support::names_in(&fixture.path("LogicMods")), ["X"]);
}

#[test]
fn an_undo_placeholder_is_removed_and_the_folder_renamed_back() {
    let fixture = logic_mod();
    let before = fixture.paks_snapshot();
    fs::rename(
        fixture.path("LogicMods/X"),
        fixture.path("LogicMods/.disabled_X"),
    )
    .expect("mv");
    support::write_tree(&fixture.paks, &[("LogicMods/X/", b"")]);
    craft(
        &fixture,
        vec![
            rename(&fixture, "LogicMods/X", "LogicMods/.disabled_X", true),
            placeholder(&fixture, "LogicMods/X"),
        ],
    );

    let report = recover(&fixture);

    assert_eq!(report.rolled_back, 1);
    assert_eq!(fixture.paks_snapshot(), before);
}

#[test]
fn a_recreated_source_is_never_overwritten() {
    let fixture = pak_mod();
    fs::rename(
        fixture.path("Mods/Cool/Cool.pak"),
        fixture.path("Mods/Cool/Cool.pak-x"),
    )
    .expect("mv");
    support::write_tree(&fixture.paks, &[("Mods/Cool/Cool.pak", b"new")]);
    craft(
        &fixture,
        vec![rename(
            &fixture,
            "Mods/Cool/Cool.pak",
            "Mods/Cool/Cool.pak-x",
            false,
        )],
    );

    let report = recover(&fixture);

    assert_eq!(report.stuck, 1);
    assert_eq!(
        fs::read(fixture.path("Mods/Cool/Cool.pak")).expect("read"),
        b"new"
    );
    assert_eq!(
        fs::read(fixture.path("Mods/Cool/Cool.pak-x")).expect("read"),
        b"pak"
    );
}
