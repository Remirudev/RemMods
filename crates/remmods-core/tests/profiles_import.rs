//! Import of the original's `profiles.json` and RemMods export/import (spec §5.9, §8.2 T-1..T-13).
#![allow(clippy::expect_used, clippy::disallowed_methods)]

mod mods_support;
mod profiles_support;
mod support;

use std::path::{Path, PathBuf};

use mods_support::DATA;
use mods_support::manage::{Extras, ToggleFixture, core_with_trash};
use profiles_support::{apply, by_name, save};
use remmods_core::CoreError;
use remmods_core::profiles::{
    IMPORT_MAX_BYTES, ImportProblem, ProfileError, ProfileMode, ProfileSave, ProfileSource,
    parse_tmm_profiles,
};
use remmods_core::safe_fs::FsError;

fn fixture() -> ToggleFixture {
    core_with_trash(&[("Mods/A/A.pak", DATA), ("Mods/B/B.pak-x", DATA)])
}

fn write(path: &Path, bytes: &[u8]) -> PathBuf {
    std::fs::write(path, bytes).expect("write source");
    path.to_path_buf()
}

fn unparsable<T: std::fmt::Debug>(result: Result<T, CoreError>) -> ImportProblem {
    match result {
        Err(CoreError::Profiles(ProfileError::Unparsable(problem))) => problem,
        other => panic!("expected Unparsable, got {other:?}"),
    }
}

const TMM: &[u8] = br#"[
  {"Id":"1","Name":"Fights","Description":"d","EnabledMods":["A","Logic:X"],"Created":"2025-01-01T00:00:00","LastUsed":null,"IsDefault":false},
  {"Id":"2","Name":"Default","Description":null,"EnabledMods":[],"IsDefault":true}
]"#;

#[test]
fn tmm_import_never_writes_source() {
    let fixture = fixture();
    let source = write(&fixture.root.join("profiles.json"), TMM);
    let modified = std::fs::metadata(&source)
        .expect("meta")
        .modified()
        .expect("mtime");
    fixture.core.profiles_import_tmm(&source).expect("import");
    assert_eq!(std::fs::read(&source).expect("read"), TMM);
    let after = std::fs::metadata(&source)
        .expect("meta")
        .modified()
        .expect("mtime");
    assert_eq!(after, modified);
}

#[test]
fn tmm_null_enabled_mods_and_missing_fields() {
    let bytes =
        br#"[{"Id":"a","Name":null,"EnabledMods":null},{"Id":"b"},{"Name":"x"},{"Id":"  "},5]"#;
    let (profiles, skipped) = parse_tmm_profiles(bytes).expect("parse");
    assert_eq!(skipped, 3);
    assert_eq!(profiles.len(), 2);
    assert!(profiles.iter().all(|profile| profile.name.is_empty()
        && profile.description.is_empty()
        && profile.enabled_mods.is_empty()
        && !profile.is_default));
}

#[test]
fn tmm_keys_without_colon_become_standard() {
    let bytes = br#"[{"Id":"a","EnabledMods":["Cool","Logic:X","",3,"Cool","Standard:Cool"]}]"#;
    let (profiles, _) = parse_tmm_profiles(bytes).expect("parse");
    assert_eq!(profiles[0].enabled_mods, ["Standard:Cool", "Logic:X"]);
}

#[test]
fn tmm_import_is_idempotent_by_id() {
    let fixture = fixture();
    let source = write(&fixture.root.join("profiles.json"), TMM);
    let first = fixture.core.profiles_import_tmm(&source).expect("import");
    assert_eq!((first.added, first.updated, first.unchanged), (2, 0, 0));
    let fights = by_name(&first.view, "Fights").clone();
    assert_eq!(fights.mode, ProfileMode::Exclusive);
    assert_eq!(
        fights.source,
        ProfileSource::Tmm {
            id: "1".into(),
            is_default: false
        }
    );
    let second = fixture
        .core
        .profiles_import_tmm(&source)
        .expect("import again");
    assert_eq!((second.added, second.updated, second.unchanged), (0, 0, 2));
    let changed = String::from_utf8_lossy(TMM).replace("Fights", "Brawls");
    write(&source, changed.as_bytes());
    let third = fixture
        .core
        .profiles_import_tmm(&source)
        .expect("import changed");
    assert_eq!((third.added, third.updated, third.unchanged), (0, 1, 1));
    assert_eq!(by_name(&third.view, "Brawls").id, fights.id);
}

#[test]
fn tmm_bom_utf16_file() {
    let mut bytes = vec![0xFF, 0xFE];
    for unit in String::from_utf8_lossy(TMM).encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    let (profiles, skipped) = parse_tmm_profiles(&bytes).expect("parse");
    assert_eq!((profiles.len(), skipped), (2, 0));
    assert_eq!(profiles[0].enabled_mods, ["Standard:A", "Logic:X"]);
}

