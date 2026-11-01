#![cfg(target_os = "linux")]
#![allow(clippy::disallowed_methods)]

mod support;

use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, symlink};

use remmods_core::platform::OsErrorClass;
use remmods_core::safe_fs::{
    FsError, RenameOp, Scope, ScopeViolation, link_unlink_noreplace, placeholder_noreplace,
};

use support::{crash_after, options};

fn violation(result: Result<(), FsError>) -> Option<ScopeViolation> {
    match result {
        Ok(()) => None,
        Err(FsError::OutOfScope { reason, .. }) => Some(reason),
        Err(other) => panic!("unexpected error {other}"),
    }
}

#[test]
fn the_link_fallback_renames_a_file_and_refuses_an_existing_target() {
    let fixture = support::fs_fixture();
    support::write_tree(
        &fixture.paks,
        &[("Mods/A/a.pak", b"a"), ("Mods/A/b.pak", b"b")],
    );
    let scope = fixture.scope();
    let inode = fs::metadata(fixture.path("Mods/A/a.pak"))
        .expect("meta")
        .ino();

    link_unlink_noreplace(
        &scope,
        &fixture.path("Mods/A/a.pak"),
        &fixture.path("Mods/A/c.pak"),
    )
    .expect("link fallback");
    let refused = link_unlink_noreplace(
        &scope,
        &fixture.path("Mods/A/c.pak"),
        &fixture.path("Mods/A/b.pak"),
    );

    assert_eq!(
        support::names_in(&fixture.path("Mods/A")),
        ["b.pak", "c.pak"]
    );
    assert_eq!(
        fs::metadata(fixture.path("Mods/A/c.pak"))
            .expect("meta")
            .ino(),
        inode
    );
    assert_eq!(
        refused.expect_err("refused").class(),
        Some(OsErrorClass::AlreadyExists)
    );
    assert_eq!(fs::read(fixture.path("Mods/A/b.pak")).expect("read"), b"b");
    assert_eq!(fs::read(fixture.path("Mods/A/c.pak")).expect("read"), b"a");
}

#[test]
fn the_placeholder_fallback_renames_a_folder_and_refuses_an_existing_target() {
    let fixture = support::fs_fixture();
    support::write_tree(
        &fixture.paks,
        &[("LogicMods/X/X.pak", b"x"), ("LogicMods/Y/Y.pak", b"y")],
    );
    let scope = fixture.scope();

    placeholder_noreplace(
        &scope,
        &fixture.path("LogicMods/X"),
        &fixture.path("LogicMods/Z"),
    )
    .expect("placeholder fallback");
    let refused = placeholder_noreplace(
        &scope,
        &fixture.path("LogicMods/Z"),
        &fixture.path("LogicMods/Y"),
    );

    assert_eq!(
        refused.expect_err("refused").class(),
        Some(OsErrorClass::AlreadyExists)
    );
    assert_eq!(support::names_in(&fixture.path("LogicMods")), ["Y", "Z"]);
    assert_eq!(support::names_in(&fixture.path("LogicMods/Z")), ["X.pak"]);
}

#[test]
fn non_utf8_names_survive_a_batch_and_the_journal() {
    let fixture = support::fs_fixture();
    let folder = fixture.path("Mods/Bad");
    fs::create_dir(&folder).expect("mkdir");
    let from = folder.join(OsStr::from_bytes(b"\xff.pak"));
    let to = folder.join(OsStr::from_bytes(b"\xff.pak-x"));
    fs::write(&from, b"pak").expect("write");
    let ops = [RenameOp {
        from: from.clone(),
        to: to.clone(),
        is_dir: false,
    }];
    {
        let gate = fixture.gate();
        let guard = gate.write(fixture.scope()).expect("write guard");
        guard.rename_batch(&ops, &options(None)).expect("batch");
        assert!(to.exists() && !from.exists());
        let undo = [RenameOp {
            from: to.clone(),
            to: from.clone(),
            is_dir: false,
        }];
        let crashed = guard.rename_batch(&undo, &options(crash_after(1)));
        assert!(crashed.is_err());
    }
    let journal = fs::read_to_string(fixture.journal_path()).expect("journal");
    assert!(journal.contains(r"\\xFF.pak-x"), "{journal}");

    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");

    assert_eq!(guard.recovery().rolled_back, 1);
    assert!(to.exists() && !from.exists());
}

#[test]
fn a_symlinked_mod_folder_is_a_link_component() {
    let fixture = support::fs_fixture();
    support::write_tree(&fixture.root, &[("elsewhere/Linked/a.pak", b"a")]);
    symlink(
        fixture.root.join("elsewhere/Linked"),
        fixture.path("Mods/Linked"),
    )
    .expect("link");

    let result = fixture.scope().check(&fixture.path("Mods/Linked/a.pak"));

    assert_eq!(violation(result), Some(ScopeViolation::LinkComponent));
}

#[test]
fn a_symlinked_root_is_in_scope() {
    let (_dir, root) = support::temp_root();
    let paks = root.join("Paks");
    support::write_tree(&root, &[("Paks/", b""), ("real/Cool/Cool.pak", b"c")]);
    symlink(root.join("real"), paks.join("Mods")).expect("link");
    let scope = Scope::for_paks(&paks, &[]).expect("scope");

    assert!(scope.check(&paks.join("Mods/Cool/Cool.pak")).is_ok());
}

#[test]
fn links_into_another_root_or_the_data_dir_are_link_components() {
    let fixture = support::fs_fixture();
    symlink("../LogicMods", fixture.path("Mods/Evil")).expect("link");
    symlink(&fixture.app.data_dir, fixture.path("Mods/Data")).expect("link");
    let scope = fixture.scope();

    assert_eq!(
        violation(scope.check(&fixture.path("Mods/Evil/x.pak"))),
        Some(ScopeViolation::LinkComponent)
    );
    assert_eq!(
        violation(scope.check(&fixture.path("Mods/Data/journal.json"))),
        Some(ScopeViolation::LinkComponent)
    );
}

#[test]
fn renaming_a_symlinked_pak_renames_the_link_only() {
    let fixture = support::fs_fixture();
    support::write_tree(
        &fixture.root,
        &[("store/a.pak", b"a"), ("Paks/Mods/L/", b"")],
    );
    let target = fixture.root.join("store/a.pak");
    symlink(&target, fixture.path("Mods/L/a.pak")).expect("link");
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");

    guard
        .rename_batch(
            &[fixture.op("Mods/L/a.pak", "Mods/L/a.pak-x", false)],
            &options(None),
        )
        .expect("batch");

    let renamed = fixture.path("Mods/L/a.pak-x");
    assert!(
        fs::symlink_metadata(&renamed)
            .expect("meta")
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read_link(&renamed).expect("read link"), target);
    assert_eq!(fs::read(&target).expect("read"), b"a");
}
