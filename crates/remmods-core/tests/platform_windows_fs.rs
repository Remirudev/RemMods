#![cfg(windows)]
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod support;

use std::fs::{self, File, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;
use std::process::Command;

use remmods_core::platform::{self, OsErrorClass, RawName};
use remmods_core::safe_fs::names::escape_wide;
use remmods_core::safe_fs::{BatchOptions, FsError, RetryMode, ScopeViolation};
use serde_json::{Value, json};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
};

use support::options;

fn hold(path: &Path, share: u32) -> File {
    OpenOptions::new()
        .read(true)
        .share_mode(share)
        .open(platform::fs_path(path).expect("fs path"))
        .expect("open for holding")
}

fn record(path: &Path) -> Value {
    match platform::raw_name(path.as_os_str()) {
        RawName::Wide(units) => json!({ "form": "wide", "text": escape_wide(&units) }),
        RawName::Bytes(_) => panic!("Windows names are wide"),
    }
}

#[test]
fn a_pak_held_without_sharing_fails_the_probe_and_nothing_moves() {
    let fixture = support::fs_fixture();
    support::write_tree(
        &fixture.paks,
        &[
            ("Mods/Cool/Cool.pak", b"pak"),
            ("Mods/Cool/Cool.ucas", b"ucas"),
        ],
    );
    let before = fixture.paks_snapshot();
    let held = hold(&fixture.path("Mods/Cool/Cool.ucas"), 0);
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");
    let ops = [
        fixture.op("Mods/Cool/Cool.pak", "Mods/Cool/Cool.pak-x", false),
        fixture.op("Mods/Cool/Cool.ucas", "Mods/Cool/Cool.ucas-x", false),
    ];

    let result = guard.rename_batch(&ops, &options(None));

    assert!(
        matches!(
            result,
            Err(FsError::Io {
                op: "probe",
                class: OsErrorClass::InUse,
                ..
            })
        ),
        "{result:?}"
    );
    // The snapshot reads every file, which the exclusive handle would refuse.
    drop(held);
    assert_eq!(fixture.paks_snapshot(), before);
}

#[test]
fn a_shared_handle_inside_a_folder_makes_the_folder_rename_busy() {
    let fixture = support::fs_fixture();
    support::write_tree(&fixture.paks, &[("LogicMods/X/X.pak", b"x")]);
    let before = fixture.paks_snapshot();
    let _held = hold(
        &fixture.path("LogicMods/X/X.pak"),
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
    );
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");
    let ops = [
        fixture.op("LogicMods/X", "LogicMods/.disabled_X", true),
        fixture.op(
            "LogicMods/.disabled_X/X.pak",
            "LogicMods/.disabled_X/X.pak-x",
            false,
        ),
    ];
    let never = BatchOptions {
        label: "test",
        retry: RetryMode::Never,
        fault: None,
    };

    let result = guard.rename_batch(&ops, &never);

    match result {
        Err(FsError::Batch(error)) => {
            assert!(
                matches!(error.cause, FsError::FolderBusy { .. }),
                "{}",
                error.cause
            );
        }
        other => panic!("expected a busy folder, got {other:?}"),
    }
    assert_eq!(fixture.paks_snapshot(), before);
}

#[test]
fn a_trailing_dot_folder_renames_through_verbatim_paths() {
    let fixture = support::fs_fixture();
    support::write_tree(&fixture.paks, &[("LogicMods/Foo./Foo.pak", b"f")]);
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");
    let ops = [fixture.op("LogicMods/Foo.", "LogicMods/.disabled_Foo.", true)];

    guard.rename_batch(&ops, &options(None)).expect("batch");

    assert_eq!(
        support::names_in(&fixture.path("LogicMods")),
        [".disabled_Foo."]
    );
}

#[test]
fn an_8_3_alias_of_the_temp_dir_is_in_scope() {
    let fixture = support::fs_fixture();
    let alias_root = fixture.dir.path();
    let scope = fixture.scope();

    let result = scope.check(&alias_root.join("Paks").join("Mods").join("a.pak"));

    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn a_mod_with_paths_over_260_characters_toggles_off_and_on() {
    let fixture = support::fs_fixture();
    let deep = [
        "a".repeat(60),
        "b".repeat(60),
        "c".repeat(60),
        "d".repeat(60),
    ]
    .join("/");
    let rel = format!("Mods/{deep}/Long.pak");
    support::write_tree(&fixture.paks, &[(rel.as_str(), b"long")]);
    assert!(fixture.path(&rel).as_os_str().len() > 260);
    let off = [fixture.op(&rel, &format!("{rel}-x"), false)];
    let on = [fixture.op(&format!("{rel}-x"), &rel, false)];
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");

    guard.rename_batch(&off, &options(None)).expect("off");
    guard.rename_batch(&on, &options(None)).expect("on");

    let path = platform::fs_path(&fixture.path(&rel)).expect("fs path");
    assert_eq!(fs::read(path).expect("read"), b"long");
}

#[test]
fn a_junction_mod_folder_is_a_link_component() {
    let fixture = support::fs_fixture();
    support::write_tree(&fixture.root, &[("elsewhere/J/J.pak", b"j")]);
    let status = Command::new("cmd")
        .arg("/C")
        .arg("mklink")
        .arg("/J")
        .arg(fixture.path("Mods/J"))
        .arg(fixture.root.join("elsewhere").join("J"))
        .status()
        .expect("mklink");
    assert!(status.success());

    let result = fixture.scope().check(&fixture.path("Mods/J/J.pak"));

    assert!(matches!(
        result,
        Err(FsError::OutOfScope {
            reason: ScopeViolation::LinkComponent,
            ..
        })
    ));
}

#[test]
fn two_case_spellings_of_one_entry_are_stuck_not_removed() {
    let fixture = support::fs_fixture();
    support::write_tree(&fixture.paks, &[("Mods/Foo/Foo.pak", b"pak")]);
    let journal = json!({
        "version": 1,
        "revision": 1,
        "entries": [{
            "id": "fedcba9876543210fedcba9876543210",
            "label": "toggle",
            "owner": { "pid": 1, "session": "0123456789abcdef0123456789abcdef" },
            "createdAt": "2026-11-01T12:00:00.000Z",
            "stuck": false,
            "ops": [{
                "kind": "rename",
                "isDir": false,
                "from": record(&fixture.path("Mods/Foo/Foo.pak")),
                "to": record(&fixture.path("Mods/Foo/foo.pak")),
            }],
        }],
    });
    fs::write(
        fixture.journal_path(),
        serde_json::to_vec(&journal).expect("encode"),
    )
    .expect("write journal");
    let gate = fixture.gate();

    let guard = gate.write(fixture.scope()).expect("write guard");

    assert_eq!(guard.recovery().stuck, 1);
    assert_eq!(support::names_in(&fixture.path("Mods/Foo")), ["Foo.pak"]);
}
