#![allow(clippy::expect_used)]

use remmods_core::mods as core;
use remmods_ipc::{ModTarget, RepairAction, RepairItem, RepairKind, RepairList};
use serde_json::{Value, json};

fn to_json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialize")
}

#[test]
fn repair_list_converts_items_with_and_without_target() {
    let list = core::RepairList {
        revision: 4,
        items: vec![
            core::RepairItem {
                id: "invalidName|1|d|1-2|3-4|Mods/Bad.".into(),
                kind: core::RepairKind::InvalidName,
                target: Some(core::ModTarget {
                    mod_ref: core::ModRef::from_token("1|d|1-2|3-4|Mods/Bad."),
                    key: "Mods/Bad.".into(),
                }),
                label: "Bad.".into(),
                actions: vec![core::RepairAction::Rename, core::RepairAction::Reveal],
                suggested_name: Some("Bad".into()),
            },
            core::RepairItem {
                id: "stuck".into(),
                kind: core::RepairKind::StuckJournal,
                target: None,
                label: "2".into(),
                actions: vec![core::RepairAction::OpenLogs],
                suggested_name: None,
            },
        ],
    };

    let dto = RepairList::from(&list);

    assert_eq!(dto.revision, 4);
    assert_eq!(
        dto.items[0],
        RepairItem {
            id: "invalidName|1|d|1-2|3-4|Mods/Bad.".into(),
            kind: RepairKind::InvalidName,
            target: Some(ModTarget {
                mod_ref: "1|d|1-2|3-4|Mods/Bad.".into(),
                key: "Mods/Bad.".into(),
            }),
            label: "Bad.".into(),
            actions: vec![RepairAction::Rename, RepairAction::Reveal],
            suggested_name: Some("Bad".into()),
        }
    );
    assert_eq!(
        to_json(&dto.items[1]),
        json!({
            "id": "stuck",
            "kind": "stuckJournal",
            "target": null,
            "label": "2",
            "actions": ["openLogs"],
            "suggestedName": null,
        })
    );
    let back: RepairList = serde_json::from_value(to_json(&dto)).expect("deserialize");
    assert_eq!(back, dto);
}

#[test]
fn repair_kinds_are_camel_case() {
    let cases = [
        (core::RepairKind::Leftover, "leftover"),
        (core::RepairKind::AmbiguousKey, "ambiguousKey"),
        (core::RepairKind::InvalidName, "invalidName"),
        (core::RepairKind::NonUnicodeName, "nonUnicodeName"),
        (
            core::RepairKind::LogicDisabledButLoaded,
            "logicDisabledButLoaded",
        ),
        (
            core::RepairKind::LogicEnabledNotLoaded,
            "logicEnabledNotLoaded",
        ),
        (core::RepairKind::TempFolderLive, "tempFolderLive"),
        (core::RepairKind::StuckJournal, "stuckJournal"),
    ];
    for (kind, name) in cases {
        assert_eq!(to_json(&RepairKind::from(kind)), json!(name));
    }
}

#[test]
fn repair_actions_are_camel_case() {
    let cases = [
        (core::RepairAction::Rename, "rename"),
        (core::RepairAction::Trash, "trash"),
        (core::RepairAction::DeletePermanently, "deletePermanently"),
        (core::RepairAction::Reveal, "reveal"),
        (core::RepairAction::FinishDisable, "finishDisable"),
        (core::RepairAction::FinishEnable, "finishEnable"),
        (core::RepairAction::OpenLogs, "openLogs"),
    ];
    for (action, name) in cases {
        assert_eq!(to_json(&RepairAction::from(action)), json!(name));
    }
}
