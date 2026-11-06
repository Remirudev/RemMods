//! Inspections through `Core` (P4 spec §8.2 P9–P17): duplicates, case clashes, name fixes,
//! existing mods, per-file rules, path length, encodings, name overrides.
#![allow(clippy::expect_used, clippy::disallowed_methods)]

mod archive_support;
mod install_support;
mod mods_support;
mod support;

use archive_support::{CraftEntry, ZipCraft};
use install_support::{default_request, fixture, has_issue, install_error};
use remmods_core::archive::NameEncoding;
use remmods_core::install::{
    ExistingRelation, InspectOptions, InstallBlock, InstallError, IssueCode, IssueSeverity,
    NameOverride,
};
use remmods_core::mods::RootKind;

use IssueSeverity::{Block, Info, Warn};

fn blocked_by(error: InstallError) -> InstallBlock {
    match error {
        InstallError::Blocked { reason } => reason,
        other => panic!("expected Blocked, got {other:?}"),
    }
}

#[test]
fn p9_duplicates() {
    let fixture = fixture(&[]);
    let same = fixture.zip(
        "Same.zip",
        &[("Cool/x.pak", b"pak"), ("Cool\\x.pak", b"pak")],
    );
    let inspection = fixture.inspect(&same);
    assert!(has_issue(&inspection, Info, IssueCode::DuplicateIdentical));
    assert!(!inspection.blocked);
    assert_eq!(inspection.candidates[0].files, 1);
    let differ = fixture.zip(
        "Differ.zip",
        &[("Cool/x.pak", b"one"), ("Cool\\x.pak", b"two")],
    );
    assert!(has_issue(
        &fixture.inspect(&differ),
        Block,
        IssueCode::DuplicateDiffers
    ));
    let raw = ZipCraft::new()
        .with(CraftEntry::file("Cool/x.pak", b"one"))
        .with(CraftEntry::file("Cool/x.pak", b"two"))
        .write(&fixture.src, "Raw.zip");
    assert!(has_issue(
        &fixture.inspect(&raw),
        Block,
        IssueCode::DuplicateRaw
    ));
    let prefix = fixture.zip(
        "Prefix.zip",
        &[("Cool/x", b"file"), ("Cool/x/y.pak", b"pak")],
    );
    assert!(has_issue(
        &fixture.inspect(&prefix),
        Block,
        IssueCode::DuplicateDiffers
    ));
}

#[test]
fn p10_case_clash_needs_a_pick() {
    let fixture = fixture(&[]);
    let zip = fixture.zip(
        "Clash.zip",
        &[("Cool/X.pak", b"upper"), ("Cool/x.pak", b"lower")],
    );
    let inspection = fixture.inspect(&zip);
    assert_eq!(inspection.clashes.len(), 1);
    assert_eq!(
        inspection.clashes[0].paths,
        vec!["Cool/X.pak", "Cool/x.pak"]
    );
    assert!(has_issue(&inspection, Warn, IssueCode::CaseClash));
    let mut request = default_request(&inspection);
    request.clashes.clear();
    let error = install_error(fixture.start_with(&request));
    assert_eq!(blocked_by(error), InstallBlock::UnresolvedClash);
    request.clashes = vec![remmods_core::install::ClashPick { id: 0, keep: 1 }];
    fixture.start_with(&request).expect("install");
    let folder = fixture.fx.paks.join("Mods").join("Cool");
    assert_eq!(support::names_in(&folder), vec!["x.pak"]);
    assert_eq!(fixture.read("Mods/Cool/x.pak"), b"lower");
}

