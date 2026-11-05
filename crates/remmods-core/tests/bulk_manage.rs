//! Bulk trash and bulk permanent delete (P3b §5.3–§5.5, §5.7 K1–K15). Data safety first.
#![allow(clippy::expect_used)]

mod mods_support;
mod support;

use std::time::{Duration, Instant};

use mods_support::DATA;
use mods_support::busy::{cancel_on, policy};
use mods_support::manage::{
    ToggleFixture, core_with_trash, core_without_trash, entry_target, mods_error, restart, tree,
};
use remmods_core::CoreError;
use remmods_core::mods::{
    BULK_MAX_TARGETS, BulkDeletePrepared, BulkDeleteRequest, BulkOutcome, BulkRowResult,
    BulkTrashRequest, DeleteRequest, DeleteTokens, DisableRecord, ManageBlock, ModRef, ModTarget,
    ModsError,
};
use remmods_core::platform::{OsKind, TrashKind, TrashUnavailableReason, os_kind};
use remmods_core::safe_fs::{BusyFault, BusyPhase, FaultPlan, FsError, Scope};

const ABC: &[(&str, &[u8])] = &[
    ("Mods/A/A.pak", DATA),
    ("Mods/A/A.ucas", DATA),
    ("Mods/B/B.pak", DATA),
    ("Mods/B/B.ucas", DATA),
    ("Mods/C/C.pak", DATA),
    ("Mods/C/C.ucas", DATA),
    ("Mods/Keep/Keep.pak", DATA),
];

fn targets(fixture: &ToggleFixture, keys: &[&str]) -> Vec<ModTarget> {
    keys.iter().map(|key| fixture.target(key)).collect()
}

fn stale_target() -> ModTarget {
    ModTarget {
        mod_ref: ModRef::from_token("1|d|-|g0|Mods/Gone"),
        key: "Standard:Gone".to_owned(),
    }
}

fn prepare(fixture: &ToggleFixture, targets: &[ModTarget]) -> BulkDeletePrepared {
    fixture
        .core
        .mods_delete_prepare_many(targets)
        .expect("prepare_many")
}

fn delete_many(
    fixture: &ToggleFixture,
    targets: &[ModTarget],
    token: &str,
) -> Result<BulkOutcome, CoreError> {
    fixture.core.mods_delete_many(&BulkDeleteRequest {
        targets: targets.to_vec(),
        token: token.to_owned(),
        apply_while_running: false,
    })
}

/// Prepare, then delete exactly the rows the token covers.
fn prepare_and_delete(fixture: &ToggleFixture, targets: &[ModTarget]) -> BulkOutcome {
    let prepared = prepare(fixture, targets);
    let token = prepared.token.expect("a token");
    delete_many(fixture, targets, &token).expect("delete_many")
}

fn trash_many(fixture: &ToggleFixture, targets: &[ModTarget]) -> Result<BulkOutcome, CoreError> {
    fixture.core.mods_trash_many(&BulkTrashRequest {
        targets: targets.to_vec(),
        apply_while_running: false,
    })
}

fn kinds(outcome: &BulkOutcome) -> Vec<&'static str> {
    outcome
        .rows
        .iter()
        .map(|row| match &row.result {
            BulkRowResult::Trashed { .. } => "trashed",
            BulkRowResult::Deleted { .. } => "deleted",
            BulkRowResult::TrashUnavailable { .. } => "unavailable",
            BulkRowResult::Failed(_) => "failed",
            BulkRowResult::NotRun => "notRun",
        })
        .collect()
}

fn failure(outcome: &BulkOutcome, index: usize) -> &CoreError {
    match &outcome.rows[index].result {
        BulkRowResult::Failed(error) => error,
        other => panic!("row {index}: expected Failed, got {other:?}"),
    }
}

fn is_busy_cancelled(error: &CoreError) -> bool {
    let fs = match error {
        CoreError::Fs(fs) | CoreError::Mods(ModsError::Fs(fs)) => fs,
        _ => return false,
    };
    match fs {
        FsError::BusyCancelled { .. } => true,
        FsError::Batch(batch) => matches!(batch.cause, FsError::BusyCancelled { .. }),
        _ => false,
    }
}

fn folder(fixture: &ToggleFixture, rel: &str) -> Vec<String> {
    support::tree_listing(&mods_support::manage::path_of(&fixture.paks, rel))
}

fn hidden_folders(fixture: &ToggleFixture) -> Vec<String> {
    fixture
        .names()
        .into_iter()
        .map(|(name, _, _)| name)
        .filter(|name| name.contains("_tmp_"))
        .collect()
}

