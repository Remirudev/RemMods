#![allow(clippy::expect_used)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use remmods_core::mods as core_mods;
use remmods_core::profiles as core;
use remmods_core::skins::LinkedSkins;
use remmods_ipc::{
    ApplyDirection, ApplyOutcome, ApplyRequest, ChangeReason, DiffSkip, EntryState, ExportReport,
    ImportProblem, ImportReport, Profile, ProfileDiff, ProfileEntryDto, ProfileMode, ProfileSave,
    ProfileSource, ProfilesView, SkipReason, StoreProblem, TransferOutcome, TransferRequest,
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

fn round_trips<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(value: &T) {
    assert_eq!(&from_json::<T>(to_json(value)), value);
}

fn core_profile(source: core::ProfileSource) -> core::Profile {
    let mut mods = BTreeMap::new();
    mods.insert(
        "Mods/b".to_owned(),
        core::ProfileEntry::new(core::EntryState::Disabled),
    );
    mods.insert(
        "Mods/B".to_owned(),
        core::ProfileEntry::new(core::EntryState::Enabled),
    );
    mods.insert(
        "LogicMods/a".to_owned(),
        core::ProfileEntry::new(core::EntryState::Enabled),
    );
    let mut extra = serde_json::Map::new();
    extra.insert("future".into(), json!(1));
    core::Profile {
        id: "p1".into(),
        name: "Tournament".into(),
        description: "d".into(),
        mode: core::ProfileMode::Exclusive,
        mods,
        created: "2026-11-01T10:00:00Z".into(),
        updated: "2026-11-01T11:00:00Z".into(),
        last_applied: None,
        source,
        extra,
    }
}

fn refreshed() -> core_mods::Refreshed {
    core_mods::Refreshed {
        snapshot: Arc::new(core_mods::ScanSnapshot {
            revision: 3,
            scanned_at: (SystemTime::UNIX_EPOCH + Duration::from_secs(2)).into(),
            stats: core_mods::ScanStats {
                dirs_read: 1,
                entries_seen: 0,
                duration: Duration::from_millis(1),
                containers_read: 0,
                containers_cached: 0,
                content_duration: Duration::ZERO,
            },
            content: core_mods::ScanContent {
                paks: PathBuf::from("Paks"),
                paks_id: "3-4".into(),
                roots: vec![],
                rows: vec![],
                archives: vec![],
                unmanaged: vec![],
                leftovers: vec![],
            },
        }),
        changed: true,
    }
}

#[test]
fn profile_mods_map_becomes_a_list_sorted_by_key_bytes_without_extra() {
    let dto = Profile::from(&core_profile(core::ProfileSource::User));

    assert_eq!(
        dto.mods,
        vec![
            ProfileEntryDto {
                key: "LogicMods/a".into(),
                state: EntryState::Enabled,
            },
            ProfileEntryDto {
                key: "Mods/B".into(),
                state: EntryState::Enabled,
            },
            ProfileEntryDto {
                key: "Mods/b".into(),
                state: EntryState::Disabled,
            },
        ]
    );
    let json = to_json(&dto);
    assert_eq!(
        json,
        json!({
            "id": "p1",
            "name": "Tournament",
            "description": "d",
            "skinSet": null,
            "mode": "exclusive",
            "mods": [
                { "key": "LogicMods/a", "state": "enabled" },
                { "key": "Mods/B", "state": "enabled" },
                { "key": "Mods/b", "state": "disabled" },
            ],
            "created": "2026-11-01T10:00:00Z",
            "updated": "2026-11-01T11:00:00Z",
            "lastApplied": null,
            "source": { "kind": "user" },
        })
    );
    round_trips(&dto);
}

#[test]
fn profile_sources_are_tagged_by_kind() {
    let cases = [
        (core::ProfileSource::User, json!({ "kind": "user" })),
        (
            core::ProfileSource::Snapshot { of: "p0".into() },
            json!({ "kind": "snapshot", "of": "p0" }),
        ),
        (
            core::ProfileSource::OtherManager {
                id: "guid".into(),
                is_default: true,
            },
            json!({ "kind": "otherManager", "id": "guid", "isDefault": true }),
        ),
        (core::ProfileSource::Import, json!({ "kind": "import" })),
    ];
    for (source, expected) in cases {
        let dto = ProfileSource::from(&source);
        assert_eq!(to_json(&dto), expected);
        round_trips(&dto);
    }
}

#[test]
fn profiles_view_converts_summaries_and_problem() {
    let view = core::ProfilesView {
        profiles: vec![core::ProfileSummary {
            profile: core_profile(core::ProfileSource::Import),
            enabled: 2,
            disabled: 1,
            missing: 1,
            matches_current: false,
        }],
        last_applied: Some(core::LastApplied {
            profile_id: "p1".into(),
            at: "2026-11-01T12:00:00Z".into(),
        }),
        read_only: true,
        problem: Some(core::StoreProblem::NewerVersion),
    };

    let dto = ProfilesView::from(&view);
    let json = to_json(&dto);

    assert_eq!(json["readOnly"], json!(true));
    assert_eq!(json["problem"], json!("newerVersion"));
    assert_eq!(
        json["lastApplied"],
        json!({ "profileId": "p1", "at": "2026-11-01T12:00:00Z" })
    );
    assert_eq!(json["profiles"][0]["matchesCurrent"], json!(false));
    assert_eq!(json["profiles"][0]["missing"], json!(1));
    assert_eq!(
        json["profiles"][0]["profile"]["source"],
        json!({ "kind": "import" })
    );
    round_trips(&dto);

    let empty = ProfilesView::from(&core::ProfilesView {
        profiles: vec![],
        last_applied: None,
        read_only: false,
        problem: None,
    });
    assert_eq!(
        to_json(&empty),
        json!({ "profiles": [], "lastApplied": null, "readOnly": false, "problem": null })
    );
}

#[test]
fn store_problems_are_camel_case() {
    let cases = [
        (
            core::StoreProblem::Corrupt,
            StoreProblem::Corrupt,
            "corrupt",
        ),
        (
            core::StoreProblem::NewerVersion,
            StoreProblem::NewerVersion,
            "newerVersion",
        ),
        (
            core::StoreProblem::Unreadable,
            StoreProblem::Unreadable,
            "unreadable",
        ),
    ];
    for (core_problem, problem, name) in cases {
        assert_eq!(StoreProblem::from(core_problem), problem);
        assert_eq!(to_json(&problem), json!(name));
    }
}

#[test]
fn profile_save_is_tagged_and_converts_into_core() {
    let new: ProfileSave =
        from_json(json!({ "kind": "new", "name": "A", "description": "", "mode": "additive" }));
    assert_eq!(
        core::ProfileSave::from(&new),
        core::ProfileSave::New {
            name: "A".into(),
            description: String::new(),
            mode: core::ProfileMode::Additive,
        }
    );
    let overwrite: ProfileSave = from_json(json!({ "kind": "overwrite", "id": "p1" }));
    assert_eq!(
        core::ProfileSave::from(&overwrite),
        core::ProfileSave::Overwrite { id: "p1".into() }
    );
    let edit = ProfileSave::Edit {
        id: "p1".into(),
        name: "B".into(),
        description: "x".into(),
        mode: ProfileMode::Exclusive,
    };
    assert_eq!(
        to_json(&edit),
        json!({ "kind": "edit", "id": "p1", "name": "B", "description": "x", "mode": "exclusive" })
    );
    assert_eq!(
        core::ProfileSave::from(&edit),
        core::ProfileSave::Edit {
            id: "p1".into(),
            name: "B".into(),
            description: "x".into(),
            mode: core::ProfileMode::Exclusive,
        }
    );
}

#[test]
fn picked_save_keeps_the_list_order_and_converts_states() {
    let picked: ProfileSave = from_json(json!({
        "kind": "picked",
        "id": null,
        "name": "Fine",
        "description": "",
        "mode": "additive",
        "mods": [
            { "key": "Standard:B", "state": "disabled" },
            { "key": "Standard:A", "state": "enabled" },
        ],
        "skins": null,
    }));
    assert_eq!(
        core::ProfileSave::from(&picked),
        core::ProfileSave::Picked {
            id: None,
            name: "Fine".into(),
            description: String::new(),
            mode: core::ProfileMode::Additive,
            mods: vec![
                ("Standard:B".into(), core::EntryState::Disabled),
                ("Standard:A".into(), core::EntryState::Enabled),
            ],
            skins: None,
        }
    );
    let edit = ProfileSave::Picked {
        id: Some("p1".into()),
        name: "G".into(),
        description: "x".into(),
        mode: ProfileMode::Exclusive,
        mods: vec![ProfileEntryDto {
            key: "Logic:L".into(),
            state: EntryState::Enabled,
        }],
        skins: Some("s1".into()),
    };
    assert_eq!(
        to_json(&edit),
        json!({
            "kind": "picked",
            "id": "p1",
            "name": "G",
            "description": "x",
            "mode": "exclusive",
            "mods": [{ "key": "Logic:L", "state": "enabled" }],
            "skins": "s1",
        })
    );
    round_trips(&edit);
}

#[test]
fn profile_diff_converts_changes_skips_and_fingerprint() {
    let diff = core::ProfileDiff {
        profile_id: "p1".into(),
        mode: core::ProfileMode::Additive,
        changes: vec![core::DiffChange {
            mod_ref: core_mods::ModRef::from_token("1|d|1-2|3-4|Mods/X"),
            key: "Mods/X".into(),
            name: "X".into(),
            enable: true,
            reason: core::ChangeReason::Listed,
        }],
        missing: vec!["Mods/Gone".into()],
        skipped: vec![core::DiffSkip {
            key: "Mods/L".into(),
            name: None,
            wanted: core::EntryState::Disabled,
            reason: core::SkipReason::Linked,
        }],
        unchanged: 5,
        fingerprint: "+Mods/X\n".into(),
    };

    let dto = ProfileDiff::from(&diff);

    assert_eq!(
        to_json(&dto),
        json!({
            "profileId": "p1",
            "mode": "additive",
            "changes": [{
                "modRef": "1|d|1-2|3-4|Mods/X",
                "key": "Mods/X",
                "name": "X",
                "enable": true,
                "reason": "listed",
            }],
            "missing": ["Mods/Gone"],
            "skipped": [{ "key": "Mods/L", "name": null, "wanted": "disabled", "reason": "linked" }],
            "unchanged": 5,
            "fingerprint": "+Mods/X\n",
        })
    );
    assert_eq!(dto.changes[0].reason, ChangeReason::Listed);
    round_trips(&dto);
}

#[test]
fn skip_reasons_are_camel_case() {
    let cases = [
        (
            core::SkipReason::Ambiguous,
            SkipReason::Ambiguous,
            "ambiguous",
        ),
        (core::SkipReason::Linked, SkipReason::Linked, "linked"),
        (
            core::SkipReason::UnknownState,
            SkipReason::UnknownState,
            "unknownState",
        ),
        (
            core::SkipReason::NoPakFiles,
            SkipReason::NoPakFiles,
            "noPakFiles",
        ),
        (
            core::SkipReason::InvalidLogicName,
            SkipReason::InvalidLogicName,
            "invalidLogicName",
        ),
    ];
    for (core_reason, reason, name) in cases {
        assert_eq!(SkipReason::from(core_reason), reason);
        assert_eq!(to_json(&reason), json!(name));
    }
}

#[test]
fn apply_request_passes_the_fingerprint_and_direction_unchanged() {
    let json = json!({
        "profileIds": ["p1", "p2"],
        "fingerprint": "-Mods/Y\n+Mods/X\n",
        "applyWhileRunning": false,
        "direction": "off",
    });
    let request: ApplyRequest = from_json(json.clone());
    assert_eq!(to_json(&request), json);
    assert_eq!(
        core::ApplyRequest::from(&request),
        core::ApplyRequest {
            profile_ids: vec!["p1".into(), "p2".into()],
            fingerprint: "-Mods/Y\n+Mods/X\n".into(),
            apply_while_running: false,
            direction: core::ApplyDirection::Off,
        }
    );
}

#[test]
fn apply_directions_are_camel_case() {
    for (direction, core_direction, name) in [
        (ApplyDirection::On, core::ApplyDirection::On, "on"),
        (ApplyDirection::Off, core::ApplyDirection::Off, "off"),
    ] {
        assert_eq!(to_json(&direction), json!(name));
        assert_eq!(core::ApplyDirection::from(direction), core_direction);
    }
}

#[test]
fn apply_outcome_converts_snapshot_and_skips() {
    let outcome = ApplyOutcome::from(&core::ApplyOutcome {
        refreshed: refreshed(),
        changed: 2,
        snapshot_id: Some("s1".into()),
        skipped: vec![core::DiffSkip {
            key: "Mods/A".into(),
            name: Some("A".into()),
            wanted: core::EntryState::Enabled,
            reason: core::SkipReason::Ambiguous,
        }],
        skins: Some(LinkedSkins::GameRunning { name: "A".into() }),
    });
    let json = to_json(&outcome);
    assert_eq!(json["skins"], json!({ "kind": "gameRunning", "name": "A" }));
    assert_eq!(json["snapshot"]["revision"], json!(3));
    assert_eq!(json["changed"], json!(2));
    assert_eq!(json["snapshotId"], json!("s1"));
    assert_eq!(
        outcome.skipped,
        vec![DiffSkip {
            key: "Mods/A".into(),
            name: Some("A".into()),
            wanted: EntryState::Enabled,
            reason: SkipReason::Ambiguous,
        }]
    );
    round_trips(&outcome);
}

#[test]
fn transfer_request_and_outcome_are_tagged() {
    let cases = [
        (
            TransferRequest::ImportOtherManager { title: "T".into() },
            json!({ "kind": "importOtherManager", "title": "T" }),
        ),
        (
            TransferRequest::Import { title: "T".into() },
            json!({ "kind": "import", "title": "T" }),
        ),
        (
            TransferRequest::Export {
                title: "T".into(),
                default_file_name: "remmods-profiles.json".into(),
            },
            json!({ "kind": "export", "title": "T", "defaultFileName": "remmods-profiles.json" }),
        ),
    ];
    for (request, expected) in cases {
        assert_eq!(to_json(&request), expected);
        round_trips(&request);
    }

    let exported = TransferOutcome::Exported {
        report: ExportReport::from(&core::ExportReport {
            count: 4,
            file_name: "remmods-profiles.json".into(),
        }),
    };
    assert_eq!(
        to_json(&exported),
        json!({ "kind": "exported", "report": { "count": 4, "fileName": "remmods-profiles.json" } })
    );
    let imported = TransferOutcome::Imported {
        report: ImportReport::from(&core::ImportReport {
            added: 1,
            updated: 2,
            unchanged: 3,
            skipped: 4,
            view: core::ProfilesView {
                profiles: vec![],
                last_applied: None,
                read_only: false,
                problem: None,
            },
        }),
    };
    let json = to_json(&imported);
    assert_eq!(json["kind"], json!("imported"));
    assert_eq!(json["report"]["skipped"], json!(4));
    assert_eq!(json["report"]["view"]["profiles"], json!([]));
    round_trips(&imported);
}

#[test]
fn import_problems_are_camel_case() {
    let cases = [
        (
            core::ImportProblem::TooLarge,
            ImportProblem::TooLarge,
            "tooLarge",
        ),
        (
            core::ImportProblem::NotJson,
            ImportProblem::NotJson,
            "notJson",
        ),
        (
            core::ImportProblem::WrongShape,
            ImportProblem::WrongShape,
            "wrongShape",
        ),
        (
            core::ImportProblem::WrongFormat,
            ImportProblem::WrongFormat,
            "wrongFormat",
        ),
        (
            core::ImportProblem::NewerVersion,
            ImportProblem::NewerVersion,
            "newerVersion",
        ),
    ];
    for (core_problem, problem, name) in cases {
        assert_eq!(ImportProblem::from(core_problem), problem);
        assert_eq!(to_json(&problem), json!(name));
    }
}