#[test]
fn p11_names_are_fixed_on_request() {
    let fixture = fixture(&[]);
    let long = format!("Cool/{}.pak", "x".repeat(250));
    let zip = fixture.zip(
        "Names.zip",
        &[
            ("Cool/CON.pak", b"c"),
            ("Cool/a:b.pak", b"p"),
            ("Cool/a:b.ucas", b"u"),
            ("Cool/a:b.utoc", b"t"),
            ("Cool/notes.", b"n"),
            (&long, b"l"),
        ],
    );
    let inspection = fixture.inspect(&zip);
    assert!(has_issue(&inspection, Warn, IssueCode::InvalidName));
    assert!(!inspection.blocked);
    let mut request = default_request(&inspection);
    request.auto_fix = false;
    assert_eq!(
        blocked_by(install_error(fixture.start_with(&request))),
        InstallBlock::InvalidName
    );
    fixture.start(&inspection).expect("install with fixes");
    let names = support::names_in(&fixture.fx.paks.join("Mods").join("Cool"));
    for wanted in ["CON_.pak", "a_b.pak", "a_b.ucas", "a_b.utoc", "notes"] {
        assert!(names.contains(&wanted.to_owned()), "{wanted} in {names:?}");
    }
    assert!(
        names
            .iter()
            .any(|name| name.len() == 253 && name.ends_with(".pak"))
    );

    let colliding = fixture.zip(
        "Collide.zip",
        &[
            ("Two/a:b.txt", b"1"),
            ("Two/a?b.txt", b"2"),
            ("Two/t.pak", b"p"),
        ],
    );
    assert!(has_issue(
        &fixture.inspect(&colliding),
        Block,
        IssueCode::InvalidName
    ));

    let hex = "0123456789abcdef0123456789abcdef";
    let disabled = fixture.zip("Disabled.zip", &[(".disabled_Cool/x.pak", b"p")]);
    let found = fixture.inspect(&disabled);
    assert_eq!(
        found.candidates[0].fixed_name.as_deref(),
        Some("disabled_Cool")
    );
    let temp_name = format!("_A_tmp_{hex}/x.pak");
    let temp = fixture.zip("Temp.zip", &[(&temp_name, b"p")]);
    let found = fixture.inspect(&temp);
    assert_eq!(found.candidates[0].fixed_name, Some(format!("A_tmp_{hex}")));
}

#[test]
fn p12_a_mod_in_an_author_folder_is_not_replaced() {
    let fixture = fixture(&[("Mods/Me/Cool/a.pak", b"a"), ("Mods/Me/Other/b.pak", b"b")]);
    let found = fixture.inspect(&fixture.zip("Cool.zip", &[("Cool/x.pak", b"p")]));
    assert!(found.candidates[0].existing.is_none());
}

#[test]
fn p12_existing_mods() {
    let fixture = fixture(&[
        ("Mods/Cool/a.pak", b"a"),
        ("Mods/cool2/a.pak", b"a"),
        ("LogicMods/.disabled_X/x.pak", b"x"),
        ("~mods/Other/o.pak", b"o"),
    ]);
    let same = fixture.inspect(&fixture.zip("Cool.zip", &[("Cool/x.pak", b"p")]));
    let existing = same.candidates[0].existing.as_ref().expect("match");
    assert_eq!(existing.relation, ExistingRelation::SameName);
    assert!(has_issue(&same, Warn, IssueCode::Replaces));
    let mut request = default_request(&same);
    request.picks[0].replace = false;
    assert_eq!(
        blocked_by(install_error(fixture.start_with(&request))),
        InstallBlock::ReplaceNotConfirmed
    );
    let fold = fixture.inspect(&fixture.zip("Cool2.zip", &[("Cool2/x.pak", b"p")]));
    let existing = fold.candidates[0].existing.as_ref().expect("match");
    assert_eq!(existing.relation, ExistingRelation::FoldEqual);
    let logic = fixture.inspect(&fixture.zip("X.zip", &[("LogicMods/X/x.pak", b"p")]));
    let existing = logic.candidates[0].existing.as_ref().expect("match");
    assert_eq!(existing.relation, ExistingRelation::DisabledPrefix);
    let also = fixture.inspect(&fixture.zip("Other.zip", &[("Other/o.pak", b"p")]));
    assert_eq!(also.candidates[0].also_in, vec![RootKind::Legacy]);
    assert!(has_issue(&also, Info, IssueCode::AlsoInRoot));
    let fresh = fixture.inspect(&fixture.zip("New.zip", &[("New/n.pak", b"p")]));
    let mut request = default_request(&fresh);
    request.picks[0].replace = true;
    assert_eq!(
        blocked_by(install_error(fixture.start_with(&request))),
        InstallBlock::ReplaceMismatch
    );
}