#[test]
fn bulk_delete_without_token_changes_nothing() {
    let fixture = core_with_trash(ABC);
    let before = tree(&fixture);
    let rows = targets(&fixture, &["Standard:A", "Standard:B"]);
    let random = uuid::Uuid::new_v4().simple().to_string();
    for token in [random.as_str(), ""] {
        let result = delete_many(&fixture, &rows, token);
        assert!(matches!(mods_error(result), ModsError::DeleteToken));
    }
    assert_eq!(tree(&fixture), before);
}

#[test]
fn bulk_token_bound_to_exact_rows() {
    let fixture = core_with_trash(ABC);
    let before = tree(&fixture);
    let a = fixture.target("Standard:A");
    let b = fixture.target("Standard:B");
    let c = fixture.target("Standard:C");
    let pair = [a.clone(), b.clone()];
    let others: [Vec<ModTarget>; 3] = [
        vec![a.clone()],
        vec![b.clone(), a.clone()],
        vec![a.clone(), b.clone(), c],
    ];
    for other in &others {
        let token = prepare(&fixture, &pair).token.expect("token");
        let refused = delete_many(&fixture, other, &token);
        assert!(matches!(mods_error(refused), ModsError::DeleteToken));
        // Spent by the refusal: the right rows are refused now as well.
        let spent = delete_many(&fixture, &pair, &token);
        assert!(matches!(mods_error(spent), ModsError::DeleteToken));
    }
    let token = prepare(&fixture, &pair).token.expect("token");
    let single = fixture.core.mods_delete_permanently(&DeleteRequest {
        target: a,
        token: token.clone(),
        apply_while_running: false,
    });
    assert!(matches!(mods_error(single), ModsError::DeleteToken));
    let spent = delete_many(&fixture, &pair, &token);
    assert!(matches!(mods_error(spent), ModsError::DeleteToken));
    assert_eq!(tree(&fixture), before);
}

#[test]
fn bulk_token_expires_and_is_single_use() {
    let tokens = DeleteTokens::new();
    let rows = [
        ModTarget {
            mod_ref: ModRef::from_token("1|d|-|g0|Mods/A"),
            key: "Standard:A".to_owned(),
        },
        ModTarget {
            mod_ref: ModRef::from_token("1|d|-|g0|Mods/B"),
            key: "Standard:B".to_owned(),
        },
    ];
    let now = Instant::now();
    let token = tokens.issue_many(&rows, now);
    let late = now + DeleteTokens::TTL + Duration::from_secs(1);
    assert!(matches!(
        tokens.consume_many(&token, &rows, late),
        Err(ModsError::DeleteToken)
    ));
    let token = tokens.issue_many(&rows, now);
    tokens
        .consume_many(&token, &rows, now + DeleteTokens::TTL)
        .expect("within the TTL");
    assert!(matches!(
        tokens.consume_many(&token, &rows, now),
        Err(ModsError::DeleteToken)
    ));
}

#[test]
fn bulk_delete_set_aside_failure_rolls_back_only_that_row() {
    let fixture = core_with_trash(ABC);
    let b_before = folder(&fixture, "Mods/B");
    let rows = targets(&fixture, &["Standard:A", "Standard:B", "Standard:C"]);
    let prepared = prepare(&fixture, &rows);
    // Row B's set-aside: the folder, then B.pak and B.ucas → -x; the last op fails.
    fixture.core.inject_batch_faults(vec![
        None,
        Some(FaultPlan {
            fail_before: Some(2),
            ..FaultPlan::default()
        }),
        None,
    ]);
    let outcome = delete_many(&fixture, &rows, &prepared.token.expect("token")).expect("bulk");
    assert_eq!(kinds(&outcome), vec!["deleted", "failed", "deleted"]);
    assert!(matches!(
        failure(&outcome, 1),
        CoreError::Fs(FsError::Batch(_))
    ));
    assert!(!fixture.exists("Mods/A"));
    assert!(!fixture.exists("Mods/C"));
    assert_eq!(folder(&fixture, "Mods/B"), b_before);
    assert!(fixture.exists("Mods/Keep/Keep.pak"));
    assert!(hidden_folders(&fixture).is_empty());
    assert!(outcome.refreshed.is_some());
}

