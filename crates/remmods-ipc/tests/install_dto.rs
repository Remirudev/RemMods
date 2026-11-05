//! Install DTOs and the install rows of the core → AppError table (SPEC-P4 §6, §8.3 I1–I4).
#![allow(clippy::expect_used)]

use std::io;

use remmods_core::CoreError;
use remmods_core::archive::{self, ArchiveError, UnsupportedReason};
use remmods_core::install as core;
use remmods_core::mods::{self as core_mods, ModRef};
use remmods_core::platform::OsErrorClass as CoreClass;
use remmods_core::safe_fs::names::NameIssue;
use remmods_ipc::{
    AppError, ArchiveProblem, CandidatePick, CaseClash, ClashPick, DropPhase, ExistingMatch,
    ExistingRelation, InspectRequest, Inspection, InstallBlock, InstallCandidate, InstallIssue,
    InstallIssueCode, InstallPhase, InstallProgress, InstallSourceRef, InstalledMod, IssueSeverity,
    KeptReason, NameEncoding, NameOverride, OsErrorClass, PickedSource, RejectedSource,
    ReplacedOld, RootKind, RootReason, SourceKind, SourceReject, StartRequest, VariantGroup,
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

fn install(error: core::InstallError) -> AppError {
    AppError::from(CoreError::Install(error))
}

fn archive_err(error: ArchiveError) -> AppError {
    install(core::InstallError::Archive(error))
}

fn core_candidate() -> core::Candidate {
    core::Candidate {
        id: 3,
        name: "CON".into(),
        inner: "Pack/CON".into(),
        files: 4,
        payload: 2,
        shipped_off: 1,
        bytes: 5_000_000_000,
        group: Some(1),
        selected: true,
        name_issue: Some(NameIssue::ReservedDeviceName),
        fixed_name: Some("CON_".into()),
        existing: Some(core::ExistingMatch {
            mod_ref: ModRef::from_token(ROW_REF),
            key: "Standard:CON_".into(),
            rel: "Mods/CON_".into(),
            relation: core::ExistingRelation::FoldEqual,
            ambiguous: false,
        }),
        also_in: vec![core_mods::RootKind::Logic],
        path_too_long: true,
    }
}

fn core_inspection() -> core::Inspection {
    core::Inspection {
        id: "insp-0123".into(),
        source_name: "pack.7z".into(),
        format: archive::ArchiveFormat::SevenZip,
        encoding: archive::NameEncoding::ShiftJis,
        legacy_names: true,
        entries: 9,
        files: 6,
        total_bytes: 7_000_000_000,
        detected_root: core_mods::RootKind::Legacy,
        root_reason: core::RootReason::PathSegment,
        root: core_mods::RootKind::Standard,
        candidates: vec![core_candidate()],
        groups: vec![core::VariantGroup {
            id: 1,
            candidates: vec![3, 4],
        }],
        clashes: vec![core::CaseClash {
            id: 2,
            paths: vec!["a.pak".into(), "A.pak".into()],
        }],
        issues: vec![core::InstallIssue {
            severity: core::IssueSeverity::Warn,
            code: core::IssueCode::ShipsBothStates,
            path: "Pack".into(),
            count: 2,
        }],
        blocked: false,
        available_bytes: Some(1 << 40),
    }
}

#[test]
fn i1_inspection_serializes_camel_case() {
    let value = to_json(&Inspection::from(&core_inspection()));
    assert_eq!(value["sourceName"], "pack.7z");
    assert_eq!(value["format"], "sevenZip");
    assert_eq!(value["encoding"], "shiftJis");
    assert_eq!(value["legacyNames"], true);
    assert_eq!(value["totalBytes"], 7_000_000_000.0);
    assert_eq!(value["detectedRoot"], "legacy");
    assert_eq!(value["rootReason"], "pathSegment");
    assert_eq!(value["root"], "standard");
    assert_eq!(value["availableBytes"], 1_099_511_627_776.0);
    assert_eq!(value["groups"], json!([{ "id": 1, "candidates": [3, 4] }]));
    assert_eq!(
        value["clashes"],
        json!([{ "id": 2, "paths": ["a.pak", "A.pak"] }])
    );
    assert_eq!(
        value["issues"],
        json!([{ "severity": "warn", "code": "shipsBothStates", "path": "Pack", "count": 2 }])
    );
    assert_eq!(
        value["candidates"][0],
        json!({
            "id": 3, "name": "CON", "inner": "Pack/CON", "files": 4, "payload": 2,
            "shippedOff": 1, "bytes": 5_000_000_000.0, "group": 1, "selected": true,
            "nameIssue": "reserved_device_name", "fixedName": "CON_",
            "existing": {
                "modRef": ROW_REF, "key": "Standard:CON_", "rel": "Mods/CON_",
                "relation": "foldEqual", "ambiguous": false
            },
            "alsoIn": ["logic"], "pathTooLong": true
        })
    );
}

#[test]
fn i1_unavailable_space_is_null() {
    let mut inspection = core_inspection();
    inspection.available_bytes = None;
    assert_eq!(
        to_json(&Inspection::from(&inspection))["availableBytes"],
        Value::Null
    );
}

#[test]
fn i1_sources_progress_and_results_serialize_camel_case() {
    assert_eq!(
        to_json(&PickedSource {
            id: "src-ab".into(),
            name: "a.zip".into(),
            kind: SourceKind::SevenZip,
        }),
        json!({ "id": "src-ab", "name": "a.zip", "kind": "sevenZip" })
    );
    assert_eq!(
        to_json(&RejectedSource {
            name: "b".into(),
            reason: SourceReject::InsidePaks,
        }),
        json!({ "name": "b", "reason": "insidePaks" })
    );
    assert_eq!(to_json(&SourceReject::TooMany), json!("tooMany"));
    assert_eq!(
        to_json(&SourceReject::UnsupportedType),
        json!("unsupportedType")
    );
    assert_eq!(to_json(&DropPhase::Dropped), json!("dropped"));
    let progress = InstallProgress::from(&core::InstallProgress {
        phase: core::InstallPhase::CleaningUp,
        done_bytes: 10,
        total_bytes: 20,
        done_files: 1,
        total_files: 2,
    });
    assert_eq!(
        to_json(&progress),
        json!({ "phase": "cleaningUp", "doneBytes": 10.0, "totalBytes": 20.0, "doneFiles": 1, "totalFiles": 2 })
    );
    let installed = InstalledMod::from(&core::InstalledMod {
        key: "Standard:Cool".into(),
        mod_ref: Some(ModRef::from_token(ROW_REF)),
        rel: "Mods/Cool".into(),
        files: 3,
        replaced: Some(core::ReplacedOld::KeptInRepair {
            reason: core::KeptReason::TrashUnavailable,
        }),
    });
    assert_eq!(
        to_json(&installed),
        json!({
            "key": "Standard:Cool", "modRef": ROW_REF, "rel": "Mods/Cool", "files": 3,
            "replaced": { "kind": "keptInRepair", "reason": "trashUnavailable" }
        })
    );
    assert_eq!(to_json(&ReplacedOld::Trashed), json!({ "kind": "trashed" }));
    assert_eq!(to_json(&KeptReason::TrashFailed), json!("trashFailed"));
    assert_eq!(to_json(&InstallPhase::Extracting), json!("extracting"));
}

#[test]
fn i1_enums_cover_every_core_variant() {
    assert_eq!(
        to_json(&InstallIssueCode::from(core::IssueCode::AlsoInRoot)),
        json!("alsoInRoot")
    );
    assert_eq!(
        to_json(&InstallIssueCode::from(core::IssueCode::EmptyDirs)),
        json!("emptyDirs")
    );
    assert_eq!(
        to_json(&IssueSeverity::from(core::IssueSeverity::Block)),
        json!("block")
    );
    assert_eq!(
        to_json(&RootReason::from(core::RootReason::ConfigLua)),
        json!("configLua")
    );
    assert_eq!(
        to_json(&ExistingRelation::from(
            core::ExistingRelation::DisabledPrefix
        )),
        json!("disabledPrefix")
    );
    assert_eq!(
        to_json(&SourceReject::from(core::SourceReject::Link)),
        json!("link")
    );
    assert_eq!(
        to_json(&SourceKind::from(core::SourceKind::Folder)),
        json!("folder")
    );
    assert_eq!(to_json(&NameEncoding::Windows1251), json!("windows1251"));
    assert_eq!(to_json(&NameEncoding::EucKr), json!("eucKr"));
    assert_eq!(
        to_json(&InstallBlock::from(core::InstallBlock::ReplaceNotConfirmed)),
        json!("replaceNotConfirmed")
    );
}

#[test]
fn i3_source_refs_deserialize_from_the_frontend_json() {
    assert_eq!(
        from_json::<InstallSourceRef>(json!({ "kind": "token", "id": "src-1" })),
        InstallSourceRef::Token { id: "src-1".into() }
    );
    assert_eq!(
        from_json::<InstallSourceRef>(json!({ "kind": "archive", "row": ROW_REF })),
        InstallSourceRef::Archive {
            row: ROW_REF.into()
        }
    );
    let request: InspectRequest = from_json(json!({
        "source": { "kind": "archive", "row": ROW_REF },
        "encoding": "auto",
        "root": null,
        "names": [{ "id": 2, "name": "Cool" }]
    }));
    assert_eq!(
        request.names,
        vec![NameOverride {
            id: 2,
            name: "Cool".into()
        }]
    );
    let options = core::InspectOptions::from(&request);
    assert_eq!(options.encoding, archive::NameEncoding::Auto);
    assert_eq!(options.root, None);
    assert_eq!(options.names[0].name, "Cool");
    let rooted: InspectRequest = from_json(json!({
        "source": { "kind": "token", "id": "src-1" },
        "encoding": "big5",
        "root": "logic",
        "names": []
    }));
    let options = core::InspectOptions::from(&rooted);
    assert_eq!(options.root, Some(core_mods::RootKind::Logic));
    assert_eq!(options.encoding, archive::NameEncoding::Big5);
}

#[test]
fn i3_start_request_deserializes_from_the_frontend_json() {
    let request: StartRequest = from_json(json!({
        "inspection": "insp-1",
        "picks": [{ "id": 3, "replace": true }],
        "clashes": [{ "id": 2, "keep": 1 }],
        "autoFix": true,
        "applyWhileRunning": false
    }));
    assert_eq!(
        request,
        StartRequest {
            inspection: "insp-1".into(),
            picks: vec![CandidatePick {
                id: 3,
                replace: true
            }],
            clashes: vec![ClashPick { id: 2, keep: 1 }],
            auto_fix: true,
            apply_while_running: false,
        }
    );
    let core_request = core::StartRequest::from(&request);
    assert_eq!(core_request.inspection, "insp-1");
    assert_eq!(
        core_request.picks,
        vec![core::CandidatePick {
            id: 3,
            replace: true
        }]
    );
    assert_eq!(
        core_request.clashes,
        vec![core::ClashPick { id: 2, keep: 1 }]
    );
    assert!(core_request.auto_fix);
    assert!(!core_request.apply_while_running);
}

#[test]
fn i4_install_dtos_carry_no_64_bit_integers() {
    for (name, source) in [
        ("install.rs", include_str!("../src/install.rs")),
        ("error.rs", include_str!("../src/error.rs")),
    ] {
        for banned in ["u64", "usize", "i64", "PathBuf", "OsString"] {
            assert!(
                !source
                    .split(|c: char| !c.is_alphanumeric())
                    .any(|word| word == banned),
                "{name} names {banned}"
            );
        }
    }
    let candidate: InstallCandidate = InstallCandidate::from(&core_candidate());
    let _: f64 = candidate.bytes;
    let existing: Option<ExistingMatch> = candidate.existing;
    assert!(existing.is_some());
    let _: Option<VariantGroup> = None;
    let _: Option<CaseClash> = None;
    let _: Option<InstallIssue> = None;
}

#[test]
fn i2_archive_errors_map_to_archive_failed() {
    let cases = [
        (ArchiveError::NotAnArchive, ArchiveProblem::NotAnArchive),
        (
            ArchiveError::Unsupported {
                reason: UnsupportedReason::MultiVolume,
            },
            ArchiveProblem::MultiVolume,
        ),
        (
            ArchiveError::Unsupported {
                reason: UnsupportedReason::SelfExtracting,
            },
            ArchiveProblem::SelfExtracting,
        ),
        (
            ArchiveError::Unsupported {
                reason: UnsupportedReason::Method,
            },
            ArchiveProblem::UnsupportedMethod,
        ),
        (
            ArchiveError::Unsupported {
                reason: UnsupportedReason::RarDisabled,
            },
            ArchiveProblem::RarDisabled,
        ),
        (ArchiveError::Encrypted, ArchiveProblem::Encrypted),
        (
            ArchiveError::Corrupt {
                detail: "bad crc".into(),
            },
            ArchiveProblem::Corrupt,
        ),
        (
            ArchiveError::SizeMismatch { index: 4 },
            ArchiveProblem::Corrupt,
        ),
        (
            ArchiveError::MemoryLimit {
                needed: u64::MAX,
                limit: 1,
            },
            ArchiveProblem::MemoryLimit,
        ),
        (
            ArchiveError::TooManyEntries { count: 70_000 },
            ArchiveProblem::TooManyEntries,
        ),
        (
            ArchiveError::TooLarge { bytes: 1 << 40 },
            ArchiveProblem::TooLarge,
        ),
    ];
    for (error, reason) in cases {
        assert_eq!(archive_err(error), AppError::ArchiveFailed { reason });
    }
    assert_eq!(
        to_json(&archive_err(ArchiveError::NotAnArchive)),
        json!({ "kind": "archiveFailed", "reason": "notAnArchive" })
    );
}

#[test]
fn i2_archive_io_maps_to_io_with_the_side() {
    let read = archive_err(ArchiveError::Io {
        op: "read",
        class: CoreClass::AccessDenied,
        source: io::Error::other("x"),
    });
    assert_eq!(
        read,
        AppError::Io {
            op: "archive_read".into(),
            class: OsErrorClass::AccessDenied,
        }
    );
    let write = archive_err(ArchiveError::Target {
        class: CoreClass::NoSpace,
        source: io::Error::other("x"),
    });
    assert_eq!(
        write,
        AppError::Io {
            op: "archive_write".into(),
            class: OsErrorClass::NoSpace,
        }
    );
}

#[test]
fn i2_cancel_from_either_layer_is_install_cancelled() {
    assert_eq!(
        archive_err(ArchiveError::Cancelled),
        AppError::InstallCancelled
    );
    assert_eq!(
        install(core::InstallError::Cancelled),
        AppError::InstallCancelled
    );
    assert_eq!(
        to_json(&AppError::InstallCancelled),
        json!({ "kind": "installCancelled" })
    );
}

#[test]
fn i2_install_errors_map_per_table() {
    assert_eq!(install(core::InstallError::Busy), AppError::InstallBusy);
    assert_eq!(
        install(core::InstallError::UnknownInspection),
        AppError::Stale
    );
    assert_eq!(install(core::InstallError::SourceChanged), AppError::Stale);
    assert_eq!(
        install(core::InstallError::Blocked {
            reason: core::InstallBlock::TargetChanged,
        }),
        AppError::InstallBlocked {
            reason: InstallBlock::TargetChanged,
        }
    );
    assert_eq!(
        install(core::InstallError::Source {
            reason: core::SourceReject::InsidePaks,
        }),
        AppError::SourceRejected {
            reason: SourceReject::InsidePaks,
        }
    );
    let no_space = install(core::InstallError::NoSpace {
        needed: 6_000_000_000,
        available: 5,
    });
    assert_eq!(
        no_space,
        AppError::NoSpace {
            needed: 6_000_000_000.0,
            available: 5.0,
        }
    );
    assert_eq!(
        to_json(&no_space),
        json!({ "kind": "noSpace", "needed": 6_000_000_000.0, "available": 5.0 })
    );
    assert_eq!(
        to_json(&AppError::InstallBlocked {
            reason: InstallBlock::UnresolvedClash,
        }),
        json!({ "kind": "installBlocked", "reason": "unresolvedClash" })
    );
    assert_eq!(
        to_json(&AppError::SourceRejected {
            reason: SourceReject::Missing,
        }),
        json!({ "kind": "sourceRejected", "reason": "missing" })
    );
    assert_eq!(
        to_json(&AppError::InstallBusy),
        json!({ "kind": "installBusy" })
    );
}

#[test]
fn i2_every_block_reason_maps_one_to_one() {
    let pairs = [
        (core::InstallBlock::Issues, "issues"),
        (core::InstallBlock::InvalidName, "invalidName"),
        (core::InstallBlock::UnresolvedClash, "unresolvedClash"),
        (core::InstallBlock::NoSelection, "noSelection"),
        (core::InstallBlock::VariantConflict, "variantConflict"),
        (
            core::InstallBlock::ReplaceNotConfirmed,
            "replaceNotConfirmed",
        ),
        (core::InstallBlock::ReplaceMismatch, "replaceMismatch"),
        (core::InstallBlock::Ambiguous, "ambiguous"),
        (core::InstallBlock::TargetChanged, "targetChanged"),
    ];
    for (block, wire) in pairs {
        assert_eq!(to_json(&InstallBlock::from(block)), json!(wire));
    }
}

#[test]
fn root_kind_round_trips_into_core() {
    for kind in core_mods::RootKind::ALL {
        assert_eq!(core_mods::RootKind::from(RootKind::from(kind)), kind);
    }
}
