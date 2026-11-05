//! Linux-only manage cases (spec §8.2 D10, R3, R4, N11, I2, P2): real moves into a TempDir
//! trash, non-UTF-8 names and read-only folders.
#![cfg(target_os = "linux")]
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod mods_support;
mod profiles_support;
mod support;

use std::ffi::OsStr;
use std::fs::{self, Permissions};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

use mods_support::DATA;
use mods_support::manage::{core_with_trash, trash_dir, tree};
use profiles_support::{apply, custom};
use remmods_core::CoreError;
use remmods_core::mods::{
    DeleteRequest, MetadataRequest, ModInfoEdit, ModIssue, ModTarget, RenameRequest, RepairKind,
    TrashRequest, repair_items,
};
use remmods_core::platform::TrashKind;
use remmods_core::profiles::{EntryState, ProfileMode};
use remmods_core::safe_fs::{FaultPlan, FsError};

/// Permission bits mean nothing to root; such runs skip the permission tests.
fn permissions_enforced(probe: &Path) -> bool {
    fs::set_permissions(probe, Permissions::from_mode(0o555)).expect("chmod probe");
    let enforced = fs::write(probe.join("probe"), b"").is_err();
    let _ = fs::remove_file(probe.join("probe"));
    fs::set_permissions(probe, Permissions::from_mode(0o755)).expect("restore probe");
    enforced
}

fn chmod(path: &Path, mode: u32) {
    fs::set_permissions(path, Permissions::from_mode(mode)).expect("chmod");
}

fn trash(
    fixture: &mods_support::manage::ToggleFixture,
    key: &str,
) -> remmods_core::mods::TrashOutcome {
    fixture
        .core
        .mods_trash(&TrashRequest {
            target: fixture.target(key),
            apply_while_running: false,
        })
        .expect("trash")
}

#[test]
fn delete_removal_failure_leaves_hidden_inert_leftover() {
    let fixture = core_with_trash(&[("Mods/Cool/Cool.pak", DATA), ("Mods/Cool/Sub/x.txt", DATA)]);
    let sub = fixture.paks.join("Mods").join("Cool").join("Sub");
    if !permissions_enforced(&sub) {
        return;
    }
    chmod(&sub, 0o555);
    let target = fixture.target("Standard:Cool");
    let token = fixture
        .core
        .mods_delete_prepare(&target)
        .expect("prepare")
        .token;
    let outcome = fixture.core.mods_delete_permanently(&DeleteRequest {
        target,
        token,
        apply_while_running: false,
    });
    let snapshot = fixture.snapshot();
    let leftovers: Vec<&str> = snapshot
        .content
        .leftovers
        .iter()
        .map(|entry| entry.rel_path.as_str())
        .collect();
    let aside = fixture.paks.join(leftovers.first().expect("a leftover"));
    chmod(&aside.join("Sub"), 0o755);
    let outcome = outcome.expect("delete reports the leftover");
    assert_eq!(outcome.leftover.as_deref(), leftovers.first().copied());
    assert!(leftovers[0].starts_with("Mods/_Cool_tmp_"), "{leftovers:?}");
    assert!(snapshot.content.rows.is_empty());
    assert!(
        snapshot.content.unmanaged.is_empty(),
        "no live payload anywhere"
    );
}

#[test]
fn trash_folder_row_moves_whole_folder() {
    let fixture = core_with_trash(&[
        ("Mods/Cool/Cool.pak", DATA),
        ("Mods/Cool/Sub/readme.txt", DATA),
        ("Mods/Keep/Keep.pak", DATA),
    ]);
    let inode = fs::metadata(fixture.paks.join("Mods").join("Cool"))
        .expect("meta")
        .ino();
    let outcome = trash(&fixture, "Standard:Cool");
    assert_eq!((outcome.items, outcome.kind), (1, TrashKind::Home));
    assert_eq!(outcome.key, "Standard:Cool");
    let trashed = trash_dir(&fixture).join("files").join("Cool");
    assert_eq!(fs::metadata(&trashed).expect("trashed").ino(), inode);
    assert!(trashed.join("Sub").join("readme.txt").is_file());
    let info = fs::read_to_string(trash_dir(&fixture).join("info").join("Cool.trashinfo"))
        .expect("trashinfo");
    let original = fixture.paks.join("Mods").join("Cool");
    let expected_path = format!(
        "Path={}\n",
        remmods_core::safe_fs::percent_encode_path(original.as_os_str().as_bytes())
    );
    assert!(info.contains(&expected_path), "{info}");
    assert!(!fixture.exists("Mods/Cool"));
    assert!(fixture.exists("Mods/Keep/Keep.pak"));
    assert!(
        outcome
            .refreshed
            .snapshot
            .content
            .rows
            .iter()
            .all(|row| row.key != "Standard:Cool")
    );
}

