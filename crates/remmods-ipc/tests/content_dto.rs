#![allow(clippy::expect_used)]

use std::path::PathBuf;
use std::time::Duration;

use remmods_core::content as core_content;
use remmods_core::mods as core;
use remmods_ipc::{
    AssetEntry, AssetOrigin, BaseIndexState, CharacterHint, ContainerStatus, ContainerSummary,
    ContentConflictLink, ContentSummary, ModContents, ModRow, ScanStats, Slot, SlotHint,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

const ROW_REF: &str = "1|d|1-2|3-4|Mods/X";
const OTHER_REF: &str = "1|d|1-5|3-4|Mods/Y";

fn to_json<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialize")
}

fn from_json<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).expect("deserialize")
}

fn core_summary() -> core_content::ContentSummary {
    core_content::ContentSummary {
        containers: vec![
            core_content::ContainerSummary {
                rel_path: "A.utoc".into(),
                live: true,
                status: core_content::ContainerStatus::Read,
                version: Some(8),
                files: 3,
            },
            core_content::ContainerSummary {
                rel_path: "B.utoc-x".into(),
                live: false,
                status: core_content::ContainerStatus::Encrypted,
                version: None,
                files: 0,
            },
        ],
        assets: 3,
        characters: vec!["cmn".into(), "ker".into(), "zbr".into()],
        slots: vec!["bdu".into(), "zzz".into()],
        internal_overlaps: 1,
        shared_identical: 4,
    }
}

fn core_link() -> core_content::ContentConflictLink {
    core_content::ContentConflictLink {
        other: core::ModRef::from_token(OTHER_REF),
        key: "Standard:Y".into(),
        name: "Y".into(),
        shared: 2,
        sample: vec!["Game/A.uasset".into(), "Game/B.uasset".into()],
    }
}

fn core_row() -> core::ModRow {
    core::ModRow {
        mod_ref: core::ModRef::from_token(ROW_REF),
        key: "Standard:X".into(),
        kind: core::ModKind::Standard,
        name: "X".into(),
        folder_name: "X".into(),
        root: "Mods".into(),
        path: PathBuf::from("Paks").join("Mods").join("X"),
        state: core::ModState::Enabled,
        logic_prefixed: false,
        payload: core::PayloadCounts::default(),
        size: Some(1),
        info: core::ModInfo {
            version: "1.0".into(),
            category: "Misc".into(),
            description: String::new(),
            author: "Unknown".into(),
            origin: core::ModInfoOrigin::Missing,
            defaulted: core::ModInfoDefaulted::default(),
        },
        conflicts_with: vec![],
        content: core_summary(),
        content_conflicts: vec![core_link()],
        ambiguous: false,
        issues: vec![],
        toggle_block: None,
    }
}

fn core_entry(path: &str, container: &str) -> core_content::AssetEntry {
    core_content::AssetEntry {
        path: path.into(),
        container: container.into(),
        live: true,
        origin: core_content::AssetOrigin::Unknown,
        character: None,
        slot: None,
        shared_with: vec![],
    }
}

#[test]
fn status_tokens_are_camel_case() {
    let cases = [
        (
            core_content::ContainerStatus::Read,
            ContainerStatus::Read,
            "read",
        ),
        (
            core_content::ContainerStatus::NotIoStore,
            ContainerStatus::NotIoStore,
            "notIoStore",
        ),
        (
            core_content::ContainerStatus::UnsupportedVersion,
            ContainerStatus::UnsupportedVersion,
            "unsupportedVersion",
        ),
        (
            core_content::ContainerStatus::Encrypted,
            ContainerStatus::Encrypted,
            "encrypted",
        ),
        (
            core_content::ContainerStatus::NoIndex,
            ContainerStatus::NoIndex,
            "noIndex",
        ),
        (
            core_content::ContainerStatus::Malformed,
            ContainerStatus::Malformed,
            "malformed",
        ),
        (
            core_content::ContainerStatus::TooLarge,
            ContainerStatus::TooLarge,
            "tooLarge",
        ),
        (
            core_content::ContainerStatus::Unreadable,
            ContainerStatus::Unreadable,
            "unreadable",
        ),
    ];
    for (core_status, dto, token) in cases {
        assert_eq!(ContainerStatus::from(core_status), dto);
        assert_eq!(to_json(&dto), json!(token));
        assert_eq!(from_json::<ContainerStatus>(json!(token)), dto);
    }
}

