//! Linux-only folder-source cases: links, FIFOs, raw-byte names and files swapped before extract
//! (P4 §8.1 L-tests).
#![cfg(target_os = "linux")]
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod archive_support;

use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::symlink;
use std::path::Path;

use archive_support::{RecordingTarget, extract_all, full_plan, temp_dir};
use remmods_core::archive::{self, ArchiveError, EntryKind, Limits, RawEntryName};
use rustix::fs::{CWD, FileType, Mode, mknodat};

fn fifo(path: &Path) {
    mknodat(CWD, path, FileType::Fifo, Mode::from_raw_mode(0o644), 0).expect("mkfifo");
}

fn kind_of(listing: &archive::Listing, name: &str) -> EntryKind {
    listing
        .entries
        .iter()
        .find(|entry| entry.name == RawEntryName::Unicode(name.to_owned()))
        .map(|entry| entry.kind)
        .expect("listed")
}

#[test]
fn l1_links_and_fifos_are_listed_not_followed() {
    let (_dir, root) = temp_dir();
    let outside = root.join("outside");
    fs::create_dir(&outside).expect("outside");
    fs::write(outside.join("secret.pak"), b"secret").expect("secret");
    let source = root.join("Cool");
    fs::create_dir(&source).expect("source");
    fs::write(source.join("x.pak"), b"pak").expect("pak");
    symlink(&outside, source.join("linked")).expect("dir link");
    symlink(outside.join("secret.pak"), source.join("file-link.pak")).expect("file link");
    fifo(&source.join("pipe.pak"));
    let reader = archive::open_folder(&source, &Limits::default()).expect("open");
    let listing = reader.listing();
    assert_eq!(kind_of(listing, "linked"), EntryKind::Link);
    assert_eq!(kind_of(listing, "file-link.pak"), EntryKind::Link);
    assert_eq!(kind_of(listing, "pipe.pak"), EntryKind::Special);
    assert_eq!(kind_of(listing, "x.pak"), EntryKind::File);
    assert_eq!(listing.entries.len(), 4, "the linked folder is not walked");

    let dir_link = root.join("link-to-source");
    symlink(&source, &dir_link).expect("source link");
    let result = archive::open_folder(&dir_link, &Limits::default());
    assert!(matches!(result, Err(ArchiveError::NotAnArchive)));
}

#[test]
fn l2_non_utf8_names_stay_bytes() {
    let (_dir, root) = temp_dir();
    let source = root.join("Cool");
    fs::create_dir(&source).expect("source");
    fs::write(source.join(OsStr::from_bytes(b"bad\xFF.pak")), b"x").expect("raw name");
    let reader = archive::open_folder(&source, &Limits::default()).expect("open");
    assert_eq!(
        reader.listing().entries[0].name,
        RawEntryName::Bytes(b"bad\xFF.pak".to_vec())
    );
}

#[test]
fn l3_swapped_files_are_refused() {
    let (_dir, root) = temp_dir();
    let outside = root.join("outside.pak");
    fs::write(&outside, b"OUTSIDE").expect("outside");
    let source = root.join("Cool");
    fs::create_dir(&source).expect("source");
    fs::write(source.join("a.pak"), b"aaaaaaa").expect("a");
    fs::write(source.join("b.pak"), b"bbbbbbb").expect("b");
    let mut reader = archive::open_folder(&source, &Limits::default()).expect("open");
    let plan = full_plan(reader.as_ref());

    fs::remove_file(source.join("a.pak")).expect("remove a");
    symlink(&outside, source.join("a.pak")).expect("swap for a link");
    let mut target = RecordingTarget::new();
    let result = extract_all(reader.as_mut(), &plan, &mut target);
    assert!(
        matches!(result, Err(ArchiveError::SizeMismatch { .. })),
        "{result:?}"
    );
    assert!(
        target
            .bytes
            .values()
            .all(|data| data.as_slice() != b"OUTSIDE")
    );
    assert!(target.created.is_empty());

    fs::remove_file(source.join("a.pak")).expect("remove link");
    fifo(&source.join("a.pak"));
    let mut target = RecordingTarget::new();
    let result = extract_all(reader.as_mut(), &plan, &mut target);
    assert!(
        matches!(result, Err(ArchiveError::SizeMismatch { .. })),
        "{result:?}"
    );
    assert!(target.created.is_empty());
}