#[test]
fn p13_p14_shipped_states_and_nested_archives() {
    let fixture = fixture(&[]);
    let both = fixture.zip("Both.zip", &[("Cool/x.pak", b"p"), ("Cool/x.pak-x", b"q")]);
    assert!(has_issue(
        &fixture.inspect(&both),
        Block,
        IssueCode::ShipsBothStates
    ));
    let off = fixture.zip("Off.zip", &[("Cool/x.pak-x", b"q"), ("Cool/y.pak", b"y")]);
    let inspection = fixture.inspect(&off);
    assert!(has_issue(&inspection, Info, IssueCode::ShipsDisabled));
    assert_eq!(inspection.candidates[0].shipped_off, 1);
    let nested = fixture.zip(
        "Nested.zip",
        &[("Cool/inner.zip", b"z"), ("Cool/z.pak", b"p")],
    );
    let inspection = fixture.inspect(&nested);
    assert!(has_issue(&inspection, Info, IssueCode::NestedArchive));
    fixture.start(&inspection).expect("install");
    assert!(!fixture.exists("Mods/Cool/inner.zip"));
    assert!(fixture.exists("Mods/Cool/z.pak"));
}

#[test]
fn p15_path_too_long() {
    let fixture = fixture(&[]);
    let deep = format!("Cool/{}/{}.pak", "d".repeat(200), "f".repeat(100));
    let inspection = fixture.inspect(&fixture.zip("Deep.zip", &[(&deep, b"p")]));
    assert!(has_issue(&inspection, Warn, IssueCode::PathTooLong));
    assert!(inspection.candidates[0].path_too_long);
}

#[test]
fn p16_name_encoding_override() {
    let fixture = fixture(&[]);
    // "テスト" in Shift_JIS: not UTF-8, so Auto reads it as CP437.
    let zip = ZipCraft::new()
        .with(CraftEntry::raw(b"\x83\x65\x83\x58\x83\x67/x.pak", b"p"))
        .write(&fixture.src, "Legacy.zip");
    let auto = fixture.inspect(&zip);
    assert!(auto.legacy_names);
    assert!(has_issue(&auto, Info, IssueCode::LegacyNames));
    let options = InspectOptions {
        encoding: NameEncoding::ShiftJis,
        ..InspectOptions::default()
    };
    let sjis = fixture.inspect_with(&zip, &options);
    assert_eq!(sjis.candidates[0].name, "テスト");
    let again = fixture.inspect_with(&zip, &options);
    let ids = |inspection: &remmods_core::install::Inspection| {
        inspection
            .candidates
            .iter()
            .map(|c| c.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&sjis), ids(&again));
    assert_ne!(sjis.id, again.id);
}

#[test]
fn p17_name_overrides() {
    let fixture = fixture(&[("Mods/Taken/t.pak", b"t")]);
    let zip = fixture.zip("Pack.zip", &[("A/a.pak", b"a"), ("B/b.pak", b"b")]);
    let options = |names: Vec<NameOverride>| InspectOptions {
        names,
        ..InspectOptions::default()
    };
    let invalid = fixture.inspect_with(
        &zip,
        &options(vec![NameOverride {
            id: 0,
            name: "CON".to_owned(),
        }]),
    );
    assert_eq!(invalid.candidates[0].fixed_name.as_deref(), Some("CON_"));
    let taken = fixture.inspect_with(
        &zip,
        &options(vec![NameOverride {
            id: 0,
            name: "Taken".to_owned(),
        }]),
    );
    assert!(taken.candidates[0].existing.is_some());
    let same = fixture.inspect_with(
        &zip,
        &options(vec![
            NameOverride {
                id: 0,
                name: "Same".to_owned(),
            },
            NameOverride {
                id: 1,
                name: "same".to_owned(),
            },
        ]),
    );
    assert_eq!(
        blocked_by(install_error(fixture.start(&same))),
        InstallBlock::InvalidName
    );
}

#[test]
fn unknown_inspection_is_refused() {
    let fixture = fixture(&[]);
    let result = fixture.fx.core.install_start(
        &remmods_core::install::StartRequest {
            inspection: "insp-unknown".to_owned(),
            picks: Vec::new(),
            clashes: Vec::new(),
            auto_fix: false,
            apply_while_running: false,
        },
        &|_| {},
    );
    assert!(matches!(
        result,
        Err(remmods_core::CoreError::Install(
            InstallError::UnknownInspection
        ))
    ));
}
