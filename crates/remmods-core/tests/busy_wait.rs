//! Folder-busy waits (WG-2, P3b §5.7 B1–B10): visible waits, Cancel rolls the batch back and
//! changes nothing, rollbacks cannot be cancelled. Data safety first.
#![allow(clippy::expect_used)]

mod mods_support;
mod support;

use std::time::Duration;

use mods_support::busy::{Recorder, cancel_on, policy, record};
use remmods_core::platform::{OsKind, RetryPolicy, os_kind};
use remmods_core::safe_fs::{
    BatchError, BatchOptions, BatchReport, BusyFault, BusyPhase, FaultPlan, FsError, OpGate,
    RenameOp, RetryMode,
};
use support::FsFixture;

fn fixture() -> FsFixture {
    let fixture = support::fs_fixture();
    support::write_tree(
        &fixture.paks,
        &[
            ("Mods/A/a.pak", b"a"),
            ("Mods/A/b.pak", b"b"),
            ("Mods/B/B.pak", b"B"),
            ("Mods/X/X.pak", b"X"),
        ],
    );
    fixture
}

fn busy_fault(op: usize, attempts: u32, undo: bool) -> Option<BusyFault> {
    Some(BusyFault { op, attempts, undo })
}

fn normal(fault: FaultPlan) -> BatchOptions {
    BatchOptions {
        label: "test",
        retry: RetryMode::Normal,
        fault: Some(fault),
    }
}

fn run(
    fixture: &FsFixture,
    gate: &OpGate,
    ops: &[RenameOp],
    fault: FaultPlan,
) -> Result<BatchReport, FsError> {
    let guard = gate.write(fixture.scope()).expect("write guard");
    guard.rename_batch(ops, &normal(fault))
}

fn quick_gate(fixture: &FsFixture) -> OpGate {
    let gate = fixture.gate();
    gate.busy().set_notice_after(Duration::ZERO);
    gate
}

fn batch_error(result: Result<BatchReport, FsError>) -> BatchError {
    match result {
        Err(FsError::Batch(error)) => *error,
        other => panic!("expected a batch error, got {other:?}"),
    }
}

fn wait_override(attempts: u32, total_ms: u64) -> Option<RetryPolicy> {
    Some(policy(attempts, total_ms))
}

fn assert_ids_consistent(recorder: &Recorder) {
    let events = recorder.events();
    let first = events.first().expect("an event").wait_id;
    assert_ne!(first, 0);
    assert!(events.iter().all(|event| event.wait_id == first));
}

#[test]
fn cancel_rolls_back_and_changes_nothing() {
    let fixture = fixture();
    let before = support::tree_listing(&fixture.paks);
    let gate = quick_gate(&fixture);
    let recorder = cancel_on(gate.busy(), BusyPhase::Waiting);
    let ops = [
        fixture.op("Mods/A/a.pak", "Mods/A/a.pak-x", false),
        fixture.op("Mods/B", "Mods/B2", true),
    ];
    let fault = FaultPlan {
        busy: busy_fault(1, 50, false),
        folder_policy: wait_override(60, 3000),
        ..FaultPlan::default()
    };
    let error = batch_error(run(&fixture, &gate, &ops, fault));
    assert!(
        matches!(error.cause, FsError::BusyCancelled { .. }),
        "{error:?}"
    );
    assert_eq!(error.failed, 1);
    assert_eq!(error.rolled_back, 1);
    assert!(error.stuck.is_empty());
    assert_eq!(support::tree_listing(&fixture.paks), before);
    assert_eq!(support::journal_entries(&fixture.journal_path()), 0);
    let phases = recorder.phases();
    assert_eq!(phases.first(), Some(&BusyPhase::Waiting), "{phases:?}");
    assert_eq!(phases.last(), Some(&BusyPhase::Cancelled), "{phases:?}");
    assert!(
        phases[..phases.len() - 1]
            .iter()
            .all(|phase| *phase == BusyPhase::Waiting)
    );
    assert_eq!(recorder.cancels().first(), Some(&true));
    assert_ids_consistent(&recorder);
    let events = recorder.events();
    assert!(events.iter().all(|event| event.cancellable));
    assert!(events.iter().all(|event| event.folder == "Mods/B"));
    assert!(events.iter().all(|event| event.holders.is_none()));
    assert!(
        events
            .iter()
            .all(|event| event.limit == Duration::from_secs(3))
    );
}