#[test]
fn slot_tokens_are_camel_case() {
    let cases = [
        ("bdu", Slot::Upper, "upper"),
        ("bdl", Slot::Lower, "lower"),
        ("btm", Slot::Bottom, "bottom"),
        ("bdf", Slot::FullBody, "fullBody"),
        ("har", Slot::Hair, "hair"),
        ("hed", Slot::Head, "head"),
        ("hef", Slot::FullHead, "fullHead"),
        ("hat", Slot::Hat, "hat"),
        ("fac", Slot::Face, "face"),
        ("fap", Slot::FacePaint, "facePaint"),
        ("fah", Slot::FaceHair, "faceHair"),
        ("eym", Slot::EyeMakeup, "eyeMakeup"),
        ("eye", Slot::Eyes, "eyes"),
        ("chk", Slot::Cheek, "cheek"),
        ("lip", Slot::Lips, "lips"),
        ("gla", Slot::Glasses, "glasses"),
        ("acc", Slot::Accessory, "accessory"),
        ("sho", Slot::Shoes, "shoes"),
        ("exp", Slot::Extra, "extra"),
    ];
    for (code, slot, token) in cases {
        let hint = SlotHint::from_code(code);
        assert_eq!(hint.slot, Some(slot), "{code}");
        assert_eq!(to_json(&slot), json!(token));
        assert_eq!(from_json::<Slot>(json!(token)), slot);
    }
    assert_eq!(
        to_json(&SlotHint::from_code("zzz")),
        json!({ "code": "zzz", "slot": null })
    );
}

#[test]
fn origin_and_base_tokens() {
    let origins = [
        (core_content::AssetOrigin::Adds, AssetOrigin::Adds, "adds"),
        (
            core_content::AssetOrigin::Replaces,
            AssetOrigin::Replaces,
            "replaces",
        ),
        (
            core_content::AssetOrigin::Unknown,
            AssetOrigin::Unknown,
            "unknown",
        ),
    ];
    for (core_origin, dto, token) in origins {
        assert_eq!(AssetOrigin::from(core_origin), dto);
        assert_eq!(to_json(&dto), json!(token));
    }
    let states = [
        (
            core_content::BaseIndexState::Complete,
            BaseIndexState::Complete,
            "complete",
        ),
        (
            core_content::BaseIndexState::Partial,
            BaseIndexState::Partial,
            "partial",
        ),
        (
            core_content::BaseIndexState::Unavailable,
            BaseIndexState::Unavailable,
            "unavailable",
        ),
    ];
    for (core_state, dto, token) in states {
        assert_eq!(BaseIndexState::from(core_state), dto);
        assert_eq!(to_json(&dto), json!(token));
    }
}

#[test]
fn row_has_content_and_content_conflicts() {
    let dto = ModRow::from(&core_row());

    let value = to_json(&dto);

    assert_eq!(value["content"]["assets"], json!(3));
    assert_eq!(value["content"]["internalOverlaps"], json!(1));
    assert_eq!(value["content"]["sharedIdentical"], json!(4));
    assert_eq!(
        value["contentConflicts"],
        json!([{
            "modRef": OTHER_REF,
            "key": "Standard:Y",
            "name": "Y",
            "shared": 2,
            "sample": ["Game/A.uasset", "Game/B.uasset"]
        }])
    );
    assert_eq!(
        dto.content_conflicts,
        vec![ContentConflictLink {
            mod_ref: OTHER_REF.into(),
            key: "Standard:Y".into(),
            name: "Y".into(),
            shared: 2,
            sample: vec!["Game/A.uasset".into(), "Game/B.uasset".into()],
        }]
    );
    assert_eq!(from_json::<ModRow>(value), dto);
}

#[test]
fn scan_stats_has_container_counts() {
    let stats = core::ScanStats {
        dirs_read: 4,
        entries_seen: 9,
        duration: Duration::from_millis(3),
        containers_read: 5,
        containers_cached: 7,
        content_duration: Duration::from_millis(2),
    };

    let dto = ScanStats::from(stats);

    assert_eq!(dto.containers_read, 5);
    assert_eq!(dto.containers_cached, 7);
    assert_eq!(
        to_json(&dto),
        json!({
            "dirsRead": 4,
            "entriesSeen": 9,
            "durationMs": 3.0,
            "containersRead": 5,
            "containersCached": 7
        })
    );
}