#[test]
fn trash_loose_group_pak_first() {
    let fixture = core_with_trash(&[
        ("Mods/Loose.ucas", DATA),
        ("Mods/Loose.pak", DATA),
        ("Mods/Loose.utoc", DATA),
    ]);
    // A fault on the first item names it in the error: the `.pak` goes first.
    fixture.core.inject_batch_fault(FaultPlan {
        fail_before: Some(0),
        ..FaultPlan::default()
    });
    let failed = fixture.core.mods_trash(&TrashRequest {
        target: fixture.target("Loose:Mods/Loose"),
        apply_while_running: false,
    });
    match failed {
        Err(CoreError::Mods(remmods_core::mods::ModsError::Fs(FsError::Io { path, .. }))) => {
            assert_eq!(path.file_name(), Some(OsStr::new("Loose.pak")));
        }
        other => panic!("expected the injected error, got {other:?}"),
    }
    let trash_files = trash_dir(&fixture).join("files");
    assert!(support::names_in(&trash_files).is_empty());
    let outcome = trash(&fixture, "Loose:Mods/Loose");
    assert_eq!(outcome.items, 3);
    assert_eq!(
        support::names_in(&trash_files),
        ["Loose.pak", "Loose.ucas", "Loose.utoc"]
    );
}

#[test]
fn rename_non_unicode_row() {
    let fixture = core_with_trash(&[("Mods/Keep/Keep.pak", DATA)]);
    let bad = fixture
        .paks
        .join("Mods")
        .join(OsStr::from_bytes(b"bad\xFFname"));
    fs::create_dir(&bad).expect("mkdir");
    fs::write(bad.join("a.pak"), DATA).expect("write");
    let snapshot = fixture.snapshot();
    let row = snapshot
        .content
        .rows
        .iter()
        .find(|row| row.issues.contains(&ModIssue::NonUnicodeName))
        .expect("non-Unicode row");
    let item = repair_items(&snapshot.content, 0)
        .into_iter()
        .find(|item| item.kind == RepairKind::NonUnicodeName)
        .expect("repair item");
    let suggested = item.suggested_name.expect("a suggestion");
    let outcome = fixture
        .core
        .mods_rename(&RenameRequest {
            target: ModTarget {
                mod_ref: row.mod_ref.clone(),
                key: row.key.clone(),
            },
            new_name: suggested.clone(),
            apply_while_running: false,
        })
        .expect("rename");
    assert_eq!(outcome.new_key, format!("Standard:{suggested}"));
    assert!(!bad.exists());
    assert!(
        fixture
            .paks
            .join("Mods")
            .join(&suggested)
            .join("a.pak")
            .is_file()
    );
}

#[test]
fn metadata_backup_failure_aborts() {
    let fixture = core_with_trash(&[
        ("Mods/Cool/Cool.pak", DATA),
        ("Mods/Cool/modinfo.json", b"{ broken"),
    ]);
    let folder = fixture.paks.join("Mods").join("Cool");
    if !permissions_enforced(&folder) {
        return;
    }
    chmod(&folder, 0o555);
    let before = tree(&fixture);
    let result = fixture.core.mods_set_metadata(&MetadataRequest {
        target: fixture.target("Standard:Cool"),
        info: ModInfoEdit {
            version: "2".into(),
            category: "c".into(),
            description: "d".into(),
            author: "a".into(),
        },
    });
    let after = tree(&fixture);
    chmod(&folder, 0o755);
    assert!(result.is_err());
    assert_eq!(after, before);
    assert_eq!(
        fs::read(folder.join("modinfo.json")).expect("read"),
        b"{ broken"
    );
}

#[test]
fn apply_snapshot_write_failure_aborts() {
    let fixture = core_with_trash(&[("Mods/A/A.pak", DATA), ("Mods/B/B.pak", DATA)]);
    let id = custom(
        &fixture,
        "Off",
        ProfileMode::Additive,
        &[("Standard:A", EntryState::Disabled)],
    );
    fixture.snapshot();
    let data = fixture.paths.data_dir.clone();
    if !permissions_enforced(&data) {
        return;
    }
    let before = tree(&fixture);
    let profiles = fs::read(fixture.paths.profiles_file()).expect("profiles");
    chmod(&data, 0o555);
    let result = apply(&fixture, &id);
    chmod(&data, 0o755);
    assert!(result.is_err(), "{result:?}");
    assert_eq!(tree(&fixture), before);
    assert_eq!(
        fs::read(fixture.paths.profiles_file()).expect("profiles"),
        profiles
    );
}
