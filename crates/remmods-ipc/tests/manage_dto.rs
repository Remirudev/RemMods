#![allow(clippy::expect_used)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use remmods_core::mods as core;
use remmods_core::platform;
use remmods_ipc::{
    DeleteOutcome, DeletePrepared, DeleteRequest, ManageAction, ManageBlock, MetadataRequest,
    ModInfoEdit, ModTarget, RekeyReport, RenameOutcome, RenameRequest, TrashKind, TrashOutcome,
    TrashRequest, TrashUnavailableReason,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

const ROW_REF: &str = "1|d|1-2|3-4|Mods/X";

fn to_json<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialize")
}

fn from_json<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).expect("deserialize")
}

fn round_trips<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(value: &T) {
    assert_eq!(&from_json::<T>(to_json(value)), value);
}

fn refreshed() -> core::Refreshed {
    core::Refreshed {
        snapshot: Arc::new(core::ScanSnapshot {
            revision: 9,
            scanned_at: (SystemTime::UNIX_EPOCH + Duration::from_secs(2)).into(),
            stats: core::ScanStats {
                dirs_read: 1,
                entries_seen: 0,
                duration: Duration::from_millis(1),
                containers_read: 0,
                containers_cached: 0,
                content_duration: Duration::ZERO,
            },
            content: core::ScanContent {
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

fn target() -> ModTarget {
    ModTarget {
        mod_ref: ROW_REF.into(),
        key: "Mods/X".into(),
    }
}

fn core_target() -> core::ModTarget {
    core::ModTarget {
        mod_ref: core::ModRef::from_token(ROW_REF),
        key: "Mods/X".into(),
    }
}

#[test]
fn delete_request_carries_token_and_target_into_core() {
    let json = json!({
        "target": { "modRef": ROW_REF, "key": "Mods/X" },
        "token": "0123abcd",
        "applyWhileRunning": true,
    });
    let request: DeleteRequest = from_json(json.clone());
    assert_eq!(to_json(&request), json);
    assert_eq!(
        core::DeleteRequest::from(&request),
        core::DeleteRequest {
            target: core_target(),
            token: "0123abcd".into(),
            apply_while_running: true,
        }
    );
}

#[test]
fn delete_prepared_size_is_a_number_or_null() {
    let mut prepared = core::DeletePrepared {
        token: "t".into(),
        key: "Mods/X".into(),
        name: "X".into(),
        size: Some(4096),
        expires_in_secs: 120,
    };
    let dto = DeletePrepared::from(&prepared);
    assert_eq!(
        to_json(&dto),
        json!({ "token": "t", "key": "Mods/X", "name": "X", "size": 4096.0, "expiresInSecs": 120 })
    );
    round_trips(&dto);

    prepared.size = None;
    assert_eq!(
        to_json(&DeletePrepared::from(&prepared))["size"],
        Value::Null
    );
}

#[test]
fn delete_outcome_converts_snapshot_and_leftover() {
    let dto = DeleteOutcome::from(&core::DeleteOutcome {
        refreshed: refreshed(),
        key: "Mods/X".into(),
        leftover: Some("Mods/~remmods-set_aside-1".into()),
    });
    let json = to_json(&dto);
    assert_eq!(json["snapshot"]["revision"], json!(9));
    assert_eq!(json["key"], json!("Mods/X"));
    assert_eq!(json["leftover"], json!("Mods/~remmods-set_aside-1"));
    round_trips(&dto);
}

#[test]
fn rename_request_and_outcome() {
    let json = json!({
        "target": { "modRef": ROW_REF, "key": "Mods/X" },
        "newName": "Neon Skin",
        "applyWhileRunning": false,
    });
    let request: RenameRequest = from_json(json.clone());
    assert_eq!(request.target, target());
    assert_eq!(to_json(&request), json);
    assert_eq!(
        core::RenameRequest::from(&request),
        core::RenameRequest {
            target: core_target(),
            new_name: "Neon Skin".into(),
            apply_while_running: false,
        }
    );

    let outcome = RenameOutcome::from(&core::RenameOutcome {
        refreshed: refreshed(),
        old_key: "Mods/X".into(),
        new_key: "Mods/Neon Skin".into(),
        new_ref: Some(core::ModRef::from_token("1|d|1-2|3-4|Mods/Neon Skin")),
        rekey: core::RekeyReport {
            profiles: 2,
            meta: true,
            skipped_ambiguous: false,
            failed: false,
        },
    });
    let json = to_json(&outcome);
    assert_eq!(json["oldKey"], json!("Mods/X"));
    assert_eq!(json["newKey"], json!("Mods/Neon Skin"));
    assert_eq!(json["newRef"], json!("1|d|1-2|3-4|Mods/Neon Skin"));
    assert_eq!(
        json["rekey"],
        json!({ "profiles": 2, "meta": true, "skippedAmbiguous": false, "failed": false })
    );
    assert_eq!(
        outcome.rekey,
        RekeyReport {
            profiles: 2,
            meta: true,
            skipped_ambiguous: false,
            failed: false,
        }
    );
    round_trips(&outcome);
}

#[test]
fn rename_outcome_without_new_ref_is_null() {
    let outcome = RenameOutcome::from(&core::RenameOutcome {
        refreshed: refreshed(),
        old_key: "a".into(),
        new_key: "b".into(),
        new_ref: None,
        rekey: core::RekeyReport::default(),
    });
    assert_eq!(to_json(&outcome)["newRef"], Value::Null);
}

#[test]
fn trash_request_and_outcome() {
    let json =
        json!({ "target": { "modRef": ROW_REF, "key": "Mods/X" }, "applyWhileRunning": true });
    let request: TrashRequest = from_json(json.clone());
    assert_eq!(to_json(&request), json);
    assert_eq!(
        core::TrashRequest::from(&request),
        core::TrashRequest {
            target: core_target(),
            apply_while_running: true,
        }
    );

    let outcome = TrashOutcome::from(&core::TrashOutcome {
        refreshed: refreshed(),
        key: "Mods/X".into(),
        items: 3,
        kind: platform::TrashKind::TopdirUser,
    });
    let json = to_json(&outcome);
    assert_eq!(json["kind"], json!("topdirUser"));
    assert_eq!(json["items"], json!(3));
    round_trips(&outcome);
}

#[test]
fn trash_kinds_are_camel_case() {
    let cases = [
        (platform::TrashKind::Home, TrashKind::Home, "home"),
        (
            platform::TrashKind::TopdirShared,
            TrashKind::TopdirShared,
            "topdirShared",
        ),
        (
            platform::TrashKind::TopdirUser,
            TrashKind::TopdirUser,
            "topdirUser",
        ),
        (
            platform::TrashKind::RecycleBin,
            TrashKind::RecycleBin,
            "recycleBin",
        ),
    ];
    for (core_kind, kind, name) in cases {
        assert_eq!(TrashKind::from(core_kind), kind);
        assert_eq!(to_json(&kind), json!(name));
    }
}

#[test]
fn metadata_request_converts_fields_verbatim() {
    let json = json!({
        "target": { "modRef": ROW_REF, "key": "Mods/X" },
        "info": { "version": "", "category": "Skin", "description": "a \"b\"", "author": "Rem" },
    });
    let request: MetadataRequest = from_json(json.clone());
    assert_eq!(to_json(&request), json);
    assert_eq!(
        core::MetadataRequest::from(&request),
        core::MetadataRequest {
            target: core_target(),
            info: core::ModInfoEdit {
                version: String::new(),
                category: "Skin".into(),
                description: "a \"b\"".into(),
                author: "Rem".into(),
            },
        }
    );
    round_trips(&ModInfoEdit {
        version: "1".into(),
        category: String::new(),
        description: String::new(),
        author: String::new(),
    });
}

#[test]
fn mod_target_converts_both_ways() {
    let dto = ModTarget::from(&core_target());
    assert_eq!(dto, target());
    assert_eq!(core::ModTarget::from(&dto), core_target());
    assert_eq!(to_json(&dto), json!({ "modRef": ROW_REF, "key": "Mods/X" }));
}

#[test]
fn manage_actions_and_blocks_are_camel_case() {
    let actions = [
        (core::ManageAction::Rename, ManageAction::Rename, "rename"),
        (core::ManageAction::Trash, ManageAction::Trash, "trash"),
        (core::ManageAction::Delete, ManageAction::Delete, "delete"),
        (
            core::ManageAction::Metadata,
            ManageAction::Metadata,
            "metadata",
        ),
    ];
    for (core_action, action, name) in actions {
        assert_eq!(ManageAction::from(core_action), action);
        assert_eq!(to_json(&action), json!(name));
    }
    let blocks = [
        (core::ManageBlock::Loose, ManageBlock::Loose, "loose"),
        (core::ManageBlock::Linked, ManageBlock::Linked, "linked"),
        (core::ManageBlock::Archive, ManageBlock::Archive, "archive"),
        (
            core::ManageBlock::Leftover,
            ManageBlock::Leftover,
            "leftover",
        ),
        (
            core::ManageBlock::Unmanaged,
            ManageBlock::Unmanaged,
            "unmanaged",
        ),
        (
            core::ManageBlock::OutsideRoots,
            ManageBlock::OutsideRoots,
            "outsideRoots",
        ),
        (
            core::ManageBlock::JournalPending,
            ManageBlock::JournalPending,
            "journalPending",
        ),
    ];
    for (core_block, block, name) in blocks {
        assert_eq!(ManageBlock::from(core_block), block);
        assert_eq!(to_json(&block), json!(name));
    }
}

#[test]
fn trash_unavailable_reasons_are_camel_case() {
    let cases = [
        (
            platform::TrashUnavailableReason::OtherDevice,
            TrashUnavailableReason::OtherDevice,
            "otherDevice",
        ),
        (
            platform::TrashUnavailableReason::NoUsableTrash,
            TrashUnavailableReason::NoUsableTrash,
            "noUsableTrash",
        ),
        (
            platform::TrashUnavailableReason::NotSupported,
            TrashUnavailableReason::NotSupported,
            "notSupported",
        ),
        (
            platform::TrashUnavailableReason::RecycleRefused,
            TrashUnavailableReason::RecycleRefused,
            "recycleRefused",
        ),
        (
            platform::TrashUnavailableReason::NotShellSafe,
            TrashUnavailableReason::NotShellSafe,
            "notShellSafe",
        ),
    ];
    for (core_reason, reason, name) in cases {
        assert_eq!(TrashUnavailableReason::from(core_reason), reason);
        assert_eq!(to_json(&reason), json!(name));
    }
}
