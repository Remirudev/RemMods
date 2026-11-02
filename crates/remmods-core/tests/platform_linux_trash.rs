//! The freedesktop.org Trash mover. Every trash here lives inside the test's TempDir.
#![cfg(target_os = "linux")]
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod support;

use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, TimeZone};
use remmods_core::platform::{
    self, OsErrorClass, TopdirPolicy, TrashEnv, TrashKind, TrashUnavailableReason, trash_location,
};
use remmods_core::safe_fs::{
    FaultPlan, FsError, Scope, TrashReport, percent_encode_path, trashinfo_text,
};

use support::{FsFixture, tree_listing};

fn now() -> DateTime<Local> {
    Local
        .with_ymd_and_hms(2026, 11, 1, 12, 34, 56)
        .single()
        .expect("unambiguous time")
}

fn uid() -> u32 {
    platform::current_uid().expect("Linux has a uid")
}

fn run_trash(
    fixture: &FsFixture,
    scope: Scope,
    env: &TrashEnv,
    items: &[PathBuf],
    fault: Option<&FaultPlan>,
) -> Result<TrashReport, FsError> {
    let gate = fixture.gate();
    let guard = gate.write(scope).expect("write guard");
    guard.trash_with_fault(env, items, now(), fault)
}

fn unavailable(result: Result<TrashReport, FsError>) -> TrashUnavailableReason {
    match result {
        Err(FsError::TrashUnavailable { reason }) => reason,
        other => panic!("expected TrashUnavailable, got {other:?}"),
    }
}

/// `<root>/top/Paks` with the three mod roots; the topdir tests treat `<root>/top` as the mount.
fn topdir_fixture() -> (FsFixture, PathBuf, Scope) {
    let fixture = support::fs_fixture();
    let top = fixture.root.join("top");
    let paks = top.join("Paks");
    support::write_tree(
        &paks,
        &[
            ("Mods/Cool/Cool.pak", b"pak"),
            ("~mods/", b""),
            ("LogicMods/", b""),
        ],
    );
    let scope = Scope::for_paks(&paks, &[&fixture.app.data_dir]).expect("scope");
    (fixture, top, scope)
}

fn topdir_env(top: &Path) -> TrashEnv {
    TrashEnv {
        home_trash: None,
        uid: Some(uid()),
        topdirs: TopdirPolicy::Fixed(top.to_path_buf()),
    }
}

fn decode_percent(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).expect("hex");
            out.push(u8::from_str_radix(hex, 16).expect("hex byte"));
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    out
}

fn path_field(info: &Path) -> String {
    let text = fs::read_to_string(info).expect("trashinfo");
    text.lines()
        .find_map(|line| line.strip_prefix("Path="))
        .expect("Path= line")
        .to_owned()
}

#[test]
fn trash_refuses_other_device_without_writing() {
    let fixture = support::fs_fixture();
    support::write_tree(&fixture.paks, &[("Mods/Cool/Cool.pak", b"pak")]);
    let env = TrashEnv {
        home_trash: None,
        uid: Some(uid()),
        topdirs: TopdirPolicy::Deny,
    };
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");
    let before = tree_listing(&fixture.root);

    let result = guard.trash(&env, &[fixture.path("Mods/Cool")], now());

    assert_eq!(unavailable(result), TrashUnavailableReason::OtherDevice);
    assert_eq!(tree_listing(&fixture.root), before);
}

#[test]
fn trash_moves_folder_into_home_trash() {
    let fixture = support::fs_fixture();
    support::write_tree(
        &fixture.paks,
        &[
            ("Mods/Cool/Cool.pak", b"pak"),
            ("Mods/Cool/sub/readme.txt", b"r"),
        ],
    );
    let env = support::trash_env(&fixture.root);
    let trash = env.home_trash.clone().expect("home trash");
    let item = fixture.path("Mods/Cool");
    let inode = fs::symlink_metadata(&item).expect("meta").ino();
    let tree = tree_listing(&item);

    let report = run_trash(
        &fixture,
        fixture.scope(),
        &env,
        std::slice::from_ref(&item),
        None,
    )
    .expect("trashed");

    assert_eq!(report.kind, TrashKind::Home);
    let trashed = trash.join("files").join("Cool");
    let info = trash.join("info").join("Cool.trashinfo");
    assert_eq!(report.items.len(), 1);
    assert_eq!(report.items[0].trashed, trashed);
    assert_eq!(report.items[0].info, info);
    assert!(!item.exists());
    assert_eq!(tree_listing(&trashed), tree);
    assert_eq!(fs::symlink_metadata(&trashed).expect("meta").ino(), inode);
    let expected = trashinfo_text(&percent_encode_path(item.as_os_str().as_bytes()), now());
    assert_eq!(fs::read_to_string(&info).expect("info"), expected);
    assert!(path_field(&info).starts_with('/'));
}

