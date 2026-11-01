#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use remmods_core::mods as core;
use remmods_core::paths::display_path;
use remmods_core::platform;
use remmods_ipc::{
    ArchiveEntry, ConflictLink, LeftoverEntry, ModDetails, ModInfo, ModInfoDefaulted,
    ModInfoOrigin, ModIssue, ModKind, ModRow, ModState, ModsSnapshot, NameProblem, OsErrorClass,
    PayloadCounts, PayloadFile, PayloadKind, RootInfo, RootKind, ScanProblem, ScanStats,
    UnmanagedEntry, UnmanagedKind,
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

fn paks() -> PathBuf {
    Path::new("games").join("TEKKEN 8").join("Paks")
}

fn default_info() -> core::ModInfo {
    core::ModInfo {
        version: "1.0".into(),
        category: "Misc".into(),
        description: String::new(),
        author: "Unknown".into(),
        origin: core::ModInfoOrigin::Missing,
        defaulted: core::ModInfoDefaulted {
            version: true,
            category: true,
            description: true,
            author: true,
        },
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
        path: paks().join("Mods").join("X"),
        state: core::ModState::Partial,
        logic_prefixed: false,
        payload: core::PayloadCounts {
            live_pak: 2,
            off_pak: 1,
            live_companion: 4,
            off_companion: 3,
        },
        size: Some(10_000_000_000),
        info: core::ModInfo {
            version: "2.0".into(),
            category: "Character".into(),
            description: "Smoke fixture".into(),
            author: "RemMods".into(),
            origin: core::ModInfoOrigin::File,
            defaulted: core::ModInfoDefaulted::default(),
        },
        conflicts_with: vec![core::ConflictLink {
            other: core::ModRef::from_token(OTHER_REF),
            key: "Standard:Y".into(),
            name: "Y".into(),
            paks: vec!["Shared.pak".into()],
        }],
        ambiguous: true,
        issues: vec![core::ModIssue::AmbiguousKey, core::ModIssue::ContainsLink],
    }
}

fn core_snapshot() -> core::ScanSnapshot {
    let paks = paks();
    core::ScanSnapshot {
        revision: 3,
        scanned_at: (SystemTime::UNIX_EPOCH + Duration::from_millis(1_500)).into(),
        stats: core::ScanStats {
            dirs_read: 12,
            entries_seen: 40,
            duration: Duration::from_micros(2_500),
        },
        content: core::ScanContent {
            paks: paks.clone(),
            paks_id: "3-4".into(),
            roots: vec![core::RootInfo {
                kind: core::RootKind::Legacy,
                name: "~mods".into(),
                path: paks.join("~mods"),
                is_link: true,
                case_variant: false,
                unreadable: Some(core::ScanProblem {
                    op: "list",
                    class: platform::OsErrorClass::AccessDenied,
                }),
            }],
            rows: vec![core_row()],
            archives: vec![core::ArchiveEntry {
                mod_ref: core::ModRef::from_token("1|f|1-9|3-4|Mods/Pack.zip"),
                rel_path: "Mods/Pack.zip".into(),
                size: 4,
            }],
            unmanaged: vec![core::UnmanagedEntry {
                mod_ref: core::ModRef::from_token("1|f|1-7|3-4|Stray.pak"),
                kind: core::UnmanagedKind::PaksFile,
                rel_path: "Stray.pak".into(),
                live_files: 1,
                copy_of: vec!["X".into()],
            }],
            leftovers: vec![core::LeftoverEntry {
                mod_ref: core::ModRef::from_token("1|d|-|3-4|Mods/_Old_tmp_0"),
                rel_path: "Mods/_Old_tmp_0".into(),
            }],
        },
    }
}

#[test]
fn unit_enums_are_camel_case_strings() {
    let kinds = [
        (ModKind::Standard, "standard"),
        (ModKind::Legacy, "legacy"),
        (ModKind::Logic, "logic"),
        (ModKind::Loose, "loose"),
    ];
    for (kind, expected) in kinds {
        assert_eq!(to_json(&kind), json!(expected));
    }
    let states = [
        (ModState::Enabled, "enabled"),
        (ModState::Partial, "partial"),
        (ModState::Disabled, "disabled"),
        (ModState::NoPakFiles, "noPakFiles"),
        (ModState::Unknown, "unknown"),
    ];
    for (state, expected) in states {
        assert_eq!(to_json(&state), json!(expected));
        assert_eq!(from_json::<ModState>(json!(expected)), state);
    }
    let roots = [
        (RootKind::Standard, "standard"),
        (RootKind::Legacy, "legacy"),
        (RootKind::Logic, "logic"),
    ];
    for (root, expected) in roots {
        assert_eq!(to_json(&root), json!(expected));
    }
    let origins = [
        (ModInfoOrigin::File, "file"),
        (ModInfoOrigin::Missing, "missing"),
        (ModInfoOrigin::Invalid, "invalid"),
    ];
    for (origin, expected) in origins {
        assert_eq!(to_json(&origin), json!(expected));
    }
    let unmanaged = [
        (UnmanagedKind::PaksFile, "paksFile"),
        (UnmanagedKind::Folder, "folder"),
        (UnmanagedKind::LinkedFolder, "linkedFolder"),
        (UnmanagedKind::TempFolder, "tempFolder"),
    ];
    for (kind, expected) in unmanaged {
        assert_eq!(to_json(&kind), json!(expected));
    }
    let payloads = [
        (PayloadKind::Pak, "pak"),
        (PayloadKind::Ucas, "ucas"),
        (PayloadKind::Utoc, "utoc"),
        (PayloadKind::Sig, "sig"),
    ];
    for (kind, expected) in payloads {
        assert_eq!(to_json(&kind), json!(expected));
    }
}

#[test]
fn name_problems_are_camel_case_and_map_from_core() {
    let cases = [
        (core::NameProblem::Empty, NameProblem::Empty, "empty"),
        (core::NameProblem::DotName, NameProblem::DotName, "dotName"),
        (core::NameProblem::TooLong, NameProblem::TooLong, "tooLong"),
        (
            core::NameProblem::ForbiddenChar,
            NameProblem::ForbiddenChar,
            "forbiddenChar",
        ),
        (
            core::NameProblem::ControlChar,
            NameProblem::ControlChar,
            "controlChar",
        ),
        (
            core::NameProblem::BidiControl,
            NameProblem::BidiControl,
            "bidiControl",
        ),
        (
            core::NameProblem::TrailingDotOrSpace,
            NameProblem::TrailingDotOrSpace,
            "trailingDotOrSpace",
        ),
        (
            core::NameProblem::ReservedDeviceName,
            NameProblem::ReservedDeviceName,
            "reservedDeviceName",
        ),
        (
            core::NameProblem::DisabledPrefix,
            NameProblem::DisabledPrefix,
            "disabledPrefix",
        ),
        (
            core::NameProblem::TempName,
            NameProblem::TempName,
            "tempName",
        ),
    ];
    for (core_problem, problem, expected) in cases {
        assert_eq!(NameProblem::from(core_problem), problem);
        assert_eq!(to_json(&problem), json!(expected));
    }
}

#[test]
fn every_mod_issue_is_tagged_by_kind() {
    let problem = ScanProblem {
        op: "read".into(),
        class: OsErrorClass::AccessDenied,
    };
    let cases = [
        (ModIssue::AmbiguousKey, json!({ "kind": "ambiguousKey" })),
        (
            ModIssue::CaseCollision {
                others: vec!["cool".into(), "COOL".into()],
            },
            json!({ "kind": "caseCollision", "others": ["cool", "COOL"] }),
        ),
        (
            ModIssue::CaseVariantRoot,
            json!({ "kind": "caseVariantRoot" }),
        ),
        (
            ModIssue::NonUnicodeName,
            json!({ "kind": "nonUnicodeName" }),
        ),
        (
            ModIssue::InvalidName {
                problem: NameProblem::ReservedDeviceName,
            },
            json!({ "kind": "invalidName", "problem": "reservedDeviceName" }),
        ),
        (
            ModIssue::NameNotPortable {
                problem: NameProblem::TooLong,
            },
            json!({ "kind": "nameNotPortable", "problem": "tooLong" }),
        ),
        (
            ModIssue::DisabledPrefixName,
            json!({ "kind": "disabledPrefixName" }),
        ),
        (ModIssue::EmptyName, json!({ "kind": "emptyName" })),
        (
            ModIssue::LogicDisabledButLoaded,
            json!({ "kind": "logicDisabledButLoaded" }),
        ),
        (
            ModIssue::LogicEnabledNotLoaded,
            json!({ "kind": "logicEnabledNotLoaded" }),
        ),
        (
            ModIssue::ModinfoInvalid,
            json!({ "kind": "modinfoInvalid" }),
        ),
        (
            ModIssue::ModinfoCaseVariants,
            json!({ "kind": "modinfoCaseVariants" }),
        ),
        (
            ModIssue::PathTooLong { longest: 262 },
            json!({ "kind": "pathTooLong", "longest": 262 }),
        ),
        (ModIssue::LinkedRoot, json!({ "kind": "linkedRoot" })),
        (ModIssue::LinkedMod, json!({ "kind": "linkedMod" })),
        (ModIssue::ContainsLink, json!({ "kind": "containsLink" })),
        (
            ModIssue::LooseCopy {
                files: vec!["cool.pak".into()],
            },
            json!({ "kind": "looseCopy", "files": ["cool.pak"] }),
        ),
        (
            ModIssue::LooseCaseVariants,
            json!({ "kind": "looseCaseVariants" }),
        ),
        (
            ModIssue::NonUnicodeFiles { count: 2 },
            json!({ "kind": "nonUnicodeFiles", "count": 2 }),
        ),
        (
            ModIssue::InvalidFileNames { count: 3 },
            json!({ "kind": "invalidFileNames", "count": 3 }),
        ),
        (
            ModIssue::NonPortableFileNames { count: 4 },
            json!({ "kind": "nonPortableFileNames", "count": 4 }),
        ),
        (
            ModIssue::Unreadable { problem },
            json!({ "kind": "unreadable", "problem": { "op": "read", "class": "accessDenied" } }),
        ),
    ];
    for (issue, expected) in cases {
        assert_eq!(to_json(&issue), expected);
        assert_eq!(from_json::<ModIssue>(expected), issue);
    }
}

#[test]
fn every_core_issue_converts() {
    let cases = [
        (core::ModIssue::AmbiguousKey, ModIssue::AmbiguousKey),
        (
            core::ModIssue::CaseCollision {
                others: vec!["a".into()],
            },
            ModIssue::CaseCollision {
                others: vec!["a".into()],
            },
        ),
        (core::ModIssue::CaseVariantRoot, ModIssue::CaseVariantRoot),
        (core::ModIssue::NonUnicodeName, ModIssue::NonUnicodeName),
        (
            core::ModIssue::InvalidName {
                problem: core::NameProblem::ForbiddenChar,
            },
            ModIssue::InvalidName {
                problem: NameProblem::ForbiddenChar,
            },
        ),
        (
            core::ModIssue::NameNotPortable {
                problem: core::NameProblem::BidiControl,
            },
            ModIssue::NameNotPortable {
                problem: NameProblem::BidiControl,
            },
        ),
        (
            core::ModIssue::DisabledPrefixName,
            ModIssue::DisabledPrefixName,
        ),
        (core::ModIssue::EmptyName, ModIssue::EmptyName),
        (
            core::ModIssue::LogicDisabledButLoaded,
            ModIssue::LogicDisabledButLoaded,
        ),
        (
            core::ModIssue::LogicEnabledNotLoaded,
            ModIssue::LogicEnabledNotLoaded,
        ),
        (core::ModIssue::ModinfoInvalid, ModIssue::ModinfoInvalid),
        (
            core::ModIssue::ModinfoCaseVariants,
            ModIssue::ModinfoCaseVariants,
        ),
        (
            core::ModIssue::PathTooLong { longest: 300 },
            ModIssue::PathTooLong { longest: 300 },
        ),
        (core::ModIssue::LinkedRoot, ModIssue::LinkedRoot),
        (core::ModIssue::LinkedMod, ModIssue::LinkedMod),
        (core::ModIssue::ContainsLink, ModIssue::ContainsLink),
        (
            core::ModIssue::LooseCopy {
                files: vec!["x.pak".into()],
            },
            ModIssue::LooseCopy {
                files: vec!["x.pak".into()],
            },
        ),
        (
            core::ModIssue::LooseCaseVariants,
            ModIssue::LooseCaseVariants,
        ),
        (
            core::ModIssue::NonUnicodeFiles { count: 1 },
            ModIssue::NonUnicodeFiles { count: 1 },
        ),
        (
            core::ModIssue::InvalidFileNames { count: 2 },
            ModIssue::InvalidFileNames { count: 2 },
        ),
        (
            core::ModIssue::NonPortableFileNames { count: 3 },
            ModIssue::NonPortableFileNames { count: 3 },
        ),
        (
            core::ModIssue::Unreadable {
                problem: core::ScanProblem {
                    op: "stat",
                    class: platform::OsErrorClass::NotFound,
                },
            },
            ModIssue::Unreadable {
                problem: ScanProblem {
                    op: "stat".into(),
                    class: OsErrorClass::NotFound,
                },
            },
        ),
    ];
    for (core_issue, expected) in cases {
        assert_eq!(ModIssue::from(&core_issue), expected);
    }
}

#[test]
fn mod_row_shape() {
    let row = ModRow {
        mod_ref: ROW_REF.into(),
        key: "Logic:Tool".into(),
        kind: ModKind::Logic,
        name: "Tool".into(),
        folder_name: ".disabled_Tool".into(),
        root: "LogicMods".into(),
        location: "/p/LogicMods/.disabled_Tool".into(),
        state: ModState::Disabled,
        logic_prefixed: true,
        payload: PayloadCounts {
            live_pak: 1,
            off_pak: 0,
            live_companion: 2,
            off_companion: 0,
        },
        size_bytes: None,
        info: ModInfo::from(&default_info()),
        conflicts_with: vec![],
        ambiguous: false,
        issues: vec![ModIssue::LogicDisabledButLoaded],
    };

    let expected = json!({
        "modRef": ROW_REF,
        "key": "Logic:Tool",
        "kind": "logic",
        "name": "Tool",
        "folderName": ".disabled_Tool",
        "root": "LogicMods",
        "location": "/p/LogicMods/.disabled_Tool",
        "state": "disabled",
        "logicPrefixed": true,
        "payload": { "livePak": 1, "offPak": 0, "liveCompanion": 2, "offCompanion": 0 },
        "sizeBytes": null,
        "info": {
            "version": "1.0",
            "category": "Misc",
            "description": "",
            "author": "Unknown",
            "origin": "missing",
            "defaulted": { "version": true, "category": true, "description": true, "author": true }
        },
        "conflictsWith": [],
        "ambiguous": false,
        "issues": [{ "kind": "logicDisabledButLoaded" }]
    });
    assert_eq!(to_json(&row), expected);
    assert_eq!(from_json::<ModRow>(expected), row);
}

#[test]
fn snapshot_converts_from_core() {
    let snapshot = core_snapshot();

    let dto = ModsSnapshot::from(&snapshot);

    assert_eq!(dto.revision, 3);
    assert_eq!(dto.scanned_at, "1970-01-01T00:00:01.500Z");
    assert_eq!(dto.paks, display_path(&paks()));
    assert_eq!(
        dto.roots,
        vec![RootInfo {
            kind: RootKind::Legacy,
            name: "~mods".into(),
            is_link: true,
            case_variant: false,
            unreadable: Some(ScanProblem {
                op: "list".into(),
                class: OsErrorClass::AccessDenied,
            }),
        }]
    );
    assert_eq!(
        dto.archives,
        vec![ArchiveEntry {
            mod_ref: "1|f|1-9|3-4|Mods/Pack.zip".into(),
            rel_path: "Mods/Pack.zip".into(),
            size_bytes: 4.0,
        }]
    );
    assert_eq!(
        dto.unmanaged,
        vec![UnmanagedEntry {
            mod_ref: "1|f|1-7|3-4|Stray.pak".into(),
            kind: UnmanagedKind::PaksFile,
            rel_path: "Stray.pak".into(),
            live_files: 1,
            copy_of: vec!["X".into()],
        }]
    );
    assert_eq!(
        dto.leftovers,
        vec![LeftoverEntry {
            mod_ref: "1|d|-|3-4|Mods/_Old_tmp_0".into(),
            rel_path: "Mods/_Old_tmp_0".into(),
        }]
    );
    assert_eq!(
        dto.stats,
        ScanStats {
            dirs_read: 12,
            entries_seen: 40,
            duration_ms: 2.5,
        }
    );
}

#[test]
fn row_converts_from_core() {
    let row = core_row();

    let dto = ModRow::from(&row);

    assert_eq!(dto.mod_ref, ROW_REF);
    assert_eq!(dto.key, "Standard:X");
    assert_eq!(dto.kind, ModKind::Standard);
    assert_eq!(dto.location, display_path(&paks().join("Mods").join("X")));
    assert_eq!(dto.state, ModState::Partial);
    assert_eq!(
        dto.payload,
        PayloadCounts {
            live_pak: 2,
            off_pak: 1,
            live_companion: 4,
            off_companion: 3,
        }
    );
    assert_eq!(dto.size_bytes, Some(10_000_000_000.0));
    assert_eq!(
        dto.info,
        ModInfo {
            version: "2.0".into(),
            category: "Character".into(),
            description: "Smoke fixture".into(),
            author: "RemMods".into(),
            origin: ModInfoOrigin::File,
            defaulted: ModInfoDefaulted {
                version: false,
                category: false,
                description: false,
                author: false,
            },
        }
    );
    assert_eq!(
        dto.conflicts_with,
        vec![ConflictLink {
            mod_ref: OTHER_REF.into(),
            key: "Standard:Y".into(),
            name: "Y".into(),
            paks: vec!["Shared.pak".into()],
        }]
    );
    assert!(dto.ambiguous);
    assert_eq!(
        dto.issues,
        vec![ModIssue::AmbiguousKey, ModIssue::ContainsLink]
    );
    let unknown_size = core::ModRow {
        size: None,
        ..core_row()
    };
    assert_eq!(ModRow::from(&unknown_size).size_bytes, None);
}

#[test]
fn snapshot_json_uses_camel_case_and_numbers() {
    let dto = ModsSnapshot::from(&core_snapshot());

    let value = to_json(&dto);

    assert_eq!(value["scannedAt"], json!("1970-01-01T00:00:01.500Z"));
    assert_eq!(value["rows"][0]["sizeBytes"], json!(10_000_000_000.0));
    assert_eq!(
        value["rows"][0]["conflictsWith"][0]["modRef"],
        json!(OTHER_REF)
    );
    assert_eq!(value["archives"][0]["sizeBytes"], json!(4.0));
    assert_eq!(
        value["roots"][0],
        json!({
            "kind": "legacy",
            "name": "~mods",
            "isLink": true,
            "caseVariant": false,
            "unreadable": { "op": "list", "class": "accessDenied" }
        })
    );
    assert_eq!(
        value["unmanaged"][0],
        json!({
            "modRef": "1|f|1-7|3-4|Stray.pak",
            "kind": "paksFile",
            "relPath": "Stray.pak",
            "liveFiles": 1,
            "copyOf": ["X"]
        })
    );
    assert_eq!(
        value["stats"],
        json!({ "dirsRead": 12, "entriesSeen": 40, "durationMs": 2.5 })
    );
    assert!(value.get("paksId").is_none());
    assert!(value["roots"][0].get("path").is_none());
    assert_eq!(from_json::<ModsSnapshot>(value), dto);
}

#[test]
fn details_convert_from_core() {
    let location = paks().join("Mods").join("X");
    let details = core::ModDetails {
        mod_ref: core::ModRef::from_token(ROW_REF),
        location: location.clone(),
        files: vec![
            core::PayloadFile {
                rel_path: "Content/X.pak".into(),
                kind: core::PayloadKind::Pak,
                live: true,
                size: 2048,
            },
            core::PayloadFile {
                rel_path: "Content/X.ucas-x".into(),
                kind: core::PayloadKind::Ucas,
                live: false,
                size: 0,
            },
        ],
        truncated: true,
        other_files: 7,
    };

    let dto = ModDetails::from(&details);

    assert_eq!(
        dto,
        ModDetails {
            mod_ref: ROW_REF.into(),
            location: display_path(&location),
            files: vec![
                PayloadFile {
                    rel_path: "Content/X.pak".into(),
                    kind: PayloadKind::Pak,
                    live: true,
                    size_bytes: 2048.0,
                },
                PayloadFile {
                    rel_path: "Content/X.ucas-x".into(),
                    kind: PayloadKind::Ucas,
                    live: false,
                    size_bytes: 0.0,
                },
            ],
            truncated: true,
            other_files: 7,
        }
    );
    assert_eq!(
        to_json(&dto.files[0]),
        json!({ "relPath": "Content/X.pak", "kind": "pak", "live": true, "sizeBytes": 2048.0 })
    );
    assert_eq!(to_json(&dto)["otherFiles"], json!(7));
}

#[test]
fn defaulted_info_keeps_the_raw_values() {
    let info = ModInfo::from(&default_info());

    assert_eq!(info.version, "1.0");
    assert_eq!(info.category, "Misc");
    assert_eq!(info.author, "Unknown");
    assert_eq!(info.origin, ModInfoOrigin::Missing);
    assert!(info.defaulted.version && info.defaulted.category);
    assert!(info.defaulted.description && info.defaulted.author);
}
