#![allow(clippy::expect_used)]

use std::io;
use std::path::Path;

use remmods_core::CoreError;
use remmods_core::game::{FolderProblem as CoreFolderProblem, GameError};
use remmods_core::mods::ModsError;
use remmods_core::mods::ToggleBlock as CoreToggleBlock;
use remmods_core::platform;
use remmods_core::safe_fs::{BatchError, FsError, JournalProblem, ScopeViolation, StuckOp};
use remmods_core::settings::SettingsError;
use remmods_ipc::{AppError, FolderProblem, OsErrorClass, RevealProblem, ToggleBlock};
use serde_json::{Value, json};

fn to_json(error: &AppError) -> Value {
    serde_json::to_value(error).expect("serialize")
}

fn io_error(kind: io::ErrorKind) -> FsError {
    FsError::io("write", Path::new("settings.json"), io::Error::from(kind))
}

#[test]
fn every_variant_is_tagged_by_kind() {
    let cases = [
        (AppError::Busy, json!({ "kind": "busy" })),
        (
            AppError::Io {
                op: "write".into(),
                class: OsErrorClass::InUse,
            },
            json!({ "kind": "io", "op": "write", "class": "inUse" }),
        ),
        (
            AppError::InvalidInput {
                field: "smoke".into(),
            },
            json!({ "kind": "invalidInput", "field": "smoke" }),
        ),
        (AppError::ReadOnly, json!({ "kind": "readOnly" })),
        (AppError::Internal, json!({ "kind": "internal" })),
    ];
    for (error, expected) in cases {
        assert_eq!(to_json(&error), expected);
        let parsed: AppError = serde_json::from_value(expected).expect("deserialize");
        assert_eq!(parsed, error);
    }
}

#[test]
fn os_error_classes_are_camel_case() {
    let cases = [
        (OsErrorClass::InUse, "inUse"),
        (OsErrorClass::AccessDenied, "accessDenied"),
        (OsErrorClass::NoSpace, "noSpace"),
        (OsErrorClass::ReadOnlyFs, "readOnlyFs"),
        (OsErrorClass::CrossDevice, "crossDevice"),
        (OsErrorClass::NameTooLong, "nameTooLong"),
        (OsErrorClass::AlreadyExists, "alreadyExists"),
        (OsErrorClass::NotFound, "notFound"),
        (OsErrorClass::Unsupported, "unsupported"),
        (OsErrorClass::Other, "other"),
    ];
    for (class, expected) in cases {
        assert_eq!(
            serde_json::to_value(class).expect("serialize"),
            json!(expected)
        );
    }
}

#[test]
fn core_classes_map_one_to_one() {
    let cases = [
        (platform::OsErrorClass::InUse, OsErrorClass::InUse),
        (
            platform::OsErrorClass::AccessDenied,
            OsErrorClass::AccessDenied,
        ),
        (platform::OsErrorClass::NoSpace, OsErrorClass::NoSpace),
        (platform::OsErrorClass::ReadOnlyFs, OsErrorClass::ReadOnlyFs),
        (
            platform::OsErrorClass::CrossDevice,
            OsErrorClass::CrossDevice,
        ),
        (
            platform::OsErrorClass::NameTooLong,
            OsErrorClass::NameTooLong,
        ),
        (
            platform::OsErrorClass::AlreadyExists,
            OsErrorClass::AlreadyExists,
        ),
        (platform::OsErrorClass::NotFound, OsErrorClass::NotFound),
        (
            platform::OsErrorClass::Unsupported,
            OsErrorClass::Unsupported,
        ),
        (platform::OsErrorClass::Other, OsErrorClass::Other),
    ];
    for (core_class, expected) in cases {
        assert_eq!(OsErrorClass::from(core_class), expected);
    }
}

#[test]
fn fs_errors_map_to_busy_and_io() {
    assert_eq!(AppError::from(CoreError::Fs(FsError::Busy)), AppError::Busy);
    assert_eq!(
        AppError::from(CoreError::Fs(io_error(io::ErrorKind::NotFound))),
        AppError::Io {
            op: "write".into(),
            class: OsErrorClass::NotFound,
        }
    );
}