#[test]
fn trash_never_replaces_existing_name() {
    let fixture = support::fs_fixture();
    support::write_tree(&fixture.paks, &[("Mods/Cool/Cool.pak", b"pak")]);
    let env = support::trash_env(&fixture.root);
    let trash = env.home_trash.clone().expect("home trash");
    support::write_tree(
        &trash,
        &[
            ("files/Cool/old.txt", b"old"),
            ("info/Cool.trashinfo", b"old info"),
        ],
    );

    let report = run_trash(
        &fixture,
        fixture.scope(),
        &env,
        &[fixture.path("Mods/Cool")],
        None,
    )
    .expect("trashed");

    assert_eq!(report.items[0].trashed, trash.join("files").join("Cool.2"));
    assert_eq!(
        fs::read(trash.join("files/Cool/old.txt")).expect("old"),
        b"old"
    );
    assert_eq!(
        fs::read(trash.join("info/Cool.trashinfo")).expect("old info"),
        b"old info"
    );

    let second = support::fs_fixture();
    support::write_tree(&second.paks, &[("Mods/Cool/Cool.pak", b"pak")]);
    let env = support::trash_env(&second.root);
    let trash = env.home_trash.clone().expect("home trash");
    support::write_tree(
        &trash,
        &[
            ("files/Cool/a.txt", b"a"),
            ("files/Cool.2/b.txt", b"b"),
            ("info/", b""),
        ],
    );

    let report = run_trash(
        &second,
        second.scope(),
        &env,
        &[second.path("Mods/Cool")],
        None,
    )
    .expect("trashed");

    assert_eq!(report.items[0].trashed, trash.join("files").join("Cool.3"));
    assert_eq!(
        report.items[0].info,
        trash.join("info").join("Cool.3.trashinfo")
    );
    assert_eq!(support::names_in(&trash.join("info")), ["Cool.3.trashinfo"]);
    assert_eq!(fs::read(trash.join("files/Cool.2/b.txt")).expect("b"), b"b");
}

#[test]
fn trash_group_rolls_back_on_failure() {
    let fixture = support::fs_fixture();
    support::write_tree(
        &fixture.paks,
        &[
            ("Mods/A.pak", b"pak"),
            ("Mods/A.utoc", b"utoc"),
            ("Mods/A.ucas", b"ucas"),
        ],
    );
    let env = support::trash_env(&fixture.root);
    let trash = env.home_trash.clone().expect("home trash");
    support::write_tree(&trash, &[("files/", b""), ("info/", b"")]);
    let items = ["Mods/A.pak", "Mods/A.utoc", "Mods/A.ucas"].map(|rel| fixture.path(rel));
    let paks_before = tree_listing(&fixture.paks);
    let trash_before = tree_listing(&trash);
    let fault = FaultPlan {
        fail_before: Some(1),
        ..FaultPlan::default()
    };

    let result = run_trash(&fixture, fixture.scope(), &env, &items, Some(&fault));

    match result {
        Err(FsError::Batch(batch)) => {
            assert_eq!(batch.failed, 1);
            assert_eq!(batch.rolled_back, 1);
            assert!(batch.stuck.is_empty());
        }
        other => panic!("expected a Batch error, got {other:?}"),
    }
    assert_eq!(tree_listing(&fixture.paks), paks_before);
    assert_eq!(tree_listing(&trash), trash_before);
}

#[test]
fn trash_topdir_user_created_0700() {
    let (fixture, top, scope) = topdir_fixture();
    let item = top.join("Paks").join("Mods").join("Cool");

    let report = run_trash(&fixture, scope, &topdir_env(&top), &[item], None).expect("trashed");

    let root = top.join(format!(".Trash-{}", uid()));
    assert_eq!(report.kind, TrashKind::TopdirUser);
    assert_eq!(report.items[0].trashed, root.join("files").join("Cool"));
    let mode = fs::metadata(&root).expect("meta").permissions().mode();
    assert_eq!(mode & 0o777, 0o700);
    assert_eq!(path_field(&report.items[0].info), "Paks/Mods/Cool");
}

#[test]
fn trash_topdir_shared_requires_sticky() {
    let (fixture, top, scope) = topdir_fixture();
    fs::create_dir(top.join(".Trash")).expect("shared trash");
    let item = top.join("Paks").join("Mods").join("Cool");

    let report =
        run_trash(&fixture, scope.clone(), &topdir_env(&top), &[item], None).expect("trashed");

    assert_eq!(report.kind, TrashKind::TopdirUser);
    assert!(
        top.join(format!(".Trash-{}", uid()))
            .join("files/Cool")
            .is_dir()
    );
    assert!(support::names_in(&top.join(".Trash")).is_empty());

    support::write_tree(&top, &[("Paks/Mods/Second/Second.pak", b"p")]);
    fs::set_permissions(top.join(".Trash"), fs::Permissions::from_mode(0o1777)).expect("chmod");
    let item = top.join("Paks").join("Mods").join("Second");

    let report = run_trash(&fixture, scope, &topdir_env(&top), &[item], None).expect("trashed");

    let root = top.join(".Trash").join(uid().to_string());
    assert_eq!(report.kind, TrashKind::TopdirShared);
    assert_eq!(report.items[0].trashed, root.join("files").join("Second"));
    assert_eq!(path_field(&report.items[0].info), "Paks/Mods/Second");
}