#[test]
fn bulk_delete_crash_mid_row_recovers_live() {
    let mut fixture = core_with_trash(ABC);
    let b_before = folder(&fixture, "Mods/B");
    let rows = targets(&fixture, &["Standard:A", "Standard:B"]);
    let prepared = prepare(&fixture, &rows);
    fixture.core.inject_batch_faults(vec![
        None,
        Some(FaultPlan {
            crash_after: Some(1),
            ..FaultPlan::default()
        }),
    ]);
    let outcome = delete_many(&fixture, &rows, &prepared.token.expect("token")).expect("bulk");
    assert_eq!(kinds(&outcome), vec!["deleted", "failed"]);
    assert!(!fixture.exists("Mods/B"), "the crash left B set aside");
    restart(&mut fixture);
    assert_eq!(folder(&fixture, "Mods/B"), b_before);
    assert!(!fixture.exists("Mods/A"));
    assert!(hidden_folders(&fixture).is_empty());
}

#[test]
fn bulk_delete_game_running_refused_once() {
    let fixture = core_with_trash(ABC);
    let before = tree(&fixture);
    let rows = targets(&fixture, &["Standard:A", "Standard:B"]);
    let token = prepare(&fixture, &rows).token.expect("token");
    fixture
        .running
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let refused = delete_many(&fixture, &rows, &token);
    assert!(matches!(mods_error(refused), ModsError::GameRunning));
    assert_eq!(tree(&fixture), before);
    let spent = delete_many(&fixture, &rows, &token);
    assert!(matches!(mods_error(spent), ModsError::DeleteToken));
    let token = prepare(&fixture, &rows).token.expect("token");
    let outcome = fixture
        .core
        .mods_delete_many(&BulkDeleteRequest {
            targets: rows,
            token,
            apply_while_running: true,
        })
        .expect("forced bulk delete");
    assert_eq!(kinds(&outcome), vec!["deleted", "deleted"]);
    assert!(!fixture.exists("Mods/A"));
    assert!(!fixture.exists("Mods/B"));
}

#[test]
fn bulk_prepare_reports_refusals_and_binds_eligible_only() {
    let mut entries = ABC.to_vec();
    entries.push(("Mods/Loose.pak", DATA));
    let fixture = core_with_trash(&entries);
    let before = tree(&fixture);
    let a = fixture.target("Standard:A");
    let loose = fixture.target("Loose:Mods/Loose");
    let b = fixture.target("Standard:B");
    let all = [a.clone(), loose, stale_target(), b.clone()];
    let prepared = prepare(&fixture, &all);
    assert_eq!(prepared.expires_in_secs, 120);
    assert_eq!(prepared.rows.len(), 4);
    assert!(prepared.rows[0].refusal.is_none());
    assert_eq!(prepared.rows[0].name, "A");
    assert_eq!(prepared.rows[0].size, fixture.row("Standard:A").size);
    assert!(prepared.rows[0].size.is_some());
    match &prepared.rows[1].refusal {
        Some(ModsError::ManageBlocked {
            reason: ManageBlock::Loose,
            ..
        }) => {}
        other => panic!("expected ManageBlocked(Loose), got {other:?}"),
    }
    assert!(matches!(prepared.rows[2].refusal, Some(ModsError::Stale)));
    assert_eq!(prepared.rows[2].name, "Standard:Gone");
    assert!(prepared.rows[3].refusal.is_none());
    assert_eq!(tree(&fixture), before, "prepare writes nothing");
    let token = prepared.token.expect("a token for the eligible rows");
    let outcome = delete_many(&fixture, &[a, b], &token).expect("eligible rows");
    assert_eq!(kinds(&outcome), vec!["deleted", "deleted"]);
    assert!(fixture.exists("Mods/Loose.pak"));
    assert!(fixture.exists("Mods/C/C.pak"));

    let none = prepare(&fixture, &[stale_target()]);
    assert!(none.token.is_none());
}

