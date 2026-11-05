//! Folder-busy waits against real Windows locks (P3b §5.7 WB1–WB2): the real folder policy,
//! `RetryMode::Normal`, a file held inside the folder.
#![cfg(windows)]
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod mods_support;
mod support;

use std::fs::{File, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;
use std::thread;
use std::time::Duration;

use mods_support::busy::{cancel_on, record};
use remmods_core::platform;
use remmods_core::safe_fs::{BatchOptions, BusyPhase, FsError, RenameOp, RetryMode};
use support::FsFixture;
use windows_sys::Win32::Storage::FileSystem::{
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
};

fn hold(path: &Path, share: u32) -> File {
    OpenOptions::new()
        .read(true)
        .share_mode(share)
        .open(platform::fs_path(path).expect("fs path"))
        .expect("open for holding")
}

fn held_fixture() -> (FsFixture, File) {
    let fixture = support::fs_fixture();
    support::write_tree(&fixture.paks, &[("LogicMods/X/X.pak", b"x")]);
    let held = hold(
        &fixture.path("LogicMods/X/X.pak"),
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
    );
    (fixture, held)
}

fn disable_ops(fixture: &FsFixture) -> [RenameOp; 2] {
    [
        fixture.op("LogicMods/X", "LogicMods/.disabled_X", true),
        fixture.op(
            "LogicMods/.disabled_X/X.pak",
            "LogicMods/.disabled_X/X.pak-x",
            false,
        ),
    ]
}

fn normal() -> BatchOptions {
    BatchOptions {
        label: "test",
        retry: RetryMode::Normal,
        fault: None,
    }
}

#[test]
fn real_holder_cancel_rolls_back() {
    let (fixture, held) = held_fixture();
    let before = fixture.paks_snapshot();
    let gate = fixture.gate();
    gate.busy().set_notice_after(Duration::from_millis(200));
    let recorder = cancel_on(gate.busy(), BusyPhase::Waiting);
    let guard = gate.write(fixture.scope()).expect("write guard");

    let result = guard.rename_batch(&disable_ops(&fixture), &normal());

    match result {
        Err(FsError::Batch(error)) => {
            assert!(
                matches!(error.cause, FsError::BusyCancelled { .. }),
                "{}",
                error.cause
            );
            assert!(error.stuck.is_empty());
        }
        other => panic!("expected a cancelled wait, got {other:?}"),
    }
    drop(guard);
    drop(held);
    assert_eq!(fixture.paks_snapshot(), before);
    assert_eq!(support::journal_entries(&fixture.journal_path()), 0);
    let phases = recorder.phases();
    assert_eq!(phases.first(), Some(&BusyPhase::Waiting), "{phases:?}");
    assert_eq!(phases.last(), Some(&BusyPhase::Cancelled), "{phases:?}");
    assert_eq!(recorder.cancels().first(), Some(&true));
}

#[test]
fn real_holder_wait_then_done() {
    let (fixture, held) = held_fixture();
    let gate = fixture.gate();
    gate.busy().set_notice_after(Duration::from_millis(200));
    let recorder = record(gate.busy());
    let releaser = thread::spawn(move || {
        thread::sleep(Duration::from_millis(1500));
        drop(held);
    });
    let guard = gate.write(fixture.scope()).expect("write guard");

    let report = guard.rename_batch(&disable_ops(&fixture), &normal());

    releaser.join().expect("releaser");
    report.expect("the folder is free after the holder closes it");
    assert!(fixture.path("LogicMods/.disabled_X/X.pak-x").is_file());
    let events = recorder.events();
    let phases = recorder.phases();
    assert_eq!(phases.first(), Some(&BusyPhase::Waiting), "{phases:?}");
    assert_eq!(phases.last(), Some(&BusyPhase::Done), "{phases:?}");
    assert!(events.iter().all(|event| event.folder == "LogicMods/X"));
    assert!(events.iter().all(|event| event.cancellable));
    assert!(
        events
            .iter()
            .all(|event| event.limit == Duration::from_secs(30))
    );
}
