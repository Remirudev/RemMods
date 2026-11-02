#![allow(clippy::expect_used)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use remmods_core::CoreError;
use remmods_core::mods::{self as core, ManageAction as CoreAction, ManageBlock as CoreBlock};
use remmods_core::platform;
use remmods_core::safe_fs::{BatchError, FsError};
use remmods_ipc::{
    AppError, BulkDeletePrepared, BulkDeleteRequest, BulkOutcome, BulkRow, BulkRowResult,
    BulkTrashRequest, ManageAction, ManageBlock, ModTarget, OsErrorClass, PreparedRow, TrashKind,
    TrashUnavailableReason,
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

fn row_ref(key: &str) -> String {
    format!("1|d|1-2|3-4|{key}")
}

fn target(key: &str) -> ModTarget {
    ModTarget {
        mod_ref: row_ref(key),
        key: key.into(),
    }
}

fn core_target(key: &str) -> core::ModTarget {
    core::ModTarget {
        mod_ref: core::ModRef::from_token(row_ref(key)),
        key: key.into(),
    }
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

fn cancelled_batch(stuck: usize) -> CoreError {
    let stuck = (0..stuck)
        .map(|index| remmods_core::safe_fs::StuckOp {
            from: format!("a{index}").into(),
            to: format!("b{index}").into(),
        })
        .collect();
    CoreError::Fs(FsError::Batch(Box::new(BatchError {
        failed: 1,
        cause: FsError::BusyCancelled {
            path: "Paks/Mods/B".into(),
        },
        rolled_back: 1,
        stuck,
    })))
}

fn core_row(key: &str, result: core::BulkRowResult) -> core::BulkRow {
    core::BulkRow {
        target: core_target(key),
        name: key.rsplit('/').next().unwrap_or(key).to_owned(),
        result,
    }
}

#[test]
fn bulk_row_result_json_shapes() {
    let cases = [
        (
            BulkRowResult::Trashed {
                items: 2,
                trash: TrashKind::RecycleBin,
            },
            json!({ "kind": "trashed", "items": 2, "trash": "recycleBin" }),
        ),
        (
            BulkRowResult::Deleted { leftover: None },
            json!({ "kind": "deleted", "leftover": null }),
        ),
        (
            BulkRowResult::Deleted {
                leftover: Some("Mods/_A_tmp_1".into()),
            },
            json!({ "kind": "deleted", "leftover": "Mods/_A_tmp_1" }),
        ),
        (
            BulkRowResult::Unavailable {
                reason: TrashUnavailableReason::RecycleRefused,
                size: Some(1024.0),
            },
            json!({ "kind": "unavailable", "reason": "recycleRefused", "size": 1024.0 }),
        ),
        (
            BulkRowResult::Unavailable {
                reason: TrashUnavailableReason::OtherDevice,
                size: None,
            },
            json!({ "kind": "unavailable", "reason": "otherDevice", "size": null }),
        ),
        (
            BulkRowResult::Failed {
                error: AppError::Stale,
            },
            json!({ "kind": "failed", "error": { "kind": "stale" } }),
        ),
        (BulkRowResult::NotRun, json!({ "kind": "notRun" })),
    ];
    for (result, expected) in cases {
        assert_eq!(to_json(&result), expected);
        assert_eq!(from_json::<BulkRowResult>(expected), result);
    }
}

#[test]
fn bulk_requests_carry_targets_in_order_into_core() {
    let trash: BulkTrashRequest = from_json(json!({
        "targets": [
            { "modRef": row_ref("Mods/B"), "key": "Mods/B" },
            { "modRef": row_ref("Mods/A"), "key": "Mods/A" },
        ],
        "applyWhileRunning": true,
    }));
    assert_eq!(
        core::BulkTrashRequest::from(&trash),
        core::BulkTrashRequest {
            targets: vec![core_target("Mods/B"), core_target("Mods/A")],
            apply_while_running: true,
        }
    );

    let delete: BulkDeleteRequest = from_json(json!({
        "targets": [{ "modRef": row_ref("Mods/A"), "key": "Mods/A" }],
        "token": "0123abcd",
        "applyWhileRunning": false,
    }));
    assert_eq!(
        core::BulkDeleteRequest::from(&delete),
        core::BulkDeleteRequest {
            targets: vec![core_target("Mods/A")],
            token: "0123abcd".into(),
            apply_while_running: false,
        }
    );
}

#[test]
fn bulk_outcome_from_core_keeps_order_and_errors() {
    let core_outcome = core::BulkOutcome {
        refreshed: Some(refreshed()),
        rows: vec![
            core_row(
                "Mods/A",
                core::BulkRowResult::Trashed {
                    items: 1,
                    kind: platform::TrashKind::Home,
                },
            ),
            core_row("Mods/B", core::BulkRowResult::Deleted { leftover: None }),
            core_row(
                "Mods/C",
                core::BulkRowResult::TrashUnavailable {
                    reason: platform::TrashUnavailableReason::NotShellSafe,
                    size: Some(77),
                },
            ),
            core_row(
                "Mods/D",
                core::BulkRowResult::Failed(CoreError::Mods(core::ModsError::Stale)),
            ),
            core_row("Mods/E", core::BulkRowResult::Failed(cancelled_batch(0))),
            core_row("Mods/F", core::BulkRowResult::Failed(cancelled_batch(1))),
            core_row("Mods/G", core::BulkRowResult::NotRun),
        ],
    };

    let outcome = BulkOutcome::from(core_outcome);

    let expected = [
        (
            "Mods/A",
            BulkRowResult::Trashed {
                items: 1,
                trash: TrashKind::Home,
            },
        ),
        ("Mods/B", BulkRowResult::Deleted { leftover: None }),
        (
            "Mods/C",
            BulkRowResult::Unavailable {
                reason: TrashUnavailableReason::NotShellSafe,
                size: Some(77.0),
            },
        ),
        (
            "Mods/D",
            BulkRowResult::Failed {
                error: AppError::Stale,
            },
        ),
        (
            "Mods/E",
            BulkRowResult::Failed {
                error: AppError::BusyCancelled,
            },
        ),
        (
            "Mods/F",
            BulkRowResult::Failed {
                error: AppError::BatchFailed {
                    class: OsErrorClass::InUse,
                    stuck: 1,
                    folder: true,
                },
            },
        ),
        ("Mods/G", BulkRowResult::NotRun),
    ];
    assert_eq!(outcome.rows.len(), expected.len());
    for (row, (key, result)) in outcome.rows.iter().zip(expected) {
        assert_eq!(
            row,
            &BulkRow {
                target: target(key),
                name: key.rsplit('/').next().unwrap_or(key).to_owned(),
                result,
            }
        );
    }
    assert_eq!(
        outcome.snapshot.as_ref().map(|snapshot| snapshot.revision),
        Some(9)
    );
    let json = to_json(&outcome);
    assert_eq!(
        json["rows"][4]["result"],
        json!({ "kind": "failed", "error": { "kind": "busyCancelled" } })
    );
    assert_eq!(from_json::<BulkOutcome>(json), outcome);
}

#[test]
fn bulk_outcome_without_refresh_has_null_snapshot() {
    let outcome = BulkOutcome::from(core::BulkOutcome {
        refreshed: None,
        rows: vec![core_row(
            "Mods/A",
            core::BulkRowResult::Deleted {
                leftover: Some("Mods/_A_tmp_1".into()),
            },
        )],
    });

    let json = to_json(&outcome);

    assert_eq!(json["snapshot"], Value::Null);
    assert_eq!(
        json["rows"][0]["result"],
        json!({ "kind": "deleted", "leftover": "Mods/_A_tmp_1" })
    );
}

#[test]
fn prepared_from_core() {
    let prepared = BulkDeletePrepared::from(core::BulkDeletePrepared {
        token: Some("feedc0de".into()),
        rows: vec![
            core::PreparedRow {
                target: core_target("Mods/A"),
                name: "A".into(),
                size: Some(4096),
                refusal: None,
            },
            core::PreparedRow {
                target: core_target("Mods/B"),
                name: "Mods/B".into(),
                size: None,
                refusal: Some(core::ModsError::Stale),
            },
            core::PreparedRow {
                target: core_target("~mods/C_P.pak"),
                name: "C_P".into(),
                size: Some(12),
                refusal: Some(core::ModsError::ManageBlocked {
                    key: "~mods/C_P.pak".into(),
                    action: CoreAction::Delete,
                    reason: CoreBlock::Loose,
                }),
            },
        ],
        expires_in_secs: 120,
    });

    assert_eq!(
        prepared,
        BulkDeletePrepared {
            token: Some("feedc0de".into()),
            rows: vec![
                PreparedRow {
                    target: target("Mods/A"),
                    name: "A".into(),
                    size: Some(4096.0),
                    refusal: None,
                },
                PreparedRow {
                    target: target("Mods/B"),
                    name: "Mods/B".into(),
                    size: None,
                    refusal: Some(AppError::Stale),
                },
                PreparedRow {
                    target: target("~mods/C_P.pak"),
                    name: "C_P".into(),
                    size: Some(12.0),
                    refusal: Some(AppError::ManageBlocked {
                        action: ManageAction::Delete,
                        reason: ManageBlock::Loose,
                    }),
                },
            ],
            expires_in_secs: 120,
        }
    );
    let json = to_json(&prepared);
    assert_eq!(json["expiresInSecs"], json!(120));
    assert_eq!(json["rows"][0]["size"], json!(4096.0));
    assert_eq!(json["rows"][1]["refusal"], json!({ "kind": "stale" }));
    assert_eq!(from_json::<BulkDeletePrepared>(json), prepared);

    let none = BulkDeletePrepared::from(core::BulkDeletePrepared {
        token: None,
        rows: vec![],
        expires_in_secs: 120,
    });
    assert_eq!(to_json(&none)["token"], Value::Null);
}