#[test]
fn bulk_delete_cancelled_wait_stops_the_rest() {
    let fixture = core_with_trash(ABC);
    let b_before = folder(&fixture, "Mods/B");
    let c_before = folder(&fixture, "Mods/C");
    let busy = fixture.core.ops.busy();
    busy.set_notice_after(Duration::ZERO);
    let recorder = cancel_on(busy, BusyPhase::Waiting);
    let rows = targets(&fixture, &["Standard:A", "Standard:B", "Standard:C"]);
    let token = prepare(&fixture, &rows).token.expect("token");
    fixture.core.inject_batch_faults(vec![
        None,
        Some(FaultPlan {
            busy: Some(BusyFault {
                op: 0,
                attempts: 50,
                undo: false,
            }),
            folder_policy: Some(policy(60, 3000)),
            ..FaultPlan::default()
        }),
        None,
    ]);
    let outcome = delete_many(&fixture, &rows, &token).expect("bulk");
    assert_eq!(kinds(&outcome), vec!["deleted", "failed", "notRun"]);
    assert!(is_busy_cancelled(failure(&outcome, 1)));
    assert_eq!(outcome.rows[2].name, "C");
    assert!(!fixture.exists("Mods/A"));
    assert_eq!(folder(&fixture, "Mods/B"), b_before);
    assert_eq!(folder(&fixture, "Mods/C"), c_before);
    let phases = recorder.phases();
    assert_eq!(phases.first(), Some(&BusyPhase::Waiting), "{phases:?}");
    assert_eq!(phases.last(), Some(&BusyPhase::Cancelled), "{phases:?}");
    assert_eq!(recorder.cancels().first(), Some(&true));
    assert!(hidden_folders(&fixture).is_empty());
}

#[test]
fn bulk_delete_accepted_cancel_is_never_lost() {
    let fixture = core_with_trash(ABC);
    let b_before = folder(&fixture, "Mods/B");
    let c_before = folder(&fixture, "Mods/C");
    let busy = fixture.core.ops.busy();
    busy.set_notice_after(Duration::ZERO);
    // Cancels while the wait reports Done: the try already succeeded, the cancel is accepted.
    let recorder = cancel_on(busy, BusyPhase::Done);
    let rows = targets(&fixture, &["Standard:A", "Standard:B", "Standard:C"]);
    let token = prepare(&fixture, &rows).token.expect("token");
    fixture.core.inject_batch_faults(vec![
        None,
        Some(FaultPlan {
            busy: Some(BusyFault {
                op: 0,
                attempts: 1,
                undo: false,
            }),
            folder_policy: Some(policy(20, 2000)),
            ..FaultPlan::default()
        }),
        None,
    ]);
    let outcome = delete_many(&fixture, &rows, &token).expect("bulk");
    assert_eq!(recorder.cancels(), vec![true]);
    // Op 0 (the folder) was not the last op of B's set-aside, so op 1 stops and B rolls back.
    assert_eq!(kinds(&outcome), vec!["deleted", "failed", "notRun"]);
    assert!(is_busy_cancelled(failure(&outcome, 1)));
    assert_eq!(folder(&fixture, "Mods/B"), b_before);
    assert_eq!(folder(&fixture, "Mods/C"), c_before);
    assert!(hidden_folders(&fixture).is_empty());
}

#[test]
fn bulk_targets_validated() {
    let fixture = core_with_trash(ABC);
    let before = tree(&fixture);
    let a = fixture.target("Standard:A");
    let too_many: Vec<ModTarget> = (0..=BULK_MAX_TARGETS)
        .map(|index| ModTarget {
            mod_ref: ModRef::from_token(format!("1|d|-|g0|Mods/X{index}")),
            key: format!("Standard:X{index}"),
        })
        .collect();
    assert_eq!(too_many.len(), 1001);
    let twice = vec![
        a.clone(),
        ModTarget {
            mod_ref: a.mod_ref.clone(),
            key: "Standard:Other".to_owned(),
        },
    ];
    for list in [Vec::new(), too_many, twice] {
        assert!(matches!(
            mods_error(fixture.core.mods_delete_prepare_many(&list)),
            ModsError::BulkTargets
        ));
        assert!(matches!(
            mods_error(trash_many(&fixture, &list)),
            ModsError::BulkTargets
        ));
        assert!(matches!(
            mods_error(delete_many(&fixture, &list, "token")),
            ModsError::BulkTargets
        ));
    }
    assert_eq!(ModsError::BulkTargets.code(), "bulk_targets");
    assert_eq!(tree(&fixture), before);
}

#[test]
fn bulk_trash_without_usable_trash_changes_nothing() {
    let fixture = core_without_trash(ABC);
    let before = tree(&fixture);
    let rows = targets(&fixture, &["Standard:A", "Standard:B"]);
    let expected = if os_kind() == OsKind::Windows {
        TrashUnavailableReason::NotSupported
    } else {
        TrashUnavailableReason::OtherDevice
    };
    let outcome = trash_many(&fixture, &rows).expect("bulk trash");
    for (row, key) in outcome.rows.iter().zip(["Standard:A", "Standard:B"]) {
        let size = fixture.row(key).size;
        assert!(size.is_some());
        match &row.result {
            BulkRowResult::TrashUnavailable { reason, size: got } => {
                assert_eq!(*reason, expected);
                assert_eq!(*got, size);
            }
            other => panic!("expected TrashUnavailable, got {other:?}"),
        }
    }
    assert_eq!(tree(&fixture), before);
}