#[test]
fn settings_errors_map_to_their_kinds() {
    let encode_error = serde_json::from_str::<u8>("x").expect_err("invalid json");
    let cases = [
        (SettingsError::ReadOnly, AppError::ReadOnly),
        (
            SettingsError::InvalidValue {
                field: "appearance.uiScale",
            },
            AppError::InvalidInput {
                field: "appearance.uiScale".into(),
            },
        ),
        (SettingsError::Encode(encode_error), AppError::Internal),
        (SettingsError::Fs(FsError::Busy), AppError::Busy),
        (
            SettingsError::Fs(io_error(io::ErrorKind::PermissionDenied)),
            AppError::Io {
                op: "write".into(),
                class: OsErrorClass::AccessDenied,
            },
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(AppError::from(CoreError::Settings(error)), expected);
    }
}

#[test]
fn missing_home_dir_is_internal() {
    assert_eq!(
        AppError::from(CoreError::MissingHomeDir),
        AppError::Internal
    );
}

#[test]
fn serialized_errors_carry_no_os_details() {
    let error = AppError::from(CoreError::Fs(io_error(io::ErrorKind::NotFound)));

    let text = serde_json::to_string(&error).expect("serialize");

    assert!(!text.contains("settings.json"));
    assert_eq!(text, r#"{"kind":"io","op":"write","class":"notFound"}"#);
}

#[test]
fn p1_kinds_round_trip() {
    let cases = [
        (AppError::Stale, json!({ "kind": "stale" })),
        (AppError::GameNotSet, json!({ "kind": "gameNotSet" })),
        (
            AppError::InvalidGameFolder {
                reason: FolderProblem::NotFound,
            },
            json!({ "kind": "invalidGameFolder", "reason": "notFound" }),
        ),
        (AppError::LaunchFailed, json!({ "kind": "launchFailed" })),
        (
            AppError::RevealFailed {
                reason: RevealProblem::UnsupportedName,
            },
            json!({ "kind": "revealFailed", "reason": "unsupportedName" }),
        ),
        (
            AppError::RevealFailed {
                reason: RevealProblem::Failed,
            },
            json!({ "kind": "revealFailed", "reason": "failed" }),
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(to_json(&error), expected);
        let parsed: AppError = serde_json::from_value(expected).expect("deserialize");
        assert_eq!(parsed, error);
    }
}

#[test]
fn game_errors_map_to_their_kinds() {
    let cases = [
        (GameError::NotSet, AppError::GameNotSet),
        (GameError::StaleCandidate, AppError::Stale),
        (
            GameError::InvalidPath,
            AppError::InvalidInput {
                field: "path".into(),
            },
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(AppError::from(CoreError::Game(error)), expected);
    }
}

#[test]
fn invalid_folders_keep_their_reason() {
    let cases = [
        (CoreFolderProblem::NotFound, FolderProblem::NotFound),
        (CoreFolderProblem::Ambiguous, FolderProblem::Ambiguous),
        (CoreFolderProblem::NotUnicode, FolderProblem::NotUnicode),
        (CoreFolderProblem::Unreadable, FolderProblem::Unreadable),
    ];
    for (problem, reason) in cases {
        assert_eq!(
            AppError::from(CoreError::Game(GameError::InvalidFolder(problem))),
            AppError::InvalidGameFolder { reason }
        );
    }
}

#[test]
fn mods_errors_map_to_their_kinds() {
    let scan_failed = FsError::io(
        "scan",
        Path::new("Paks"),
        io::Error::from(io::ErrorKind::NotFound),
    );
    let cases = [
        (
            ModsError::Fs(scan_failed),
            AppError::Io {
                op: "scan".into(),
                class: OsErrorClass::NotFound,
            },
        ),
        (ModsError::Fs(FsError::Busy), AppError::Busy),
        (ModsError::Stale, AppError::Stale),
        (
            ModsError::BadRef,
            AppError::InvalidInput {
                field: "modRef".into(),
            },
        ),
        (
            ModsError::NotShellSafe,
            AppError::RevealFailed {
                reason: RevealProblem::UnsupportedName,
            },
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(AppError::from(CoreError::Mods(error)), expected);
    }
}

#[test]
fn new_kinds_carry_no_paths() {
    let error = AppError::from(CoreError::Game(GameError::InvalidFolder(
        CoreFolderProblem::Unreadable,
    )));

    let text = serde_json::to_string(&error).expect("serialize");

    assert_eq!(
        text,
        r#"{"kind":"invalidGameFolder","reason":"unreadable"}"#
    );
}

fn fs(error: FsError) -> AppError {
    AppError::from(CoreError::Fs(error))
}

fn batch(cause: FsError, stuck: usize) -> FsError {
    let stuck = (0..stuck)
        .map(|index| StuckOp {
            from: format!("a{index}").into(),
            to: format!("b{index}").into(),
        })
        .collect();
    FsError::Batch(Box::new(BatchError {
        failed: 1,
        cause,
        rolled_back: 1,
        stuck,
    }))
}

#[test]
fn p2_kinds_round_trip() {
    let cases = [
        (AppError::GameRunning, json!({ "kind": "gameRunning" })),
        (AppError::TargetExists, json!({ "kind": "targetExists" })),
        (AppError::OutOfScope, json!({ "kind": "outOfScope" })),
        (
            AppError::ToggleBlocked {
                reason: ToggleBlock::NoPakFiles,
            },
            json!({ "kind": "toggleBlocked", "reason": "noPakFiles" }),
        ),
        (
            AppError::BatchFailed {
                class: OsErrorClass::AlreadyExists,
                stuck: 2,
                folder: true,
            },
            json!({ "kind": "batchFailed", "class": "alreadyExists", "stuck": 2, "folder": true }),
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(to_json(&error), expected);
        let parsed: AppError = serde_json::from_value(expected).expect("deserialize");
        assert_eq!(parsed, error);
    }
}

#[test]
fn rename_errors_map_to_their_kinds() {
    let path = Path::new("Paks/Mods/X");
    assert_eq!(
        fs(FsError::OutOfScope {
            path: path.into(),
            reason: ScopeViolation::OutsideRoots,
        }),
        AppError::OutOfScope
    );
    assert_eq!(
        fs(FsError::TargetExists { path: path.into() }),
        AppError::TargetExists
    );
    assert_eq!(
        fs(FsError::SourceMissing { path: path.into() }),
        AppError::Stale
    );
    let probe = FsError::io(
        "probe",
        path,
        io::Error::from(io::ErrorKind::PermissionDenied),
    );
    assert_eq!(
        fs(probe),
        AppError::Io {
            op: "probe".into(),
            class: OsErrorClass::AccessDenied,
        }
    );
}

#[test]
fn internal_fs_errors_map_to_internal() {
    let cases = [
        FsError::FolderBusy {
            path: "Paks/LogicMods/Tool".into(),
            holders: None,
        },
        FsError::InvalidBatch { reason: "order" },
        FsError::Journal {
            reason: JournalProblem::NewerVersion,
        },
    ];
    for error in cases {
        assert_eq!(fs(error), AppError::Internal);
    }
}

#[test]
fn batch_failures_carry_class_stuck_and_folder() {
    let in_use = FsError::io(
        "rename",
        Path::new("X.pak"),
        io::Error::from(io::ErrorKind::PermissionDenied),
    );
    assert_eq!(
        fs(batch(in_use, 0)),
        AppError::BatchFailed {
            class: OsErrorClass::AccessDenied,
            stuck: 0,
            folder: false,
        }
    );
    let folder = FsError::FolderBusy {
        path: "Paks/LogicMods/Tool".into(),
        holders: Some(vec!["explorer.exe".into()]),
    };
    assert_eq!(
        fs(batch(folder, 2)),
        AppError::BatchFailed {
            class: OsErrorClass::InUse,
            stuck: 2,
            folder: true,
        }
    );
    assert_eq!(
        fs(batch(
            FsError::TargetExists {
                path: "X.pak".into()
            },
            0
        )),
        AppError::BatchFailed {
            class: OsErrorClass::AlreadyExists,
            stuck: 0,
            folder: false,
        }
    );
    assert_eq!(
        fs(batch(FsError::InvalidBatch { reason: "fault" }, 1)),
        AppError::BatchFailed {
            class: OsErrorClass::Other,
            stuck: 1,
            folder: false,
        }
    );
}

#[test]
fn toggle_errors_map_to_their_kinds() {
    let cases = [
        (
            ModsError::Blocked {
                key: "Standard:X".into(),
                reason: CoreToggleBlock::Linked,
            },
            AppError::ToggleBlocked {
                reason: ToggleBlock::Linked,
            },
        ),
        (ModsError::GameRunning, AppError::GameRunning),
        (
            ModsError::NotAFixture,
            AppError::InvalidInput {
                field: "fixture".into(),
            },
        ),
        (
            ModsError::Fs(FsError::SourceMissing {
                path: "X.pak".into(),
            }),
            AppError::Stale,
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(AppError::from(CoreError::Mods(error)), expected);
    }
}

#[test]
fn toggle_errors_carry_no_keys_or_paths() {
    let error = AppError::from(CoreError::Mods(ModsError::Blocked {
        key: "Standard:Secret".into(),
        reason: CoreToggleBlock::Ambiguous,
    }));

    let text = serde_json::to_string(&error).expect("serialize");

    assert_eq!(text, r#"{"kind":"toggleBlocked","reason":"ambiguous"}"#);
}
