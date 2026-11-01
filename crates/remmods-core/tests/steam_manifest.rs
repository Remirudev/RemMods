#![allow(clippy::expect_used)]

mod game_support;

use game_support::{steam_fixture, steam_text};
use remmods_core::game::{ManifestError, parse_app_manifest};

fn with_key(key: &str, value: &str) -> String {
    let text = steam_text("appmanifest_1778820.acf");
    let line = text
        .lines()
        .find(|line| line.starts_with(&format!("\t\"{key}\"")))
        .expect("key in fixture");
    text.replace(line, &format!("\t\"{key}\"\t\t\"{value}\""))
}

#[test]
fn real_manifest_parses() {
    let manifest =
        parse_app_manifest(&steam_fixture("appmanifest_1778820.acf")).expect("manifest parses");
    assert_eq!(manifest.app_id, 1_778_820);
    assert_eq!(manifest.install_dir, "TEKKEN 8");
    assert_eq!(manifest.build_id, "25221600");
    assert_eq!(manifest.state_flags, 4);
    assert_eq!(manifest.target_build_id.as_deref(), Some("0"));
    assert_eq!(manifest.last_updated, Some(1_789_022_667));
    assert_eq!(manifest.name.as_deref(), Some("TEKKEN 8"));
    assert!(manifest.fully_installed());
    assert!(!manifest.update_pending());
}

#[test]
fn update_pending_variant() {
    let manifest = parse_app_manifest(&steam_fixture("appmanifest_1778820.update-pending.acf"))
        .expect("manifest parses");
    assert_eq!(manifest.build_id, "25221600");
    assert_eq!(manifest.state_flags & 2, 2);
    assert_eq!(manifest.last_updated, Some(1_789_022_667));
    assert_eq!(manifest.target_build_id.as_deref(), Some("26410000"));
    assert!(manifest.update_pending());
}

#[test]
fn unsafe_install_dirs_are_rejected() {
    for value in ["", "..", "../x", "a/b", r"a\\b", "C:x"] {
        let text = with_key("installdir", value);
        let error = parse_app_manifest(text.as_bytes()).expect_err("unsafe installdir");
        assert!(
            matches!(error, ManifestError::UnsafeInstallDir),
            "{value:?} gave {error:?}"
        );
        assert_eq!(error.code(), "unsafeInstallDir");
    }
}

#[test]
fn wrong_app_id_is_rejected() {
    let text = with_key("appid", "1778821");
    let error = parse_app_manifest(text.as_bytes()).expect_err("wrong app id");
    assert!(matches!(error, ManifestError::WrongAppId));
    assert_eq!(error.code(), "wrongAppId");
}

#[test]
fn non_digit_build_id_is_rejected() {
    let text = with_key("buildid", "25x");
    let error = parse_app_manifest(text.as_bytes()).expect_err("bad build id");
    assert!(matches!(error, ManifestError::BadBuildId));
    assert_eq!(error.code(), "badBuildId");
}

#[test]
fn other_error_codes() {
    let missing = steam_text("appmanifest_1778820.acf").replace("\"buildid\"", "\"build\"");
    let error = parse_app_manifest(missing.as_bytes()).expect_err("missing key");
    assert!(matches!(error, ManifestError::MissingKey("buildid")));
    assert_eq!(error.code(), "missingKey");
    let bad_flags = with_key("StateFlags", "four");
    let error = parse_app_manifest(bad_flags.as_bytes()).expect_err("bad number");
    assert_eq!(error.code(), "badNumber");
    let library = steam_fixture("libraryfolders.old.vdf");
    let error = parse_app_manifest(&library).expect_err("wrong root");
    assert_eq!((error.code(), error.line()), ("wrongRoot", None));
    let error =
        parse_app_manifest(&steam_fixture("libraryfolders.broken.vdf")).expect_err("broken");
    assert_eq!(
        (error.code(), error.line()),
        ("unterminatedString", Some(14))
    );
}
