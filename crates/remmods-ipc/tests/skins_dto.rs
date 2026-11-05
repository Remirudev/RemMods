#![allow(clippy::expect_used)]

use remmods_core::CoreError;
use remmods_core::skins::{self as core, SkinsError};
use remmods_ipc::{AppError, SkinsView};
use serde_json::json;

fn view() -> core::SkinsView {
    core::SkinsView {
        account: Some("7656".to_owned()),
        available: true,
        sets: vec![core::SkinSetView {
            set: core::SkinSet {
                id: "a".repeat(32),
                name: "Fine".to_owned(),
                notes: "Reina 3".to_owned(),
                created: "2026-11-05T10:00:00.000Z".to_owned(),
                updated: "2026-11-05T11:00:00.000Z".to_owned(),
                screenshots: vec!["own-1.png".to_owned()],
                extra: serde_json::Map::new(),
            },
            bytes: 42,
        }],
        active: Some("a".repeat(32)),
        interrupted: None,
        read_only: false,
    }
}

#[test]
fn the_view_is_camel_case_and_counts_bytes_as_a_number() {
    let value = serde_json::to_value(SkinsView::from(&view())).expect("json");
    assert_eq!(value["readOnly"], json!(false));
    assert_eq!(value["sets"][0]["bytes"], json!(42.0));
    assert_eq!(value["sets"][0]["screenshots"], json!(["own-1.png"]));
    assert_eq!(value["interrupted"], json!(null));
}

#[test]
fn every_core_error_has_its_kind() {
    let cases = [
        (SkinsError::NoSaves, json!({ "kind": "skinsUnavailable" })),
        (SkinsError::GameRunning, json!({ "kind": "gameRunning" })),
        (SkinsError::UnknownSet, json!({ "kind": "stale" })),
        (SkinsError::NameTaken, json!({ "kind": "skinSetNameTaken" })),
        (
            SkinsError::InvalidName,
            json!({ "kind": "invalidInput", "field": "skinSetName" }),
        ),
        (
            SkinsError::NotesTooLong,
            json!({ "kind": "invalidInput", "field": "skinSetNotes" }),
        ),
        (SkinsError::SetInUse, json!({ "kind": "skinSetInUse" })),
        (SkinsError::BrokenSet, json!({ "kind": "skinSetBroken" })),
        (
            SkinsError::Interrupted,
            json!({ "kind": "skinSwitchInterrupted" }),
        ),
        (
            SkinsError::SwitchFailed { restored: true },
            json!({ "kind": "skinSwitchFailed", "restored": true }),
        ),
        (SkinsError::ReadOnly, json!({ "kind": "skinSetsReadOnly" })),
    ];
    for (error, expected) in cases {
        let value = serde_json::to_value(AppError::from(CoreError::Skins(error))).expect("json");
        assert_eq!(value, expected);
    }
}

#[test]
fn a_copy_names_the_set_it_came_from() {
    let copy = core::SkinCopy {
        id: "game-20261105".to_owned(),
        taken: "2026-11-05T11:00:00.000Z".to_owned(),
        set_name: Some("Fine".to_owned()),
    };

    assert_eq!(
        serde_json::to_value(remmods_ipc::SkinCopy::from(&copy)).expect("json"),
        json!({
            "id": "game-20261105",
            "taken": "2026-11-05T11:00:00.000Z",
            "setName": "Fine",
        })
    );
}
