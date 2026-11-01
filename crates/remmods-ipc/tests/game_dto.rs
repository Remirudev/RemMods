#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};

use remmods_core::game as core;
use remmods_core::paths::display_path;
use remmods_ipc::{
    DetectionNote, FolderProblem, GameCandidate, GameLocation, GameStatus, InstallInfo,
    LocationChoice, LocationSource, NoteCode, PickOutcome, RevealProblem, RunningState, SteamKind,
    UpdateNotice, UpdateTracking,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

fn to_json<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialize")
}

fn from_json<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).expect("deserialize")
}

fn game_root() -> PathBuf {
    Path::new("steamapps").join("common").join("TEKKEN 8")
}

fn core_install() -> core::InstallInfo {
    let root = game_root();
    core::InstallInfo {
        paks: root.join("Polaris").join("Content").join("Paks"),
        exe: root
            .join("Polaris")
            .join("Binaries")
            .join("Win64")
            .join("Polaris-Win64-Shipping.exe"),
        root,
        source: core::LocationSource::Steam,
        steam_kind: Some(core::SteamKind::Flatpak),
        build_id: Some("25221600".into()),
        update_tracking: core::UpdateTracking::Available,
    }
}

fn core_status() -> core::GameStatus {
    core::GameStatus {
        location: core::GameLocation::Ready(core_install()),
        running: core::RunningState::Running,
        session: 2,
        original_app: true,
        update: Some(core::UpdateNotice {
            from: "1".into(),
            to: "25221600".into(),
        }),
        candidates: vec![core::GameCandidate {
            id: "c1-0".into(),
            root: game_root(),
            library: PathBuf::from("library"),
            steam_kind: core::SteamKind::Native,
            build_id: None,
            fully_installed: false,
            problem: Some(core::FolderProblem::Ambiguous),
            in_use: true,
        }],
        notes: vec![core::DetectionNote {
            code: core::NoteCode::VdfError,
            path: PathBuf::from("libraryfolders.vdf"),
            line: Some(14),
            detail: Some("unterminatedString".into()),
        }],
    }
}

#[test]
fn unit_enums_are_camel_case_strings() {
    let running = [
        (RunningState::Running, "running"),
        (RunningState::NotRunning, "notRunning"),
        (RunningState::Unknown, "unknown"),
    ];
    for (state, expected) in running {
        assert_eq!(to_json(&state), json!(expected));
        assert_eq!(from_json::<RunningState>(json!(expected)), state);
    }
    let sources = [
        (LocationSource::Steam, "steam"),
        (LocationSource::Manual, "manual"),
        (LocationSource::Fixture, "fixture"),
    ];
    for (source, expected) in sources {
        assert_eq!(to_json(&source), json!(expected));
    }
    let kinds = [
        (SteamKind::Native, "native"),
        (SteamKind::Flatpak, "flatpak"),
        (SteamKind::Snap, "snap"),
        (SteamKind::Windows, "windows"),
    ];
    for (kind, expected) in kinds {
        assert_eq!(to_json(&kind), json!(expected));
    }
    let tracking = [
        (UpdateTracking::Available, "available"),
        (UpdateTracking::NoManifest, "noManifest"),
        (UpdateTracking::Unreadable, "unreadable"),
    ];
    for (value, expected) in tracking {
        assert_eq!(to_json(&value), json!(expected));
    }
    let reveal = [
        (RevealProblem::UnsupportedName, "unsupportedName"),
        (RevealProblem::Failed, "failed"),
    ];
    for (problem, expected) in reveal {
        assert_eq!(to_json(&problem), json!(expected));
    }
}

#[test]
fn folder_problems_map_from_core() {
    let cases = [
        (
            core::FolderProblem::NotFound,
            FolderProblem::NotFound,
            "notFound",
        ),
        (
            core::FolderProblem::Ambiguous,
            FolderProblem::Ambiguous,
            "ambiguous",
        ),
        (
            core::FolderProblem::NotUnicode,
            FolderProblem::NotUnicode,
            "notUnicode",
        ),
        (
            core::FolderProblem::Unreadable,
            FolderProblem::Unreadable,
            "unreadable",
        ),
    ];
    for (core_problem, problem, expected) in cases {
        assert_eq!(FolderProblem::from(core_problem), problem);
        assert_eq!(to_json(&problem), json!(expected));
    }
}

