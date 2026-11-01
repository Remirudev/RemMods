#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use remmods_core::safe_fs::{FsError, ScopeViolation};

use support::{FsFixture, tree_listing};

fn export(fixture: &FsFixture, path: &Path, forbidden: &[&Path]) -> Result<(), FsError> {
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");
    guard.write_export(path, b"[]", forbidden)
}

fn assert_refused(fixture: &FsFixture, paths: &[PathBuf], forbidden: &[&Path]) {
    for path in paths {
        let before = tree_listing(&fixture.root);
        let result = export(fixture, path, forbidden);
        assert!(
            matches!(result, Err(FsError::OutOfScope { .. })),
            "{}: {result:?}",
            path.display()
        );
        let strip = |listing: Vec<String>| -> Vec<String> {
            listing
                .into_iter()
                .filter(|line| !line.contains("app/data/sessions") && !line.contains(".lock"))
                .collect()
        };
        assert_eq!(strip(tree_listing(&fixture.root)), strip(before));
        assert!(!path.exists(), "{}", path.display());
    }
}

#[test]
fn write_export_refuses_inside_paks_and_app_dirs() {
    let fixture = support::fs_fixture();
    support::write_tree(&fixture.paks, &[("Mods/Cool/Cool.pak", b"pak")]);
    let paks = fixture.paks.clone();
    assert_refused(
        &fixture,
        &[
            fixture.paks.join("export.json"),
            fixture.path("Mods/export.json"),
            fixture.path("Mods/Cool/export.json"),
            fixture.path("~mods/export.json"),
            fixture.app.data_dir.join("export.json"),
            fixture.app.config_dir.join("export.json"),
        ],
        &[&paks],
    );
}

#[test]
fn write_export_refuses_profiles_json_and_forbidden_dir() {
    let fixture = support::fs_fixture();
    let out = fixture.root.join("out");
    let tmm = fixture.root.join("tmm");
    fs::create_dir_all(&out).expect("out");
    fs::create_dir_all(&tmm).expect("tmm");
    let paks = fixture.paks.clone();
    assert_refused(
        &fixture,
        &[
            out.join("Profiles.JSON"),
            out.join("profiles.json"),
            tmm.join("backup.json"),
        ],
        &[&paks, &tmm],
    );
}

#[test]
fn write_export_refuses_non_json_and_relative() {
    let fixture = support::fs_fixture();
    let out = fixture.root.join("out");
    fs::create_dir_all(&out).expect("out");
    assert_refused(
        &fixture,
        &[
            out.join("export.txt"),
            out.join("export"),
            out.join("export.json.bak"),
        ],
        &[],
    );
    let relative = export(&fixture, Path::new("export.json"), &[]);
    assert!(matches!(
        relative,
        Err(FsError::OutOfScope {
            reason: ScopeViolation::NotAbsolute,
            ..
        })
    ));
    let dotted = export(&fixture, &out.join("..").join("x.json"), &[]);
    assert!(matches!(
        dotted,
        Err(FsError::OutOfScope {
            reason: ScopeViolation::DotComponent,
            ..
        })
    ));
}

#[test]
fn write_export_replaces_existing_json() {
    let fixture = support::fs_fixture();
    let out = fixture.root.join("out");
    fs::create_dir_all(&out).expect("out");
    let file = out.join("Remmods-Profiles.JSON");
    fs::write(&file, b"old").expect("old export");
    let paks = fixture.paks.clone();

    export(&fixture, &file, &[&paks]).expect("exported");

    assert_eq!(fs::read(&file).expect("read"), b"[]");
    assert_eq!(support::names_in(&out), ["Remmods-Profiles.JSON"]);
}