#[test]
fn cancel_after_last_try_stops_the_next_op() {
    let fixture = fixture();
    let before = support::tree_listing(&fixture.paks);
    let gate = quick_gate(&fixture);
    // The sink cancels while op 0 reports Done (before the wait ends): accepted and sticky.
    let recorder = cancel_on(gate.busy(), BusyPhase::Done);
    let ops = [
        fixture.op("Mods/A", "Mods/A2", true),
        fixture.op("Mods/B", "Mods/B2", true),
    ];
    let fault = FaultPlan {
        busy: busy_fault(0, 1, false),
        folder_policy: wait_override(20, 2000),
        ..FaultPlan::default()
    };
    let error = batch_error(run(&fixture, &gate, &ops, fault));
    assert!(
        matches!(error.cause, FsError::BusyCancelled { .. }),
        "{error:?}"
    );
    assert_eq!(error.failed, 1);
    assert_eq!(error.rolled_back, 1);
    assert!(error.stuck.is_empty());
    assert_eq!(recorder.cancels(), vec![true]);
    assert_eq!(recorder.phases(), vec![BusyPhase::Waiting, BusyPhase::Done]);
    assert_eq!(support::tree_listing(&fixture.paks), before);
    assert_eq!(support::journal_entries(&fixture.journal_path()), 0);
}

#[test]
fn rollback_wait_is_not_cancellable() {
    let fixture = fixture();
    let before = support::tree_listing(&fixture.paks);
    let gate = quick_gate(&fixture);
    let recorder = cancel_on(gate.busy(), BusyPhase::Waiting);
    let ops = [
        fixture.op("Mods/X", "Mods/X2", true),
        fixture.op("Mods/A/a.pak", "Mods/A/a.pak-x", false),
        fixture.op("Mods/A/b.pak", "Mods/A/b.pak-x", false),
    ];
    let fault = FaultPlan {
        fail_before: Some(2),
        busy: busy_fault(0, 3, true),
        folder_policy: wait_override(20, 2000),
        ..FaultPlan::default()
    };
    let error = batch_error(run(&fixture, &gate, &ops, fault));
    assert_eq!(error.failed, 2);
    assert_eq!(error.rolled_back, 2);
    assert!(error.stuck.is_empty(), "{error:?}");
    assert_eq!(support::tree_listing(&fixture.paks), before);
    assert_eq!(support::journal_entries(&fixture.journal_path()), 0);
    let cancels = recorder.cancels();
    assert!(!cancels.is_empty());
    assert!(cancels.iter().all(|accepted| !accepted));
    let events = recorder.events();
    assert!(events.iter().all(|event| !event.cancellable));
    assert!(events.iter().all(|event| event.folder == "Mods/X2"));
    assert_eq!(recorder.phases().last(), Some(&BusyPhase::Done));
    assert!(!gate.busy().cancel_requested());
}

#[test]
fn times_out_as_folder_busy() {
    let fixture = fixture();
    let before = support::tree_listing(&fixture.paks);
    let gate = quick_gate(&fixture);
    let recorder = record(gate.busy());
    let ops = [fixture.op("Mods/B", "Mods/B2", true)];
    let fault = FaultPlan {
        busy: busy_fault(0, 50, false),
        folder_policy: wait_override(5, 250),
        ..FaultPlan::default()
    };
    let error = batch_error(run(&fixture, &gate, &ops, fault));
    assert!(
        matches!(error.cause, FsError::FolderBusy { .. }),
        "{error:?}"
    );
    assert_eq!(error.rolled_back, 0);
    assert_eq!(support::tree_listing(&fixture.paks), before);
    assert_eq!(recorder.phases().first(), Some(&BusyPhase::Waiting));
    assert_eq!(recorder.phases().last(), Some(&BusyPhase::TimedOut));
}

#[test]
fn waits_then_succeeds() {
    let fixture = fixture();
    let gate = quick_gate(&fixture);
    let recorder = record(gate.busy());
    let ops = [fixture.op("Mods/B", "Mods/B2", true)];
    let fault = FaultPlan {
        busy: busy_fault(0, 3, false),
        folder_policy: wait_override(20, 2000),
        ..FaultPlan::default()
    };
    run(&fixture, &gate, &ops, fault).expect("the wait ends");
    assert!(fixture.path("Mods/B2/B.pak").exists());
    let phases = recorder.phases();
    assert_eq!(phases.last(), Some(&BusyPhase::Done), "{phases:?}");
    assert!(
        phases[..phases.len() - 1]
            .iter()
            .all(|phase| *phase == BusyPhase::Waiting),
        "{phases:?}"
    );
    assert!(phases.len() >= 2);
    assert_ids_consistent(&recorder);
    let done = recorder.events().pop().expect("done");
    assert_eq!(done.folder, "Mods/B");
    assert!(done.elapsed > Duration::ZERO);
}

