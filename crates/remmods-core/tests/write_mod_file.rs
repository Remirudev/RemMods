#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod support;

use std::fs;

use remmods_core::safe_fs::FsError;

use support::{FsFixture, tree_listing};

fn write(fixture: &FsFixture, rel: &str, bytes: &[u8]) -> Result<(), FsError> {
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");
    guard.write_mod_file(&fixture.path(rel), bytes)
}

#[test]
fn write_mod_file_refuses_root_level_and_paks() {
    let fixture = support::fs_fixture();
    support::write_tree(&fixture.paks, &[("Mods/Cool/modinfo.json", b"{}")]);
    let before = tree_listing(&fixture.paks);

    for rel in [
        "Mods/modinfo.json",
        "modinfo.json",
        "Mods/Cool/sub/deeper/modinfo.json",
    ] {
        let result = write(&fixture, rel, b"new");
        assert!(
            matches!(result, Err(FsError::OutOfScope { .. })),
            "{rel}: {result:?}"
        );
    }
    assert_eq!(tree_listing(&fixture.paks), before);
}

#[test]
fn write_mod_file_reaches_a_mod_in_an_author_folder() {
    let fixture = support::fs_fixture();
    support::write_tree(&fixture.paks, &[("Mods/Me/Cool/Cool.pak", b"p")]);
    write(&fixture, "Mods/Me/Cool/modinfo.json", b"new").expect("written");
    assert_eq!(
        fs::read(fixture.path("Mods/Me/Cool/modinfo.json")).expect("read"),
        b"new"
    );
}

#[test]
fn write_mod_file_refuses_linked_mod_folder() {
    let fixture = support::fs_fixture();
    support::write_tree(&fixture.paks, &[("Mods/Cool/", b"")]);
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");
    let before = tree_listing(&fixture.root);

    let outside = fixture.root.join("modinfo.json");
    let result = guard.write_mod_file(&outside, b"new");

    assert!(
        matches!(result, Err(FsError::OutOfScope { .. })),
        "{result:?}"
    );
    assert_eq!(tree_listing(&fixture.root), before);
}

#[test]
fn write_mod_file_writes_atomically() {
    let fixture = support::fs_fixture();
    support::write_tree(&fixture.paks, &[("Mods/Cool/modinfo.json", b"{\"old\":1}")]);

    write(&fixture, "Mods/Cool/modinfo.json", b"{\"new\":2}").expect("written");

    assert_eq!(
        fs::read(fixture.path("Mods/Cool/modinfo.json")).expect("read"),
        b"{\"new\":2}"
    );
    assert_eq!(
        support::names_in(&fixture.path("Mods/Cool")),
        ["modinfo.json"]
    );
}

#[test]
fn backup_mod_file_names_backup() {
    let fixture = support::fs_fixture();
    support::write_tree(&fixture.paks, &[("LogicMods/X/modinfo.json", b"{\"v\":1}")]);
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");

    let backup = guard
        .backup_mod_file(&fixture.path("LogicMods/X/modinfo.json"))
        .expect("backup");

    let name = backup
        .file_name()
        .expect("name")
        .to_string_lossy()
        .into_owned();
    let stamp = name
        .strip_prefix("modinfo.json.backup.")
        .expect("backup prefix");
    assert_eq!(stamp.len(), 15, "{name}");
    assert!(stamp.chars().enumerate().all(|(index, c)| if index == 8 {
        c == '_'
    } else {
        c.is_ascii_digit()
    }));
    assert_eq!(backup.parent(), Some(fixture.path("LogicMods/X").as_path()));
    assert_eq!(fs::read(&backup).expect("read"), b"{\"v\":1}");

    let refused = guard.backup_mod_file(&fixture.root.join("modinfo.json"));
    assert!(matches!(refused, Err(FsError::OutOfScope { .. })));
}
