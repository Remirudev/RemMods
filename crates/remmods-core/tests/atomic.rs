#![allow(clippy::expect_used)]

mod support;

use std::fs;
use std::path::Path;

use remmods_core::safe_fs::{backup_file, write_atomic};

fn file_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("list folder")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

#[test]
fn creates_missing_parent_folders() {
    let (_dir, root) = support::temp_root();
    let path = root.join("a").join("b").join("settings.json");

    write_atomic(&path, b"{}\n").expect("write");

    assert_eq!(fs::read(&path).expect("read"), b"{}\n");
}

#[test]
fn replaces_existing_content_completely() {
    let (_dir, root) = support::temp_root();
    let path = root.join("settings.json");
    write_atomic(&path, b"a much longer first version\n").expect("first write");

    write_atomic(&path, b"short\n").expect("second write");

    assert_eq!(fs::read(&path).expect("read"), b"short\n");
}

#[test]
fn leaves_no_temp_files_behind() {
    let (_dir, root) = support::temp_root();
    let path = root.join("settings.json");

    write_atomic(&path, b"one\n").expect("first write");
    write_atomic(&path, b"two\n").expect("second write");

    assert_eq!(file_names(&root), ["settings.json"]);
}

#[test]
fn failed_replace_removes_its_temp_file() {
    let (_dir, root) = support::temp_root();
    let path = root.join("settings.json");
    write_atomic(&path.join("blocker.txt"), b"x\n").expect("make the target a folder");

    let result = write_atomic(&path, b"new\n");

    assert!(result.is_err(), "a file cannot replace a non-empty folder");
    assert_eq!(file_names(&root), ["settings.json"]);
    assert_eq!(file_names(&path), ["blocker.txt"]);
}

#[test]
fn backups_never_overwrite_each_other() {
    let (_dir, root) = support::temp_root();
    let path = root.join("settings.json");
    write_atomic(&path, b"original\n").expect("write");

    let first = backup_file(&path).expect("first backup");
    let second = backup_file(&path).expect("second backup");

    assert_ne!(first, second);
    assert_eq!(fs::read(&first).expect("read first"), b"original\n");
    assert_eq!(fs::read(&second).expect("read second"), b"original\n");
    assert_eq!(fs::read(&path).expect("read original"), b"original\n");
}
