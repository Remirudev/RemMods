#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod support;

use std::ffi::OsStr;

use proptest::prelude::*;
use remmods_core::safe_fs::names::{is_install_temp_name, utf16_len};
use remmods_core::safe_fs::{FsError, ScopeViolation, set_aside_name};

use support::{FsFixture, crash_after, options, tree_listing};

const HEX: &str = "0123456789abcdef0123456789abcdef";

fn aside_rel(label: &str) -> String {
    format!("Mods/_{label}_tmp_{HEX}")
}

/// Runs `remove_aside` on `rel` and returns the error; the Paks tree must be unchanged.
fn refused(fixture: &FsFixture, path: &std::path::Path) -> FsError {
    let before = tree_listing(&fixture.paks);
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");
    let error = guard.remove_aside(path).expect_err("refused");
    assert_eq!(tree_listing(&fixture.paks), before, "unchanged");
    error
}

#[test]
fn remove_aside_refuses_non_temp_name() {
    let fixture = support::fs_fixture();
    support::write_tree(&fixture.paks, &[("Mods/Cool/readme.txt", b"r")]);
    let error = refused(&fixture, &fixture.path("Mods/Cool"));
    assert!(matches!(error, FsError::NotAnAside { .. }), "{error}");
}

#[test]
fn remove_aside_refuses_live_payload_at_depth_3() {
    let fixture = support::fs_fixture();
    let rel = aside_rel("x");
    support::write_tree(&fixture.paks, &[(&format!("{rel}/a/b/c.pak"), b"pak")]);
    let error = refused(&fixture, &fixture.path(&rel));
    assert!(matches!(error, FsError::AsideNotInert { .. }), "{error}");
}

#[test]
fn remove_aside_refuses_paks_level_and_nested() {
    let fixture = support::fs_fixture();
    let name = format!("_x_tmp_{HEX}");
    let nested = format!("Mods/Cool/{name}");
    support::write_tree(
        &fixture.paks,
        &[
            (&format!("{name}/a.txt"), b"a"),
            (&format!("{nested}/b.txt"), b"b"),
        ],
    );
    let paks_level = refused(&fixture, &fixture.paks.join(&name));
    assert!(
        matches!(
            paks_level,
            FsError::OutOfScope { .. } | FsError::NotAnAside { .. }
        ),
        "{paks_level}"
    );
    let inside_mod = refused(&fixture, &fixture.path(&nested));
    assert!(
        matches!(inside_mod, FsError::NotAnAside { .. }),
        "{inside_mod}"
    );
}

#[test]
fn remove_aside_refuses_outside_scope() {
    let fixture = support::fs_fixture();
    let name = format!("_x_tmp_{HEX}");
    support::write_tree(&fixture.root, &[(&format!("{name}/a.txt"), b"a")]);
    let before = tree_listing(&fixture.root.join(&name));
    let error = refused(&fixture, &fixture.root.join(&name));
    assert!(
        matches!(
            error,
            FsError::OutOfScope {
                reason: ScopeViolation::OutsideRoots,
                ..
            }
        ),
        "{error}"
    );
    assert_eq!(tree_listing(&fixture.root.join(&name)), before);
}

#[test]
fn remove_aside_refuses_uppercase_live_ext() {
    let fixture = support::fs_fixture();
    let rel = aside_rel("x");
    support::write_tree(
        &fixture.paks,
        &[
            (&format!("{rel}/X.PAK"), b"pak"),
            (&format!("{rel}/a.txt"), b"a"),
        ],
    );
    let error = refused(&fixture, &fixture.path(&rel));
    assert!(matches!(error, FsError::AsideNotInert { .. }), "{error}");
}

#[test]
fn remove_aside_removes_inert_folder() {
    let fixture = support::fs_fixture();
    let rel = aside_rel("Cool");
    support::write_tree(
        &fixture.paks,
        &[
            (&format!("{rel}/Cool.pak-x"), b"pak"),
            (&format!("{rel}/Cool.ucas-x"), b"ucas"),
            (&format!("{rel}/sub/notes.txt"), b"n"),
            ("Mods/Other/Other.pak", b"other"),
            ("~mods/Keep.pak", b"keep"),
        ],
    );
    let siblings: Vec<String> = tree_listing(&fixture.paks)
        .into_iter()
        .filter(|line| !line.contains("_Cool_tmp_"))
        .collect();
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");

    guard.remove_aside(&fixture.path(&rel)).expect("removed");

    assert!(!fixture.path(&rel).exists());
    assert_eq!(tree_listing(&fixture.paks), siblings);
}

#[test]
fn remove_aside_refuses_journaled_folder() {
    let fixture = support::fs_fixture();
    support::write_tree(&fixture.paks, &[("Mods/Cool/Cool.pak-x", b"pak")]);
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");
    let aside = aside_rel("Cool");
    let ops = [fixture.op("Mods/Cool", &aside, true)];
    guard
        .rename_batch(&ops, &options(crash_after(1)))
        .expect_err("crash leaves the journal entry");
    let before = tree_listing(&fixture.paks);

    let error = guard
        .remove_aside(&fixture.path(&aside))
        .expect_err("journaled");

    assert!(matches!(error, FsError::AsideJournaled { .. }), "{error}");
    assert_eq!(tree_listing(&fixture.paks), before);
    assert!(fixture.path(&aside).join("Cool.pak-x").is_file());
}

fn assert_bounded_temp_name(label: &str) {
    let name = set_aside_name(OsStr::new(label), HEX);
    let text = name.to_str().expect("UTF-8 name");
    assert!(is_install_temp_name(&name), "{text}");
    assert!(text.len() <= 255, "{} bytes", text.len());
    assert!(utf16_len(text) <= 255, "{} units", utf16_len(text));
    assert!(text.ends_with(&format!("_tmp_{HEX}")));
}

#[test]
fn set_aside_name_is_temp_and_bounded_examples() {
    assert_bounded_temp_name(&"a".repeat(300));
    assert_bounded_temp_name(&"日本語".repeat(100));
    assert_bounded_temp_name(".disabled_Logic");
    assert_eq!(
        set_aside_name(OsStr::new("Cool"), HEX),
        OsStr::new(&format!("_Cool_tmp_{HEX}"))
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn set_aside_name_is_temp_and_bounded(label in "\\PC{0,300}") {
        assert_bounded_temp_name(&label);
    }
}