#[test]
fn tmm_wrong_shape_and_too_large() {
    let fixture = fixture();
    let shape = write(&fixture.root.join("shape.json"), br#"{"Id":"1"}"#);
    assert_eq!(
        unparsable(fixture.core.profiles_import_tmm(&shape)),
        ImportProblem::WrongShape
    );
    let text = write(&fixture.root.join("text.json"), b"not json");
    assert_eq!(
        unparsable(fixture.core.profiles_import_tmm(&text)),
        ImportProblem::NotJson
    );
    let size = usize::try_from(IMPORT_MAX_BYTES).expect("fits") + 1;
    let large = write(&fixture.root.join("large.json"), &vec![b' '; size]);
    assert_eq!(
        unparsable(fixture.core.profiles_import_tmm(&large)),
        ImportProblem::TooLarge
    );
    assert!(!fixture.paths.profiles_file().exists(), "nothing written");
}

#[test]
fn tmm_name_collision_suffix() {
    let fixture = fixture();
    save(&fixture, "default", ProfileMode::Additive);
    let source = write(&fixture.root.join("profiles.json"), TMM);
    let report = fixture.core.profiles_import_tmm(&source).expect("import");
    by_name(&report.view, "Default (2)");
}

fn export(fixture: &ToggleFixture, name: &str) -> PathBuf {
    let file = fixture.root.join(name);
    let report = fixture.core.profiles_export(&file).expect("export");
    assert_eq!(report.file_name, name);
    file
}

#[test]
fn export_import_round_trip() {
    let fixture = fixture();
    save(&fixture, "One", ProfileMode::Additive);
    fixture
        .core
        .profiles_save(&ProfileSave::New {
            name: "Two".into(),
            description: "second".into(),
            mode: ProfileMode::Exclusive,
        })
        .expect("save");
    let file = export(&fixture, "out.json");
    let text = std::fs::read_to_string(&file).expect("read export");
    assert!(text.starts_with("{\n  \"format\": \"remmods-profiles\",\n  \"version\": 1,\n"));
    assert!(text.ends_with("}\n"));
    let other = core_with_trash(&[]);
    let report = other.core.profiles_import(&file).expect("import");
    assert_eq!((report.added, report.skipped), (2, 0));
    let original = fixture.core.profiles_list().expect("list");
    for summary in &original.profiles {
        let source = &summary.profile;
        let copy = by_name(&report.view, &source.name);
        assert_eq!(copy.id, source.id);
        assert_eq!(
            (&copy.description, copy.mode, &copy.mods),
            (&source.description, source.mode, &source.mods)
        );
        assert_eq!(copy.source, ProfileSource::Import);
    }
}

#[test]
fn import_same_id_different_content_adds_copy() {
    let fixture = fixture();
    let id = save(&fixture, "One", ProfileMode::Additive);
    let file = export(&fixture, "out.json");
    let again = fixture
        .core
        .profiles_import(&file)
        .expect("import unchanged");
    assert_eq!((again.added, again.unchanged), (0, 1));
    fixture
        .core
        .profiles_save(&ProfileSave::Edit {
            id: id.clone(),
            name: "One".into(),
            description: "edited".into(),
            mode: ProfileMode::Additive,
        })
        .expect("edit");
    let report = fixture.core.profiles_import(&file).expect("import");
    assert_eq!(report.added, 1);
    let copy = by_name(&report.view, "One (2)");
    assert_ne!(copy.id, id);
    assert_eq!(by_name(&report.view, "One").id, id);
}

#[test]
fn import_wrong_format_and_newer_version() {
    let fixture = fixture();
    let wrong = write(
        &fixture.root.join("wrong.json"),
        br#"{"format":"other","version":1,"profiles":[]}"#,
    );
    assert_eq!(
        unparsable(fixture.core.profiles_import(&wrong)),
        ImportProblem::WrongFormat
    );
    let newer = write(
        &fixture.root.join("newer.json"),
        br#"{"format":"remmods-profiles","version":2,"profiles":[]}"#,
    );
    assert_eq!(
        unparsable(fixture.core.profiles_import(&newer)),
        ImportProblem::NewerVersion
    );
    assert!(!fixture.paths.profiles_file().exists());
}

#[test]
fn export_excludes_snapshots() {
    let fixture = fixture();
    let id = profiles_support::custom(
        &fixture,
        "Flip",
        ProfileMode::Additive,
        &[("Standard:A", remmods_core::profiles::EntryState::Disabled)],
    );
    let outcome = apply(&fixture, &id).expect("apply");
    assert!(outcome.snapshot_id.is_some());
    let file = fixture.root.join("out.json");
    let report = fixture.core.profiles_export(&file).expect("export");
    assert_eq!(report.count, 1);
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&file).expect("read")).expect("json");
    assert_eq!(value["profiles"].as_array().expect("profiles").len(), 1);
}

fn assert_out_of_scope<T: std::fmt::Debug>(result: Result<T, CoreError>) {
    assert!(
        matches!(result, Err(CoreError::Fs(FsError::OutOfScope { .. }))),
        "{result:?}"
    );
}

#[test]
fn export_refused_inside_paks() {
    let fixture = fixture();
    save(&fixture, "One", ProfileMode::Additive);
    let before = support::tree_listing(&fixture.paks);
    assert_out_of_scope(fixture.core.profiles_export(&fixture.paks.join("out.json")));
    assert_out_of_scope(
        fixture
            .core
            .profiles_export(&fixture.paks.join("Mods").join("out.json")),
    );
    assert_eq!(support::tree_listing(&fixture.paks), before);
}

#[test]
fn export_refused_into_tmm_dir() {
    let fixture = ToggleFixture::build(&[("Mods/A/A.pak", DATA)], |root| {
        let tmm = root.join("TekkenModManager");
        std::fs::create_dir_all(&tmm).expect("tmm dir");
        Extras {
            tmm_dir: Some(tmm),
            ..Extras::default()
        }
    });
    save(&fixture, "One", ProfileMode::Additive);
    let tmm = fixture.root.join("TekkenModManager");
    assert_out_of_scope(fixture.core.profiles_export(&tmm.join("backup.json")));
    assert!(support::names_in(&tmm).is_empty(), "nothing written");
}