#[test]
fn character_hint_names_only_confirmed() {
    assert_eq!(
        CharacterHint::from_code("ker"),
        CharacterHint {
            code: "ker".into(),
            name: Some("Kunimitsu".into()),
            common: false,
        }
    );
    assert_eq!(
        to_json(&CharacterHint::from_code("zbr")),
        json!({ "code": "zbr", "name": null, "common": false })
    );
    assert_eq!(
        CharacterHint::from_code("cmn"),
        CharacterHint {
            code: "cmn".into(),
            name: None,
            common: true,
        }
    );
}

#[test]
fn summary_mapping_keeps_counts() {
    let dto = ContentSummary::from(&core_summary());

    assert_eq!(
        dto.containers,
        vec![
            ContainerSummary {
                rel_path: "A.utoc".into(),
                live: true,
                status: ContainerStatus::Read,
                version: Some(8),
                files: 3,
            },
            ContainerSummary {
                rel_path: "B.utoc-x".into(),
                live: false,
                status: ContainerStatus::Encrypted,
                version: None,
                files: 0,
            },
        ]
    );
    assert_eq!(dto.assets, 3);
    assert_eq!(dto.internal_overlaps, 1);
    assert_eq!(dto.shared_identical, 4);
    let codes: Vec<_> = dto.characters.iter().map(|c| c.code.as_str()).collect();
    assert_eq!(codes, ["cmn", "ker", "zbr"]);
    assert_eq!(
        dto.slots,
        vec![
            SlotHint {
                code: "bdu".into(),
                slot: Some(Slot::Upper),
            },
            SlotHint {
                code: "zzz".into(),
                slot: None,
            },
        ]
    );
    assert_eq!(
        to_json(&dto.containers[0]),
        json!({ "relPath": "A.utoc", "live": true, "status": "read", "version": 8, "files": 3 })
    );
    assert_eq!(
        ContentSummary::from(&core_content::ContentSummary::default()),
        ContentSummary::default()
    );
}

#[test]
fn contents_mapping_keeps_order_and_truncation() {
    let mut first = core_entry("Game/A.uasset", "A.utoc");
    first.origin = core_content::AssetOrigin::Replaces;
    first.character = Some("ker".into());
    first.slot = Some("bdu".into());
    first.shared_with = vec![core::ModRef::from_token(OTHER_REF)];
    let mut second = core_entry("Game/A.uasset", "B.utoc-x");
    second.live = false;
    let third = core_entry("Game/C.uasset", "A.utoc");
    let contents = core_content::ModContents {
        mod_ref: core::ModRef::from_token(ROW_REF),
        base: core_content::BaseIndexState::Partial,
        entries: vec![first, second, third],
        total: 6000,
        truncated: true,
    };

    let dto = ModContents::from(&contents);

    assert_eq!(dto.mod_ref, ROW_REF);
    assert_eq!(dto.base, BaseIndexState::Partial);
    assert_eq!(dto.total, 6000);
    assert!(dto.truncated);
    let order: Vec<_> = dto
        .entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry.container.as_str()))
        .collect();
    assert_eq!(
        order,
        [
            ("Game/A.uasset", "A.utoc"),
            ("Game/A.uasset", "B.utoc-x"),
            ("Game/C.uasset", "A.utoc"),
        ]
    );
    assert_eq!(
        dto.entries[0],
        AssetEntry {
            path: "Game/A.uasset".into(),
            container: "A.utoc".into(),
            live: true,
            origin: AssetOrigin::Replaces,
            character: Some(CharacterHint {
                code: "ker".into(),
                name: Some("Kunimitsu".into()),
                common: false,
            }),
            slot: Some(SlotHint {
                code: "bdu".into(),
                slot: Some(Slot::Upper),
            }),
            shared_with: vec![OTHER_REF.into()],
        }
    );
    let value = to_json(&dto);
    assert_eq!(value["base"], json!("partial"));
    assert_eq!(value["entries"][0]["sharedWith"], json!([OTHER_REF]));
    assert_eq!(value["entries"][1]["character"], json!(null));
    assert_eq!(from_json::<ModContents>(value), dto);
}