#[test]
fn note_codes_map_from_core() {
    let cases = [
        (core::NoteCode::SteamRoot, NoteCode::SteamRoot, "steamRoot"),
        (core::NoteCode::NoSteam, NoteCode::NoSteam, "noSteam"),
        (core::NoteCode::VdfError, NoteCode::VdfError, "vdfError"),
        (
            core::NoteCode::LibraryMissing,
            NoteCode::LibraryMissing,
            "libraryMissing",
        ),
        (
            core::NoteCode::LibraryNotAbsolute,
            NoteCode::LibraryNotAbsolute,
            "libraryNotAbsolute",
        ),
        (
            core::NoteCode::ManifestError,
            NoteCode::ManifestError,
            "manifestError",
        ),
        (
            core::NoteCode::InstallIncomplete,
            NoteCode::InstallIncomplete,
            "installIncomplete",
        ),
        (
            core::NoteCode::GameFolderInvalid,
            NoteCode::GameFolderInvalid,
            "gameFolderInvalid",
        ),
        (
            core::NoteCode::PathNotUnicode,
            NoteCode::PathNotUnicode,
            "pathNotUnicode",
        ),
        (
            core::NoteCode::RegistryUnreadable,
            NoteCode::RegistryUnreadable,
            "registryUnreadable",
        ),
    ];
    for (core_code, code, expected) in cases {
        assert_eq!(NoteCode::from(core_code), code);
        assert_eq!(to_json(&code), json!(expected));
    }
}

#[test]
fn game_location_is_tagged_by_kind() {
    let install = InstallInfo {
        root: "/g".into(),
        paks: "/g/Polaris/Content/Paks".into(),
        source: LocationSource::Manual,
        steam_kind: None,
        build_id: None,
        update_tracking: UpdateTracking::NoManifest,
    };
    let cases = [
        (GameLocation::NotSet, json!({ "kind": "notSet" })),
        (
            GameLocation::Ready { install },
            json!({
                "kind": "ready",
                "install": {
                    "root": "/g",
                    "paks": "/g/Polaris/Content/Paks",
                    "source": "manual",
                    "steamKind": null,
                    "buildId": null,
                    "updateTracking": "noManifest"
                }
            }),
        ),
        (
            GameLocation::Missing { path: "/g".into() },
            json!({ "kind": "missing", "path": "/g" }),
        ),
    ];
    for (location, expected) in cases {
        assert_eq!(to_json(&location), expected);
        assert_eq!(from_json::<GameLocation>(expected), location);
    }
}

#[test]
fn game_status_shape() {
    let status = GameStatus {
        location: GameLocation::NotSet,
        running: RunningState::Unknown,
        session: 0,
        original_app: false,
        update: None,
        candidates: vec![GameCandidate {
            id: "c0-0".into(),
            root: "/s/steamapps/common/TEKKEN 8".into(),
            library: "/s".into(),
            steam_kind: SteamKind::Snap,
            build_id: Some("25221600".into()),
            fully_installed: true,
            problem: None,
            in_use: false,
        }],
        notes: vec![DetectionNote {
            code: NoteCode::NoSteam,
            path: String::new(),
            line: None,
            detail: None,
        }],
    };

    let expected = json!({
        "location": { "kind": "notSet" },
        "running": "unknown",
        "session": 0,
        "originalApp": false,
        "update": null,
        "candidates": [{
            "id": "c0-0",
            "root": "/s/steamapps/common/TEKKEN 8",
            "library": "/s",
            "steamKind": "snap",
            "buildId": "25221600",
            "fullyInstalled": true,
            "problem": null,
            "inUse": false
        }],
        "notes": [{ "code": "noSteam", "path": "", "line": null, "detail": null }]
    });
    assert_eq!(to_json(&status), expected);
    assert_eq!(from_json::<GameStatus>(expected), status);
}