#[test]
fn trash_topdir_rejects_symlinked_trash() {
    let (fixture, top, scope) = topdir_fixture();
    let elsewhere = fixture.root.join("elsewhere");
    fs::create_dir(&elsewhere).expect("elsewhere");
    symlink(&elsewhere, top.join(format!(".Trash-{}", uid()))).expect("symlink");
    let item = top.join("Paks").join("Mods").join("Cool");
    let before = tree_listing(&fixture.root);

    let result = run_trash(&fixture, scope, &topdir_env(&top), &[item], None);

    assert_eq!(unavailable(result), TrashUnavailableReason::NoUsableTrash);
    assert!(support::names_in(&elsewhere).is_empty());
    let after: Vec<String> = tree_listing(&fixture.root)
        .into_iter()
        .filter(|line| !line.contains("app/"))
        .collect();
    let before: Vec<String> = before
        .into_iter()
        .filter(|line| !line.contains("app/"))
        .collect();
    assert_eq!(after, before);
}

#[test]
fn trash_non_utf8_name_round_trips() {
    let fixture = support::fs_fixture();
    let raw: &[u8] = b"Caf\xe9 #1";
    let item = fixture.paks.join("Mods").join(OsStr::from_bytes(raw));
    fs::create_dir(&item).expect("raw-named folder");
    fs::write(item.join("x.pak"), b"x").expect("pak");
    let env = support::trash_env(&fixture.root);

    let report = run_trash(
        &fixture,
        fixture.scope(),
        &env,
        std::slice::from_ref(&item),
        None,
    )
    .expect("trashed");

    let field = path_field(&report.items[0].info);
    assert!(field.ends_with("/Mods/Caf%E9%20%231"), "{field}");
    assert_eq!(decode_percent(&field), item.as_os_str().as_bytes());
    assert_eq!(report.items[0].trashed.file_name(), item.file_name());
}

#[test]
fn trashinfo_text_exact() {
    assert_eq!(
        trashinfo_text("/home/u/Paks/Mods/Cool", now()),
        "[Trash Info]\nPath=/home/u/Paks/Mods/Cool\nDeletionDate=2026-11-01T12:34:56\n"
    );
}

#[test]
fn percent_encode_path_table() {
    let table: &[(&[u8], &str)] = &[
        (b"a b", "a%20b"),
        (b"100%", "100%25"),
        (b"#tag", "%23tag"),
        (b"what?", "what%3F"),
        (b"Mods/Cool.pak", "Mods/Cool.pak"),
        (b"A-Z_a.z~0", "A-Z_a.z~0"),
        (b"!$&'()*+,;=:@", "!$&'()*+,;=:@"),
        ("é".as_bytes(), "%C3%A9"),
        ("日".as_bytes(), "%E6%97%A5"),
        (b"\xff", "%FF"),
        (b"[x]\"<>\\^`{|}", "%5Bx%5D%22%3C%3E%5C%5E%60%7B%7C%7D"),
    ];
    for (bytes, expected) in table {
        assert_eq!(percent_encode_path(bytes), *expected, "{bytes:?}");
    }
}

#[test]
fn trash_location_writes_nothing() {
    let (fixture, top, _scope) = topdir_fixture();
    let home_env = support::trash_env(&fixture.root);
    let item = top.join("Paks").join("Mods").join("Cool");
    let before = tree_listing(&fixture.root);

    let home = trash_location(&home_env, &item)
        .expect("probe")
        .expect("home trash");
    let user = trash_location(&topdir_env(&top), &item)
        .expect("probe")
        .expect("topdir trash");

    assert_eq!(home.kind, TrashKind::Home);
    assert_eq!(home.missing.len(), 3);
    assert_eq!(user.kind, TrashKind::TopdirUser);
    assert_eq!(user.missing.len(), 3);
    assert_eq!(tree_listing(&fixture.root), before);
}

/// L1: the Recycle Bin mover exists only on Windows; nothing moves here.
#[test]
fn recycle_is_unsupported_on_linux() {
    let fixture = support::fs_fixture();
    support::write_tree(&fixture.paks, &[("Mods/Cool/Cool.pak", b"pak")]);
    let before = tree_listing(&fixture.paks);
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");

    for force_abort in [false, true] {
        match guard.recycle_forced(&fixture.path("Mods/Cool"), force_abort) {
            Err(FsError::Io { class, .. }) => assert_eq!(class, OsErrorClass::Unsupported),
            other => panic!("expected an Unsupported io error, got {other:?}"),
        }
    }
    drop(guard);
    assert_eq!(tree_listing(&fixture.paks), before);
    assert!(matches!(
        platform::enter_mta_for_tests(),
        Err(error) if platform::classify(&error) == OsErrorClass::Unsupported
    ));
}
