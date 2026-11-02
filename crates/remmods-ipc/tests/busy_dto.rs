#![allow(clippy::expect_used)]

use std::time::Duration;

use remmods_core::safe_fs as core;
use remmods_ipc::{BusyPhase, FolderBusyInfo};
use serde_json::{Value, json};

fn event(phase: core::BusyPhase, holders: Option<Vec<String>>) -> core::BusyEvent {
    core::BusyEvent {
        wait_id: 3,
        phase,
        folder: "LogicMods/Tool".into(),
        holders,
        elapsed: Duration::from_micros(1_234_999),
        limit: Duration::from_secs(30),
        cancellable: true,
    }
}

#[test]
fn folder_busy_info_from_event() {
    let info = FolderBusyInfo::from(&event(core::BusyPhase::Waiting, None));

    assert_eq!(
        info,
        FolderBusyInfo {
            wait_id: 3,
            phase: BusyPhase::Waiting,
            folder: "LogicMods/Tool".into(),
            holders: vec![],
            elapsed_ms: 1234.0,
            limit_ms: 30_000.0,
            cancellable: true,
        }
    );
    let json = serde_json::to_value(&info).expect("serialize");
    assert_eq!(
        json,
        json!({
            "waitId": 3,
            "phase": "waiting",
            "folder": "LogicMods/Tool",
            "holders": [],
            "elapsedMs": 1234.0,
            "limitMs": 30000.0,
            "cancellable": true,
        })
    );
    let parsed: FolderBusyInfo = serde_json::from_value(json).expect("deserialize");
    assert_eq!(parsed, info);
}

#[test]
fn folder_busy_keeps_holders_when_known() {
    let info = FolderBusyInfo::from(&event(
        core::BusyPhase::Done,
        Some(vec!["explorer.exe".into()]),
    ));

    assert_eq!(info.holders, vec!["explorer.exe".to_owned()]);
}

#[test]
fn busy_phases_are_camel_case() {
    let cases = [
        (core::BusyPhase::Waiting, BusyPhase::Waiting, "waiting"),
        (core::BusyPhase::Done, BusyPhase::Done, "done"),
        (core::BusyPhase::TimedOut, BusyPhase::TimedOut, "timedOut"),
        (
            core::BusyPhase::Cancelled,
            BusyPhase::Cancelled,
            "cancelled",
        ),
        (core::BusyPhase::Failed, BusyPhase::Failed, "failed"),
    ];
    for (core_phase, phase, name) in cases {
        assert_eq!(BusyPhase::from(core_phase), phase);
        assert_eq!(
            serde_json::to_value(phase).expect("serialize"),
            Value::String(name.into())
        );
        let info = FolderBusyInfo::from(&event(core_phase, None));
        assert_eq!(info.phase, phase);
    }
}