#[test]
fn bulk_rows_report_refusals_in_request_order() {
    let mut entries = ABC.to_vec();
    entries.push(("Stray.pak", DATA));
    let fixture = core_with_trash(&entries);
    let before = tree(&fixture);
    let unmanaged = entry_target(&fixture.snapshot().content, "Stray.pak");
    let a = fixture.target("Standard:A");
    let rows = [unmanaged, stale_target(), a];
    let outcome = trash_many(&fixture, &rows).expect("bulk trash");
    assert_eq!(outcome.rows.len(), 3);
    for (row, target) in outcome.rows.iter().zip(&rows) {
        assert_eq!(&row.target, target);
    }
    match failure(&outcome, 0) {
        CoreError::Mods(ModsError::ManageBlocked {
            reason: ManageBlock::Unmanaged,
            ..
        }) => {}
        other => panic!("expected ManageBlocked(Unmanaged), got {other:?}"),
    }
    assert!(matches!(
        failure(&outcome, 1),
        CoreError::Mods(ModsError::Stale)
    ));
    assert_eq!(outcome.rows[1].name, "Standard:Gone");
    assert_eq!(outcome.rows[2].name, "A");
    if os_kind() == OsKind::Windows {
        assert_eq!(kinds(&outcome)[2], "unavailable");
        assert_eq!(tree(&fixture), before);
    } else {
        assert!(matches!(
            outcome.rows[2].result,
            BulkRowResult::Trashed {
                items: 1,
                kind: TrashKind::Home
            }
        ));
        assert!(!fixture.exists("Mods/A"));
        assert!(fixture.exists("Stray.pak"));
    }
}

#[test]
fn bulk_delete_meta_written_once() {
    let fixture = core_with_trash(&[
        ("Mods/A/A.pak", DATA),
        ("Mods/B/B.pak", DATA),
        ("Mods/Keep/Keep.pak", DATA),
        ("LogicMods/X/X.pak", DATA),
        ("LogicMods/.disabled_X/X.pak-x", DATA),
    ]);
    let record = DisableRecord {
        files: vec!["x.pak".to_owned()],
        at: "2026-11-02T00:00:00Z".to_owned(),
    };
    {
        let app_dirs = [
            fixture.paths.data_dir.as_path(),
            fixture.paths.config_dir.as_path(),
        ];
        let scope = Scope::for_paks(&fixture.paks, &app_dirs).expect("scope");
        let guard = fixture.core.ops.write(scope).expect("guard");
        fixture
            .core
            .mods
            .meta()
            .update(&guard, |meta| {
                for key in ["Standard:A", "Standard:B", "Standard:Keep", "Logic:X"] {
                    meta.set_record(key, Some(record.clone()));
                }
            })
            .expect("meta");
    }
    let save = remmods_core::profiles::ProfileSave::New {
        name: "P".to_owned(),
        description: String::new(),
        mode: remmods_core::profiles::ProfileMode::Additive,
    };
    fixture.core.profiles_save(&save).expect("save profile");
    let snapshot = fixture.snapshot();
    let logic_live = snapshot
        .content
        .rows
        .iter()
        .find(|row| row.key == "Logic:X" && row.path.ends_with("X"))
        .expect("LogicMods/X");
    let logic = ModTarget {
        mod_ref: logic_live.mod_ref.clone(),
        key: logic_live.key.clone(),
    };
    let mut rows = targets(&fixture, &["Standard:A", "Standard:B"]);
    rows.push(logic);
    let outcome = prepare_and_delete(&fixture, &rows);
    assert_eq!(kinds(&outcome), vec!["deleted", "deleted", "deleted"]);
    assert!(fixture.exists("LogicMods/.disabled_X/X.pak-x"));
    let meta = fixture.core.mods.meta().load().meta;
    assert!(meta.record("Standard:A").is_none());
    assert!(meta.record("Standard:B").is_none());
    assert!(meta.record("Standard:Keep").is_some());
    assert!(
        meta.record("Logic:X").is_some(),
        "LogicMods/.disabled_X still has the key"
    );
    let view = fixture.core.profiles_list().expect("profiles");
    let profile = &view.profiles[0].profile;
    assert!(profile.mods.contains_key("Standard:A"));
    assert!(profile.mods.contains_key("Standard:B"));
}