#[test]
fn file_renames_never_notify() {
    let fixture = fixture();
    let before = support::tree_listing(&fixture.paks);
    let gate = quick_gate(&fixture);
    let recorder = record(gate.busy());
    let ops = [fixture.op("Mods/A/a.pak", "Mods/A/a.pak-x", false)];
    let fault = FaultPlan {
        busy: busy_fault(0, 3, false),
        folder_policy: wait_override(20, 2000),
        ..FaultPlan::default()
    };
    // AccessDenied on a file is not retried on either OS.
    assert!(run(&fixture, &gate, &ops, fault).is_err());
    assert!(recorder.events().is_empty());
    assert_eq!(support::tree_listing(&fixture.paks), before);
}

#[test]
fn default_policy_per_os() {
    let fixture = fixture();
    let before = support::tree_listing(&fixture.paks);
    let gate = fixture.gate();
    let recorder = record(gate.busy());
    let ops = [fixture.op("Mods/B", "Mods/B2", true)];
    let fault = FaultPlan {
        busy: busy_fault(0, 1, false),
        ..FaultPlan::default()
    };
    let result = run(&fixture, &gate, &ops, fault);
    match os_kind() {
        OsKind::Windows => {
            result.expect("one retry after half a second");
            assert!(fixture.path("Mods/B2/B.pak").exists());
        }
        _ => {
            let error = batch_error(result);
            assert!(
                matches!(error.cause, FsError::FolderBusy { .. }),
                "{error:?}"
            );
            assert_eq!(support::tree_listing(&fixture.paks), before);
        }
    }
    assert!(recorder.events().is_empty());
}

#[test]
fn cancel_of_an_ended_wait_is_false() {
    let fixture = fixture();
    let gate = quick_gate(&fixture);
    let recorder = record(gate.busy());
    let fault = || FaultPlan {
        busy: busy_fault(0, 2, false),
        folder_policy: wait_override(20, 2000),
        ..FaultPlan::default()
    };
    run(
        &fixture,
        &gate,
        &[fixture.op("Mods/B", "Mods/B2", true)],
        fault(),
    )
    .expect("first wait");
    let first = recorder.events().first().expect("an event").wait_id;
    assert!(!gate.busy().cancel(first));
    assert!(!gate.busy().canceller().cancel(first));
    run(
        &fixture,
        &gate,
        &[fixture.op("Mods/X", "Mods/X2", true)],
        fault(),
    )
    .expect("second wait");
    let second = recorder.events().last().expect("an event").wait_id;
    assert!(second > first, "{first} then {second}");
    assert!(!gate.busy().cancel(second));
    assert!(!gate.busy().cancel(0));
}

#[test]
fn notice_waits_for_threshold() {
    let fixture = fixture();
    let gate = fixture.gate();
    gate.busy().set_notice_after(Duration::from_millis(400));
    let recorder = record(gate.busy());
    let fault = FaultPlan {
        busy: busy_fault(0, 2, false),
        folder_policy: wait_override(20, 2000),
        ..FaultPlan::default()
    };
    run(
        &fixture,
        &gate,
        &[fixture.op("Mods/B", "Mods/B2", true)],
        fault,
    )
    .expect("a short wait");
    assert!(recorder.events().is_empty(), "{:?}", recorder.events());
}

#[test]
fn cancel_is_sticky_after_finish() {
    let fixture = fixture();
    let gate = quick_gate(&fixture);
    let recorder = cancel_on(gate.busy(), BusyPhase::Waiting);
    let fault = FaultPlan {
        busy: busy_fault(0, 50, false),
        folder_policy: wait_override(60, 3000),
        ..FaultPlan::default()
    };
    {
        let guard = gate.write(fixture.scope()).expect("write guard");
        let result = guard.rename_batch(&[fixture.op("Mods/B", "Mods/B2", true)], &normal(fault));
        assert!(matches!(
            batch_error(result).cause,
            FsError::BusyCancelled { .. }
        ));
        assert_eq!(recorder.cancels().first(), Some(&true));
        assert!(
            gate.busy().cancel_requested(),
            "sticky after the wait ended"
        );
    }
    assert!(
        gate.busy().cancel_requested(),
        "only a new write guard clears it"
    );
    let guard = gate.write(fixture.scope()).expect("write guard");
    assert!(!gate.busy().cancel_requested());
    drop(guard);
}
