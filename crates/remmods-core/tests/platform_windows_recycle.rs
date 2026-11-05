//! The Windows Recycle Bin mover (P3b §4.8). Abort, never delete: tier 1 needs no working bin and
//! proves that refused, rewritten or overlong items stay byte for byte; tier 2 recycles
//! `remmods-recycle-test-*` items of the test's TempDir into the runner's own bin (they stay there).
//! Only this file builds a `Detect` env: in cfg-free test code it would reach a real trash on Linux.
#![cfg(windows)]
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod mods_support;
mod support;

use std::fs::{self, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::thread;

use chrono::{DateTime, Local};
use mods_support::manage::{Extras, ToggleFixture};
use mods_support::{DATA, MOUNT, toc};
use remmods_core::CoreError;
use remmods_core::mods::{
    BulkDeleteRequest, BulkRowResult, BulkTrashRequest, DeleteRequest, ModsError, TrashRequest,
};
use remmods_core::platform::{
    self, OsErrorClass, Recycled, TopdirPolicy, TrashEnv, TrashKind, TrashUnavailableReason,
};
use remmods_core::safe_fs::{FaultPlan, FsError, Scope, TrashReport};

use support::{FsFixture, tree_listing};

const PREFIX: &str = "remmods-recycle-test-";
/// A folder name the shell namespace parses as a junction into a virtual folder.
const CLSID_FOLDER: &str = "remmods-recycle-test-x.{ED7BA470-8E54-465E-825C-99712043E01C}";

fn recycle_env() -> TrashEnv {
    TrashEnv {
        home_trash: None,
        uid: None,
        topdirs: TopdirPolicy::Detect,
    }
}

fn now() -> DateTime<Local> {
    Local::now()
}

fn name(suffix: &str) -> String {
    format!("{PREFIX}{suffix}")
}

/// `Mods/<prefix><suffix>/{a.pak,b.txt}`; returns the folder.
fn test_folder(fixture: &FsFixture, suffix: &str) -> PathBuf {
    let folder = format!("Mods/{}", name(suffix));
    support::write_tree(
        &fixture.paks,
        &[
            (&format!("{folder}/a.pak"), b"pak bytes"),
            (&format!("{folder}/b.txt"), b"text bytes"),
        ],
    );
    fixture.path(&folder)
}

fn exists(path: &Path) -> bool {
    platform::fs_path(path)
        .and_then(fs::symlink_metadata)
        .is_ok()
}

fn read(path: &Path) -> Vec<u8> {
    fs::read(platform::fs_path(path).expect("fs path")).expect("read")
}

fn unavailable(result: Result<TrashReport, FsError>) -> TrashUnavailableReason {
    match result {
        Err(FsError::TrashUnavailable { reason }) => reason,
        other => panic!("expected TrashUnavailable, got {other:?}"),
    }
}

fn recycle_forced(fixture: &FsFixture, item: &Path, force_abort: bool) -> Recycled {
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");
    guard
        .recycle_forced(item, force_abort)
        .expect("recycle_forced")
}

fn trash(
    fixture: &FsFixture,
    items: &[PathBuf],
    fault: Option<&FaultPlan>,
) -> Result<TrashReport, FsError> {
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");
    guard.trash_with_fault(&recycle_env(), items, now(), fault)
}

fn in_recycle_bin(path: &Path) -> bool {
    path.to_string_lossy()
        .to_ascii_lowercase()
        .contains("$recycle.bin")
}

/// A Core whose trash env is the Recycle Bin.
fn recycle_core(entries: &[(&str, &[u8])]) -> ToggleFixture {
    ToggleFixture::build(entries, |_| Extras {
        trash: Some(recycle_env()),
        ..Extras::default()
    })
}

/// `<key>/x.pak` + `<key>/x.utoc` for each folder.
fn pak_folders(folders: &[&str]) -> Vec<(String, Vec<u8>)> {
    folders
        .iter()
        .flat_map(|folder| {
            [
                (format!("Mods/{folder}/x.pak"), DATA.to_vec()),
                (
                    format!("Mods/{folder}/x.utoc"),
                    toc(MOUNT, &["Character/x.uasset"]),
                ),
            ]
        })
        .collect()
}

fn entries(files: &[(String, Vec<u8>)]) -> Vec<(&str, &[u8])> {
    files
        .iter()
        .map(|(path, bytes)| (path.as_str(), bytes.as_slice()))
        .collect()
}

fn folder_listing(fixture: &ToggleFixture, folder: &str) -> Vec<String> {
    tree_listing(&fixture.paks.join("Mods").join(folder))
}

fn mods_names(fixture: &ToggleFixture) -> Vec<String> {
    support::names_in(&fixture.paks.join("Mods"))
}

// ---- Tier 1: abort, never delete ----

/// W1
#[test]
fn forced_abort_keeps_item_byte_for_byte() {
    let fixture = support::fs_fixture();
    let item = test_folder(&fixture, "w1");
    let before = tree_listing(&fixture.paks);

    let result = recycle_forced(&fixture, &item, true);

    assert_eq!(result, Recycled::Refused);
    assert_eq!(tree_listing(&fixture.paks), before);
}

/// W2
#[test]
fn guard_trash_abort_reports_recycle_refused() {
    let fixture = support::fs_fixture();
    let item = test_folder(&fixture, "w2");
    let before = tree_listing(&fixture.paks);
    let fault = FaultPlan {
        abort_recycle: Some(0),
        ..FaultPlan::default()
    };

    let result = trash(&fixture, &[item], Some(&fault));

    assert_eq!(unavailable(result), TrashUnavailableReason::RecycleRefused);
    assert_eq!(tree_listing(&fixture.paks), before);
}

/// `C:\…` → `\\localhost\C$\…`.
fn admin_share(path: &Path) -> PathBuf {
    let text = path.to_str().expect("Unicode temp path");
    let (drive, rest) = text.split_at(2);
    assert!(drive.ends_with(':'), "a drive path: {text}");
    PathBuf::from(format!(r"\\localhost\{}${rest}", &drive[..1]))
}

/// W3: a network path has no Recycle Bin; the shell's permanent delete is refused.
#[test]
fn admin_share_is_never_deleted() {
    let fixture = support::fs_fixture();
    let item = test_folder(&fixture, "w3");
    let unc_paks = admin_share(&fixture.paks);
    let unc_item = unc_paks.join("Mods").join(name("w3"));
    if !exists(&unc_item) {
        println!("SKIP: admin share unavailable");
        return;
    }
    let before = tree_listing(&fixture.paks);
    let before_unc = tree_listing(&unc_paks);
    let gate = fixture.gate();
    let scope = Scope::from_roots(&[unc_paks.join("Mods")], &[]).expect("scope");
    let guard = gate.write(scope).expect("write guard");

    let result = guard.recycle_forced(&unc_item, false);
    drop(guard);

    match &result {
        Ok(Recycled::Refused) => println!("admin share: refused by the sink"),
        Ok(Recycled::NotShellSafe) => println!("admin share: not shell safe"),
        Err(FsError::Io { class, source, .. }) => {
            println!("admin share: io error {class:?}: {source}");
        }
        other => panic!("an admin share item must never be recycled or deleted: {other:?}"),
    }
    assert_eq!(tree_listing(&unc_paks), before_unc);
    assert_eq!(tree_listing(&fixture.paks), before);
    assert!(exists(&item));
}

/// W4: names Win32 rewrites never reach the shell, and their Win32 twins stay untouched.
#[test]
fn rewritten_names_never_reach_the_shell() {
    let fixture = support::fs_fixture();
    let dotted = name("Foo.");
    let spaced = name("Bar ");
    let superscript = "COM\u{B9}";
    let files = [
        format!("Mods/{}/a.pak", name("Foo")),
        format!("Mods/{dotted}/a.pak"),
        format!("Mods/{}/a.pak", name("Bar")),
        format!("Mods/{spaced}/a.pak"),
        "Mods/CON/a.pak".to_owned(),
        format!("Mods/{superscript}/a.pak"),
        format!("Mods/{CLSID_FOLDER}/a.pak"),
    ];
    let tree: Vec<(&str, &[u8])> = files
        .iter()
        .map(|path| (path.as_str(), b"bytes".as_slice()))
        .collect();
    support::write_tree(&fixture.paks, &tree);
    let before = tree_listing(&fixture.paks);

    for folder in [&dotted, &spaced] {
        let item = fixture.path(&format!("Mods/{folder}"));
        assert_eq!(
            recycle_forced(&fixture, &item, true),
            Recycled::NotShellSafe,
            "platform guard for {folder:?}"
        );
        assert_eq!(
            unavailable(trash(&fixture, &[item], None)),
            TrashUnavailableReason::NotShellSafe,
            "name check for {folder:?}"
        );
    }
    for folder in ["CON", superscript] {
        let item = fixture.path(&format!("Mods/{folder}"));
        assert_eq!(
            unavailable(trash(&fixture, &[item], None)),
            TrashUnavailableReason::NotShellSafe,
            "{folder:?}"
        );
    }
    let clsid = fixture.path(&format!("Mods/{CLSID_FOLDER}"));
    match recycle_forced(&fixture, &clsid, true) {
        Recycled::NotShellSafe => println!("CLSID folder: refused by the identity check"),
        Recycled::Refused => println!("CLSID folder: the shell kept it a plain folder"),
        Recycled::Done { bin_path } => panic!("recycled despite force_abort: {bin_path:?}"),
    }
    assert_eq!(tree_listing(&fixture.paks), before);
}

/// W4b: a permanent delete of a row Win32 would rewrite removes that row only.
#[test]
fn delete_of_not_shell_safe_rows_spares_their_twins() {
    let foo = name("Foo");
    let foo_dot = name("Foo.");
    let bar = name("Bar");
    let bar_space = name("Bar ");
    let files = pak_folders(&[&foo, &foo_dot, &bar, &bar_space]);
    let fixture = recycle_core(&entries(&files));
    let before = support::tree_listing(&fixture.paks);
    let foo_before = folder_listing(&fixture, &foo);
    let bar_before = folder_listing(&fixture, &bar);

    let foo_dot_target = fixture.target(&format!("Standard:{foo_dot}"));
    let trashed = fixture.core.mods_trash(&TrashRequest {
        target: foo_dot_target.clone(),
        apply_while_running: false,
    });
    match trashed {
        Err(CoreError::Mods(ModsError::TrashUnavailable { reason, .. })) => {
            assert_eq!(reason, TrashUnavailableReason::NotShellSafe);
        }
        other => panic!("expected TrashUnavailable, got {other:?}"),
    }
    assert_eq!(support::tree_listing(&fixture.paks), before);

    let token = fixture
        .core
        .mods_delete_prepare(&foo_dot_target)
        .expect("prepare")
        .token;
    let outcome = fixture
        .core
        .mods_delete_permanently(&DeleteRequest {
            target: foo_dot_target,
            token,
            apply_while_running: false,
        })
        .expect("delete");
    assert_eq!(outcome.leftover, None);
    assert!(!fixture.exists(&format!("Mods/{foo_dot}")));
    assert_eq!(folder_listing(&fixture, &foo), foo_before);

    let bar_space_target = fixture.target(&format!("Standard:{bar_space}"));
    let prepared = fixture
        .core
        .mods_delete_prepare_many(std::slice::from_ref(&bar_space_target))
        .expect("prepare many");
    let token = prepared.token.expect("the row is eligible");
    let bulk = fixture
        .core
        .mods_delete_many(&BulkDeleteRequest {
            targets: vec![bar_space_target],
            token,
            apply_while_running: false,
        })
        .expect("delete many");
    assert_eq!(bulk.rows.len(), 1);
    assert!(
        matches!(
            bulk.rows[0].result,
            BulkRowResult::Deleted { leftover: None }
        ),
        "{:?}",
        bulk.rows[0].result
    );
    assert!(!fixture.exists(&format!("Mods/{bar_space}")));
    assert_eq!(folder_listing(&fixture, &bar), bar_before);
    let names = mods_names(&fixture);
    assert_eq!(names, vec![bar.clone(), foo.clone()], "no set-aside left");
}

/// W5
#[test]
fn overlong_path_never_reaches_the_shell() {
    let fixture = support::fs_fixture();
    let parent = fixture.path(&format!("Mods/{}", name("w5")));
    let head = parent.to_str().expect("Unicode temp path").len() + 1;
    let long = "a".repeat(270 - head);
    assert!((1..=255).contains(&long.len()), "{}", long.len());
    support::write_tree(
        &fixture.paks,
        &[(&format!("Mods/{}/{long}/a.pak", name("w5")), b"bytes")],
    );
    let item = parent.join(&long);
    assert_eq!(item.to_str().expect("Unicode").encode_utf16().count(), 270);
    let before = tree_listing(&fixture.paks);

    assert_eq!(
        recycle_forced(&fixture, &item, true),
        Recycled::NotShellSafe
    );
    assert_eq!(
        unavailable(trash(&fixture, std::slice::from_ref(&item), None)),
        TrashUnavailableReason::NotShellSafe
    );
    assert_eq!(tree_listing(&fixture.paks), before);
}

// ---- Tier 2: into the runner's own Recycle Bin ----

/// W6: a loose group whose second item is refused keeps the rest in place.
#[test]
fn group_failure_keeps_rest_in_place() {
    let fixture = support::fs_fixture();
    let stem = name("w6");
    let rels = ["pak", "utoc", "ucas"].map(|ext| format!("Mods/{stem}.{ext}"));
    let tree: Vec<(&str, &[u8])> = rels
        .iter()
        .map(|rel| (rel.as_str(), rel.as_bytes()))
        .collect();
    support::write_tree(&fixture.paks, &tree);
    let items: Vec<PathBuf> = rels.iter().map(|rel| fixture.path(rel)).collect();
    let fault = FaultPlan {
        abort_recycle: Some(1),
        ..FaultPlan::default()
    };

    let result = trash(&fixture, &items, Some(&fault));

    let batch = match result {
        Err(FsError::Batch(batch)) => batch,
        other => panic!("expected a Batch error, got {other:?}"),
    };
    assert_eq!((batch.failed, batch.rolled_back), (1, 0));
    assert!(matches!(
        batch.cause,
        FsError::TrashUnavailable {
            reason: TrashUnavailableReason::RecycleRefused
        }
    ));
    assert_eq!(batch.stuck.len(), 1);
    assert!(in_recycle_bin(&batch.stuck[0].from), "{:?}", batch.stuck[0]);
    assert!(exists(&batch.stuck[0].from));
    assert_eq!(batch.stuck[0].to, items[0]);
    assert!(!exists(&items[0]));
    for (item, rel) in items.iter().zip(&rels).skip(1) {
        assert_eq!(read(item), rel.as_bytes());
    }
}

/// W7
#[test]
fn recycles_folder_into_the_bin() {
    let fixture = support::fs_fixture();
    let item = test_folder(&fixture, "w7");

    let bin_path = match recycle_forced(&fixture, &item, false) {
        Recycled::Done { bin_path } => bin_path,
        other => panic!("expected Done, got {other:?}"),
    };

    assert!(!exists(&item));
    assert!(in_recycle_bin(&bin_path), "{bin_path:?}");
    assert_eq!(read(&bin_path.join("a.pak")), b"pak bytes");
    assert_eq!(read(&bin_path.join("b.txt")), b"text bytes");
}

/// W8
#[test]
fn core_trash_uses_the_recycle_bin() {
    let folder = name("w8");
    let fixture = recycle_core(&entries(&pak_folders(&[&folder])));
    let key = format!("Standard:{folder}");

    let outcome = fixture
        .core
        .mods_trash(&TrashRequest {
            target: fixture.target(&key),
            apply_while_running: false,
        })
        .expect("trash");

    assert_eq!((outcome.kind, outcome.items), (TrashKind::RecycleBin, 1));
    assert!(
        outcome
            .refreshed
            .snapshot
            .content
            .rows
            .iter()
            .all(|row| row.key != key)
    );
    assert!(!fixture.exists(&format!("Mods/{folder}")));
}

/// W9
#[test]
fn bulk_trash_into_the_bin() {
    let first = name("w9a");
    let second = name("w9b");
    let fixture = recycle_core(&entries(&pak_folders(&[&first, &second])));
    let targets =
        Vec::from([&first, &second].map(|folder| fixture.target(&format!("Standard:{folder}"))));

    let outcome = fixture
        .core
        .mods_trash_many(&BulkTrashRequest {
            targets,
            apply_while_running: false,
        })
        .expect("bulk trash");

    assert_eq!(outcome.rows.len(), 2);
    for row in &outcome.rows {
        assert!(
            matches!(
                row.result,
                BulkRowResult::Trashed {
                    kind: TrashKind::RecycleBin,
                    ..
                }
            ),
            "{row:?}"
        );
    }
    assert!(!fixture.exists(&format!("Mods/{first}")));
    assert!(!fixture.exists(&format!("Mods/{second}")));
}

/// W10: `recycle` brings its own STA thread, whatever apartment the caller is in.
#[test]
fn caller_apartment_does_not_matter() {
    let done = thread::spawn(|| {
        platform::enter_mta_for_tests().expect("enter the MTA");
        let fixture = support::fs_fixture();
        let item = test_folder(&fixture, "w10");
        let result = recycle_forced(&fixture, &item, false);
        (result, exists(&item))
    })
    .join()
    .expect("test thread");

    assert!(matches!(done.0, Recycled::Done { .. }), "{:?}", done.0);
    assert!(!done.1);
}

/// W12: a held file keeps the folder whole (or, at worst, reports it partially recycled).
#[test]
fn held_file_keeps_folder_whole() {
    let fixture = support::fs_fixture();
    let item = test_folder(&fixture, "w12");
    let held_path = item.join("a.pak");
    let held = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(platform::fs_path(&held_path).expect("fs path"))
        .expect("hold a.pak");
    let before = tree_listing(&fixture.paks);

    let result = trash(&fixture, std::slice::from_ref(&item), None);
    let after = tree_listing(&fixture.paks);
    drop(held);

    match &result {
        Ok(report) => panic!("a held folder must not be trashed: {report:?}"),
        Err(FsError::Io { source, .. }) if source.to_string().contains("partially recycled") => {
            println!("held file: partially recycled");
        }
        Err(FsError::Io { class, source, .. }) => {
            println!("held file: io error {class:?}: {source}");
            assert!(
                matches!(class, OsErrorClass::InUse | OsErrorClass::AccessDenied),
                "{class:?}"
            );
            assert_eq!(after, before);
        }
        Err(FsError::TrashUnavailable {
            reason: TrashUnavailableReason::RecycleRefused,
        }) => {
            println!("held file: the shell asked for a permanent delete, refused");
            assert_eq!(after, before);
        }
        Err(other) => panic!("unexpected error: {other:?}"),
    }
    assert_eq!(read(&held_path), b"pak bytes");
}