#[test]
fn bulk_publishes_like_a_single_call() {
    let fixture = core_with_trash(ABC);
    let start = fixture.snapshot().revision;
    let single = fixture.target("Standard:Keep");
    let prepared = fixture.core.mods_delete_prepare(&single).expect("prepare");
    let outcome = fixture
        .core
        .mods_delete_permanently(&DeleteRequest {
            target: single,
            token: prepared.token,
            apply_while_running: false,
        })
        .expect("single delete");
    let single_delta = outcome.refreshed.snapshot.revision - start;
    let rows = targets(&fixture, &["Standard:A", "Standard:B", "Standard:C"]);
    let middle = fixture.snapshot().revision;
    let outcome = prepare_and_delete(&fixture, &rows);
    let bulk_delta = outcome.refreshed.expect("refreshed").snapshot.revision - middle;
    assert_eq!(bulk_delta, single_delta);
    assert_eq!(bulk_delta, 1);
}

#[test]
fn bulk_delete_refused_removal_reports_leftover() {
    let fixture = core_with_trash(&[
        ("Mods/A/A.pak", DATA),
        ("Mods/A/Data.pak/readme.txt", DATA),
        ("Mods/B/B.pak", DATA),
        ("Mods/B/Data.pak/readme.txt", DATA),
    ]);
    let outcome = prepare_and_delete(&fixture, &targets(&fixture, &["Standard:A"]));
    let leftover = match &outcome.rows[0].result {
        BulkRowResult::Deleted {
            leftover: Some(leftover),
        } => leftover.clone(),
        other => panic!("expected Deleted with a leftover, got {other:?}"),
    };
    assert!(leftover.starts_with("Mods/_A_tmp_"), "{leftover}");
    assert!(!fixture.exists("Mods/A"));
    assert!(fixture.exists(&format!("{leftover}/A.pak-x")));
    assert!(!fixture.exists(&format!("{leftover}/A.pak")));
    let snapshot = fixture.snapshot();
    assert!(
        snapshot
            .content
            .leftovers
            .iter()
            .any(|entry| entry.rel_path == leftover)
    );
    assert!(
        snapshot
            .content
            .rows
            .iter()
            .all(|row| row.key != "Standard:A")
    );

    let target = fixture.target("Standard:B");
    let prepared = fixture.core.mods_delete_prepare(&target).expect("prepare");
    let single = fixture
        .core
        .mods_delete_permanently(&DeleteRequest {
            target,
            token: prepared.token,
            apply_while_running: false,
        })
        .expect("single delete");
    let leftover = single.leftover.expect("a leftover");
    assert!(leftover.starts_with("Mods/_B_tmp_"), "{leftover}");
    assert!(!fixture.exists("Mods/B"));
}

#[test]
fn bulk_rows_survive_a_failed_final_refresh() {
    let fixture = core_with_trash(ABC);
    let rows = targets(&fixture, &["Standard:A", "Standard:B"]);
    let token = prepare(&fixture, &rows).token.expect("token");
    fixture.core.fail_next_final_refresh();
    let before = fixture.core.mods_last().expect("published").revision;
    let outcome = delete_many(&fixture, &rows, &token).expect("rows still count");
    assert!(outcome.refreshed.is_none());
    assert_eq!(kinds(&outcome), vec!["deleted", "deleted"]);
    assert!(!fixture.exists("Mods/A"));
    // The cached scan from before the loop is stale: mods_list scans again, revision moves on.
    let listed = fixture.core.mods_list().expect("list rescans");
    let keys: Vec<&str> = listed
        .content
        .rows
        .iter()
        .map(|row| row.key.as_str())
        .collect();
    assert!(
        !keys.contains(&"Standard:A") && !keys.contains(&"Standard:B"),
        "{keys:?}"
    );
    assert!(listed.revision > before);

    fixture.core.fail_next_final_refresh();
    let outcome = trash_many(&fixture, &targets(&fixture, &["Standard:C"])).expect("trash");
    assert!(outcome.refreshed.is_none());
    let expected = if os_kind() == OsKind::Windows {
        "unavailable"
    } else {
        "trashed"
    };
    assert_eq!(kinds(&outcome), vec![expected]);
    // The hook fires once.
    let outcome = trash_many(&fixture, &targets(&fixture, &["Standard:Keep"])).expect("trash");
    assert!(outcome.refreshed.is_some());
}
