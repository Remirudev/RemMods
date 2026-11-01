#![allow(clippy::expect_used)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use remmods_core::mods as core;
use remmods_ipc::{
    SkippedRow, ToggleBlock, ToggleOutcome, ToggleRequest, ToggleTarget, WatchState, WatchStatus,
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

fn empty_snapshot() -> core::ScanSnapshot {
    core::ScanSnapshot {
        revision: 7,
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
    }
}

#[test]
fn toggle_blocks_are_camel_case_and_map_from_core() {
    let cases = [
        (
            core::ToggleBlock::Ambiguous,
            ToggleBlock::Ambiguous,
            "ambiguous",
        ),
        (core::ToggleBlock::Linked, ToggleBlock::Linked, "linked"),
        (
            core::ToggleBlock::UnknownState,
            ToggleBlock::UnknownState,
            "unknownState",
        ),
        (
            core::ToggleBlock::NoPakFiles,
            ToggleBlock::NoPakFiles,
            "noPakFiles",
        ),
        (
            core::ToggleBlock::InvalidLogicName,
            ToggleBlock::InvalidLogicName,
            "invalidLogicName",
        ),
    ];
    for (core_block, expected, text) in cases {
        let block = ToggleBlock::from(core_block);
        assert_eq!(block, expected);
        assert_eq!(to_json(&block), json!(text));
    }
}

#[test]
fn toggle_request_round_trips_in_camel_case() {
    let request = ToggleRequest {
        targets: vec![ToggleTarget {
            mod_ref: ROW_REF.into(),
            key: "Standard:X".into(),
        }],
        enable: false,
        apply_while_running: true,
    };
    let expected = json!({
        "targets": [{ "modRef": ROW_REF, "key": "Standard:X" }],
        "enable": false,
        "applyWhileRunning": true
    });

    assert_eq!(to_json(&request), expected);
    assert_eq!(from_json::<ToggleRequest>(expected), request);
}

#[test]
fn toggle_request_converts_to_core() {
    let request = ToggleRequest {
        targets: vec![ToggleTarget {
            mod_ref: ROW_REF.into(),
            key: "Standard:X".into(),
        }],
        enable: true,
        apply_while_running: false,
    };

    let converted = core::ToggleRequest::from(&request);

    assert_eq!(
        converted,
        core::ToggleRequest {
            targets: vec![core::ToggleTarget {
                mod_ref: core::ModRef::from_token(ROW_REF),
                key: "Standard:X".into(),
            }],
            enable: true,
            apply_while_running: false,
        }
    );
}

#[test]
fn toggle_outcome_converts_and_serializes() {
    let outcome = core::ToggleOutcome {
        refreshed: core::Refreshed {
            snapshot: Arc::new(empty_snapshot()),
            changed: true,
        },
        skipped: vec![core::SkippedRow {
            key: "Standard:L".into(),
            name: "L".into(),
            reason: core::ToggleBlock::Linked,
        }],
        unmanaged: 2,
    };

    let dto = ToggleOutcome::from(&outcome);

    assert_eq!(dto.snapshot.revision, 7);
    assert_eq!(
        dto.skipped,
        vec![SkippedRow {
            key: "Standard:L".into(),
            name: "L".into(),
            reason: ToggleBlock::Linked,
        }]
    );
    assert_eq!(dto.unmanaged, 2);
    let value = to_json(&dto);
    assert_eq!(value["snapshot"]["revision"], json!(7));
    assert_eq!(
        value["skipped"],
        json!([{ "key": "Standard:L", "name": "L", "reason": "linked" }])
    );
    assert_eq!(value["unmanaged"], json!(2));
    assert_eq!(from_json::<ToggleOutcome>(value), dto);
}

#[test]
fn watch_status_is_camel_case() {
    let cases = [
        (WatchState::Active, "active"),
        (WatchState::LimitReached, "limitReached"),
        (WatchState::Unavailable, "unavailable"),
        (WatchState::Failed, "failed"),
    ];
    for (state, text) in cases {
        let status = WatchStatus { state };
        let expected = json!({ "state": text });
        assert_eq!(to_json(&status), expected);
        assert_eq!(from_json::<WatchStatus>(expected), status);
    }
}
