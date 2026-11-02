//! Windows-only folder and path cases: junctions and verbatim-only folder names (P4 §8.1
//! W-tests).
#![cfg(windows)]
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod archive_support;

use std::fs;
use std::path::Path;
use std::process::Command;

use archive_support::{CraftEntry, ZipCraft, temp_dir};
use remmods_core::archive::{self, EntryKind, Limits, RawEntryName};
use remmods_core::platform;

fn junction(link: &Path, target: &Path) {
    let status = Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .status()
        .expect("run mklink");
    assert!(status.success(), "mklink /J failed");
}

#[test]
fn w1_junction_is_a_link() {
    let (_dir, root) = temp_dir();
    let outside = root.join("outside");
    fs::create_dir(&outside).expect("outside");
    fs::write(outside.join("secret.pak"), b"secret").expect("secret");
    let source = root.join("Cool");
    fs::create_dir(&source).expect("source");
    fs::write(source.join("x.pak"), b"pak").expect("pak");
    junction(&source.join("linked"), &outside);
    let reader = archive::open_folder(&source, &Limits::default()).expect("open");
    let linked = reader
        .listing()
        .entries
        .iter()
        .find(|entry| entry.name == RawEntryName::Unicode("linked".to_owned()))
        .expect("junction listed");
    assert_eq!(linked.kind, EntryKind::Link);
    assert_eq!(
        reader.listing().entries.len(),
        2,
        "the junction is not walked"
    );
}

#[test]
fn w2_archive_in_a_verbatim_only_folder_opens() {
    let (_dir, root) = temp_dir();
    let folder = root.join("trailing.");
    fs::create_dir(platform::fs_path(&folder).expect("fs path")).expect("create trailing.");
    let bytes = ZipCraft::new()
        .with(CraftEntry::file("x.pak", b"x"))
        .bytes();
    let archive_path = folder.join("x.zip");
    fs::write(platform::fs_path(&archive_path).expect("fs path"), bytes).expect("write zip");
    let reader = archive::open(&archive_path, &Limits::default()).expect("open through fs_path");
    assert_eq!(reader.listing().entries.len(), 1);
    assert!(archive::fingerprint(&archive_path).is_ok());
}
