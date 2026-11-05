//! `Core::mods_trash` refusals on both OSes and the Logic anomalies the toggle finishes
//! (spec §5.3, §5.6, §8.2 R1, R2, R5–R8). The Linux moves are in `platform_linux_mods.rs`.
#![allow(clippy::expect_used)]

mod mods_support;
mod support;

use mods_support::DATA;
use mods_support::manage::{core_with_trash, core_without_trash, entry_target, mods_error, tree};
use remmods_core::mods::{ManageAction, ManageBlock, ModsError, TrashRequest};
use remmods_core::platform::{self, OsKind, TrashUnavailableReason};

fn trash_request(target: remmods_core::mods::ModTarget) -> TrashRequest {
    TrashRequest {
        target,
        apply_while_running: false,
    }
}

fn assert_blocked<T: std::fmt::Debug>(
    result: Result<T, remmods_core::CoreError>,
    expected: ManageBlock,
) {
    match mods_error(result) {
        ModsError::ManageBlocked {
            action: ManageAction::Trash,
            reason,
            ..
        } => assert_eq!(reason, expected),
        other => panic!("expected ManageBlocked({expected:?}), got {other:?}"),
    }
}

#[test]
fn trash_unavailable_reports_size_and_changes_nothing() {
    let fixture = core_without_trash(&[("Mods/Cool/Cool.pak", DATA), ("Mods/Cool/a.txt", DATA)]);
    let before = tree(&fixture);
    let row = fixture.row("Standard:Cool");
    let result = fixture
        .core
        .mods_trash(&trash_request(fixture.target("Standard:Cool")));
    let expected = if platform::os_kind() == OsKind::Windows {
        TrashUnavailableReason::NotSupported
    } else {
        TrashUnavailableReason::OtherDevice
    };
    match mods_error(result) {
        ModsError::TrashUnavailable { reason, size } => {
            assert_eq!(reason, expected);
            assert_eq!(size, row.size);
            assert!(size.is_some());
        }
        other => panic!("expected TrashUnavailable, got {other:?}"),
    }
    assert_eq!(tree(&fixture), before);
}

#[test]
fn trash_refused_for_unmanaged_and_linked() {
    let fixture = core_with_trash(&[("Mods/Cool/Cool.pak", DATA), ("Stray.pak", DATA)]);
    let before = tree(&fixture);
    let target = entry_target(&fixture.snapshot().content, "Stray.pak");
    assert_blocked(
        fixture.core.mods_trash(&trash_request(target)),
        ManageBlock::Unmanaged,
    );
    assert_eq!(tree(&fixture), before);
    assert_eq!(
        remmods_core::mods::manage_block(
            remmods_core::mods::ManagedKind::Folder,
            true,
            ManageAction::Trash
        ),
        Some(ManageBlock::Linked)
    );
}

#[test]
fn trash_keeps_profile_and_meta_keys() {
    let fixture = core_without_trash(&[("Mods/Cool/Cool.pak", DATA)]);
    fixture.toggle(&["Standard:Cool"], false).expect("disable");
    let save = remmods_core::profiles::ProfileSave::New {
        name: "P".to_owned(),
        description: String::new(),
        mode: remmods_core::profiles::ProfileMode::Exclusive,
    };
    fixture.core.profiles_save(&save).expect("save");
    let meta = std::fs::read(fixture.paths.data_dir.join("mod-meta.json")).expect("meta");
    let profiles = std::fs::read(fixture.paths.data_dir.join("profiles.json")).expect("profiles");
    let _ = fixture
        .core
        .mods_trash(&trash_request(fixture.target("Standard:Cool")));
    assert_eq!(
        std::fs::read(fixture.paths.data_dir.join("mod-meta.json")).expect("meta"),
        meta
    );
    assert_eq!(
        std::fs::read(fixture.paths.data_dir.join("profiles.json")).expect("profiles"),
        profiles
    );
}

#[test]
fn trash_archive_directly_in_paks_refused() {
    let fixture = core_with_trash(&[("Pack.zip", DATA), ("Mods/In.rar", DATA)]);
    let before = tree(&fixture);
    let content = fixture.snapshot().content.clone();
    assert_blocked(
        fixture
            .core
            .mods_trash(&trash_request(entry_target(&content, "Pack.zip"))),
        ManageBlock::OutsideRoots,
    );
    assert_eq!(tree(&fixture), before);
}

#[test]
fn toggle_finishes_logic_disabled_but_loaded() {
    let fixture = core_with_trash(&[
        ("LogicMods/.disabled_X/X.pak", DATA),
        ("LogicMods/.disabled_X/X.ucas", DATA),
    ]);
    let row = fixture.row("Logic:X");
    assert!(
        row.issues
            .contains(&remmods_core::mods::ModIssue::LogicDisabledButLoaded)
    );
    fixture.toggle(&["Logic:X"], false).expect("finish disable");
    assert!(fixture.exists("LogicMods/.disabled_X/X.pak-x"));
    assert!(fixture.exists("LogicMods/.disabled_X/X.ucas-x"));
    assert!(!fixture.exists("LogicMods/.disabled_X/X.pak"));
}

#[test]
fn toggle_finishes_logic_enabled_not_loaded() {
    let fixture = core_with_trash(&[
        ("LogicMods/X/X.pak-x", DATA),
        ("LogicMods/X/X.ucas-x", DATA),
    ]);
    let row = fixture.row("Logic:X");
    assert!(
        row.issues
            .contains(&remmods_core::mods::ModIssue::LogicEnabledNotLoaded)
    );
    fixture.toggle(&["Logic:X"], true).expect("finish enable");
    assert!(fixture.exists("LogicMods/X/X.pak"));
    assert!(fixture.exists("LogicMods/X/X.ucas"));
    assert!(!fixture.exists("LogicMods/.disabled_X"));
}