#[test]
fn game_status_converts_from_core() {
    let root = game_root();

    let status = GameStatus::from(&core_status());

    assert_eq!(
        status.location,
        GameLocation::Ready {
            install: InstallInfo {
                root: display_path(&root),
                paks: display_path(&root.join("Polaris").join("Content").join("Paks")),
                source: LocationSource::Steam,
                steam_kind: Some(SteamKind::Flatpak),
                build_id: Some("25221600".into()),
                update_tracking: UpdateTracking::Available,
            },
        }
    );
    assert_eq!(status.running, RunningState::Running);
    assert_eq!(status.session, 2);
    assert!(status.original_app);
    assert_eq!(
        status.update,
        Some(UpdateNotice {
            from: "1".into(),
            to: "25221600".into(),
        })
    );
    assert_eq!(
        status.candidates,
        vec![GameCandidate {
            id: "c1-0".into(),
            root: display_path(&root),
            library: "library".into(),
            steam_kind: SteamKind::Native,
            build_id: None,
            fully_installed: false,
            problem: Some(FolderProblem::Ambiguous),
            in_use: true,
        }]
    );
    assert_eq!(
        status.notes,
        vec![DetectionNote {
            code: NoteCode::VdfError,
            path: "libraryfolders.vdf".into(),
            line: Some(14),
            detail: Some("unterminatedString".into()),
        }]
    );
}

#[test]
fn install_info_leaves_the_exe_out() {
    let value = to_json(&InstallInfo::from(&core_install()));

    assert!(value.get("exe").is_none());
    assert_eq!(value["steamKind"], json!("flatpak"));
    assert_eq!(value["updateTracking"], json!("available"));
}

#[test]
fn missing_and_unset_locations_convert() {
    let missing = core::GameLocation::Missing { path: game_root() };

    assert_eq!(
        GameLocation::from(&missing),
        GameLocation::Missing {
            path: display_path(&game_root()),
        }
    );
    assert_eq!(
        GameLocation::from(&core::GameLocation::NotSet),
        GameLocation::NotSet
    );
}

#[test]
fn location_choice_is_input_only_and_converts_to_core() {
    let candidate: LocationChoice = from_json(json!({ "kind": "candidate", "id": "c3-1" }));
    let path: LocationChoice = from_json(json!({ "kind": "path", "path": "~/Games/TEKKEN 8" }));

    assert_eq!(
        core::LocationChoice::from(candidate),
        core::LocationChoice::Candidate { id: "c3-1".into() }
    );
    assert_eq!(
        core::LocationChoice::from(path),
        core::LocationChoice::Path {
            path: PathBuf::from("~/Games/TEKKEN 8"),
        }
    );
}

#[test]
fn pick_outcome_is_tagged_by_kind() {
    let invalid = PickOutcome::Invalid {
        reason: FolderProblem::NotFound,
        path: "/home/user/Downloads".into(),
    };

    assert_eq!(
        to_json(&PickOutcome::Cancelled),
        json!({ "kind": "cancelled" })
    );
    assert_eq!(
        to_json(&invalid),
        json!({ "kind": "invalid", "reason": "notFound", "path": "/home/user/Downloads" })
    );
    let selected = to_json(&PickOutcome::Selected {
        status: GameStatus::from(&core_status()),
    });
    assert_eq!(selected["kind"], json!("selected"));
    assert_eq!(selected["status"]["location"]["kind"], json!("ready"));
    assert_eq!(
        from_json::<PickOutcome>(json!({ "kind": "cancelled" })),
        PickOutcome::Cancelled
    );
}

#[test]
fn pick_results_convert_from_core() {
    let picked = Path::new("Downloads").join("TEKKEN 8");
    let invalid = core::PickResult::Invalid {
        problem: core::FolderProblem::NotUnicode,
        path: picked.clone(),
    };
    let selected = core::PickResult::Selected(core_status());

    assert_eq!(
        PickOutcome::from(&invalid),
        PickOutcome::Invalid {
            reason: FolderProblem::NotUnicode,
            path: display_path(&picked),
        }
    );
    assert_eq!(
        PickOutcome::from(&selected),
        PickOutcome::Selected {
            status: GameStatus::from(&core_status()),
        }
    );
}
