//! Linux-only install cases (P4 spec §8.2 L-I1–L-I3): symlinks in staging and as sources,
//! case-variant roots, archive modes never reaching the disk.
#![cfg(target_os = "linux")]
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod archive_support;
mod install_support;
mod mods_support;
mod support;

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};

use archive_support::{CraftEntry, ZipCraft};
use install_support::{fixture, has_issue};
use remmods_core::install::{IssueCode, IssueSeverity, SourceReject, classify_source};
use remmods_core::safe_fs::Scope;

#[test]
fn l_i1_symlinks_are_refused() {
    let fixture = fixture(&[("Mods/Other/o.pak", b"o")]);
    let app_dirs = [fixture.fx.paths.data_dir.as_path()];
    let scope = Scope::for_paks(&fixture.fx.paks, &app_dirs).expect("scope");
    let guard = fixture.fx.core.ops.write(scope).expect("guard");
    let staging = guard
        .create_staging(&fixture.fx.paks.join("Mods"), mods_support::manage::HEX)
        .expect("staging");
    let elsewhere = fixture.src.join("elsewhere");
    fs::create_dir(&elsewhere).expect("dir");
    symlink(&elsewhere, staging.path().join("sub")).expect("plant link");
    let refused = staging.create_file(&["sub".to_owned(), "x.pak-x".to_owned()]);
    assert!(refused.is_err());
    assert!(
        support::names_in(&elsewhere).is_empty(),
        "nothing written through the link"
    );
    fs::remove_file(staging.path().join("sub")).expect("remove link");
    guard.discard_staging(staging).expect("discard");

    symlink(&elsewhere, fixture.src.join("link")).expect("source link");
    assert_eq!(
        classify_source(&fixture.src.join("link"), Some(&fixture.fx.paks)),
        Err(SourceReject::Link)
    );
}

#[test]
fn l_i2_case_variant_roots_make_the_target_ambiguous() {
    let fixture = fixture(&[("Mods/Cool/a.pak", b"a"), ("mods/Cool/b.pak", b"b")]);
    let zip = fixture.zip("Cool.zip", &[("Cool/x.pak", b"p")]);
    let inspection = fixture.inspect(&zip);
    assert!(
        has_issue(&inspection, IssueSeverity::Block, IssueCode::Ambiguous),
        "{inspection:?}"
    );
    assert!(fixture.start(&inspection).is_err());
}

#[test]
fn l_i3_archive_modes_are_ignored() {
    let fixture = fixture(&[]);
    let zip = ZipCraft::new()
        .with(CraftEntry {
            unix_mode: Some(0o104_755),
            ..CraftEntry::file("Cool/x.pak", b"setuid?")
        })
        .write(&fixture.src, "Mode.zip");
    let inspection = fixture.inspect(&zip);
    fixture.start(&inspection).expect("install");
    let path = fixture.fx.paks.join("Mods").join("Cool").join("x.pak");
    let mode = fs::metadata(path).expect("metadata").permissions().mode();
    assert_eq!(mode & 0o7111, 0, "mode {mode:o}");
}
