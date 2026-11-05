//! The update seal (SPEC-P5 §5.2, §9.1 S1–S9): refusals before the seal, mutations refused while
//! sealed, scans still working, and the app lock held across processes. S10 and S11 test the
//! crate-private install slot and live in `install/service.rs`.
#![allow(clippy::expect_used, clippy::disallowed_methods)]

mod archive_support;
mod install_support;
mod mods_support;
mod support;

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use install_support::{InstallFixture, fixture, install_error, journal_entries};
use mods_support::DATA;
use mods_support::manage::tree;
use remmods_core::app_update::{AppUpdateError, UpdateBlock};
use remmods_core::game::GameOptions;
use remmods_core::install::{InstallError, InstallHooks};
use remmods_core::mods::{ModsError, RenameRequest};
use remmods_core::profiles::{
    ApplyDirection, ApplyRequest, ProfileError, ProfileMode, ProfileSave,
};
use remmods_core::safe_fs::{FaultPlan, FsError, OpGate, Scope};
use remmods_core::{AppPaths, Core, CoreError, CoreOptions};

const BASE: &[(&str, &[u8])] = &[("Mods/A/A.pak", DATA), ("Mods/B/B.pak", DATA)];
const COOL: &[(&str, &[u8])] = &[("Cool/x.pak", b"pak")];

fn scope_of(fixture: &InstallFixture) -> Scope {
    let app_dirs = [
        fixture.fx.paths.data_dir.as_path(),
        fixture.fx.paths.config_dir.as_path(),
    ];
    Scope::for_paks(&fixture.fx.paks, &app_dirs).expect("scope")
}

fn blocked(result: Result<(), CoreError>) -> UpdateBlock {
    match result {
        Err(CoreError::AppUpdate(AppUpdateError::Blocked(block))) => block,
        other => panic!("expected an update block, got {other:?}"),
    }
}

/// Busy as each command reports it: from the gate directly or wrapped by its module.
fn is_busy(error: &CoreError) -> bool {
    matches!(
        error,
        CoreError::Fs(FsError::Busy)
            | CoreError::Mods(ModsError::Fs(FsError::Busy))
            | CoreError::Profiles(ProfileError::Fs(FsError::Busy))
            | CoreError::Install(InstallError::Busy)
    )
}

/// A rename of B that stops after its first op: the live session's journal entry stays.
fn crash_a_rename(fixture: &InstallFixture) {
    fixture.fx.core.inject_batch_fault(FaultPlan {
        crash_after: Some(1),
        ..FaultPlan::default()
    });
    let renamed = fixture.fx.core.mods_rename(&RenameRequest {
        target: fixture.fx.target("Standard:B"),
        new_name: "Renamed".to_owned(),
        apply_while_running: false,
    });
    assert!(renamed.is_err());
    assert_eq!(journal_entries(&fixture.fx.paths.data_dir).len(), 1);
}

#[test]
fn s1_a_running_install_job_blocks_the_seal() {
    let fixture = fixture(BASE);
    let zip = fixture.zip("Cool.zip", COOL);
    let inspection = fixture.inspect(&zip);
    let (staged_tx, staged_rx) = mpsc::channel::<()>();
    let (go_tx, go_rx) = mpsc::channel::<()>();
    let staged_tx = std::sync::Mutex::new(staged_tx);
    let go_rx = std::sync::Mutex::new(go_rx);
    fixture.fx.core.inject_install_hooks(InstallHooks {
        after_stage: Some(std::sync::Arc::new(move |_| {
            staged_tx.lock().expect("lock").send(()).expect("staged");
            go_rx.lock().expect("lock").recv().expect("go");
        })),
        ..InstallHooks::default()
    });
    thread::scope(|scope| {
        let job = scope.spawn(|| fixture.start(&inspection));
        staged_rx.recv().expect("job staged");
        let core = &fixture.fx.core;
        assert_eq!(blocked(core.seal_for_update()), UpdateBlock::InstallRunning);
        assert_eq!(
            blocked(core.update_preflight()),
            UpdateBlock::InstallRunning
        );
        assert!(!core.is_sealed_for_update());
        go_tx.send(()).expect("release the job");
        job.join()
            .expect("join")
            .expect("the job was not cancelled");
    });
    assert!(fixture.exists("Mods/Cool/x.pak"));
    assert!(journal_entries(&fixture.fx.paths.data_dir).is_empty());
    assert!(!fixture.fx.core.install_cancel());
}

#[test]
fn s2_a_pending_entry_of_the_live_session_blocks_the_seal() {
    let fixture = fixture(BASE);
    crash_a_rename(&fixture);
    let before = tree(&fixture.fx);
    let entries = journal_entries(&fixture.fx.paths.data_dir);
    let core = &fixture.fx.core;
    assert_eq!(
        blocked(core.update_preflight()),
        UpdateBlock::JournalPending
    );
    assert_eq!(blocked(core.seal_for_update()), UpdateBlock::JournalPending);
    assert!(!core.is_sealed_for_update());
    // The install slot was released again: the second attempt is not InstallRunning.
    assert_eq!(blocked(core.seal_for_update()), UpdateBlock::JournalPending);
    assert_eq!(tree(&fixture.fx), before);
    assert_eq!(journal_entries(&fixture.fx.paths.data_dir), entries);
}

#[test]
fn s3_a_dead_owners_entry_is_rolled_back_first() {
    let mut fixture = fixture(BASE);
    let before = tree(&fixture.fx);
    crash_a_rename(&fixture);
    assert_ne!(tree(&fixture.fx), before);
    let fresh = fixture.fx.second_core();
    drop(std::mem::replace(&mut fixture.fx.core, fresh));

    fixture.fx.core.update_preflight().expect("preflight");
    assert_eq!(tree(&fixture.fx), before);
    assert!(journal_entries(&fixture.fx.paths.data_dir).is_empty());

    let mut second = install_support::fixture(BASE);
    let second_before = tree(&second.fx);
    crash_a_rename(&second);
    let fresh = second.fx.second_core();
    drop(std::mem::replace(&mut second.fx.core, fresh));
    second.fx.core.seal_for_update().expect("seal");
    assert!(second.fx.core.is_sealed_for_update());
    assert_eq!(tree(&second.fx), second_before);
    assert!(journal_entries(&second.fx.paths.data_dir).is_empty());
}

#[test]
fn s4_s5_sealed_refuses_mutations_but_scans_and_unseal_reopens() {
    let fixture = fixture(BASE);
    let profile = fixture
        .fx
        .core
        .profiles_save(&ProfileSave::New {
            name: "P".to_owned(),
            description: String::new(),
            mode: ProfileMode::Exclusive,
        })
        .expect("save profile")
        .profiles[0]
        .profile
        .id
        .clone();
    let diff = fixture
        .fx
        .core
        .profiles_diff(&profile, ApplyDirection::On)
        .expect("diff");
    let zip = fixture.zip("Cool.zip", COOL);
    let inspection = fixture.inspect(&zip);
    let rename = RenameRequest {
        target: fixture.fx.target("Standard:B"),
        new_name: "Renamed".to_owned(),
        apply_while_running: false,
    };
    let before = fixture.state();

    fixture.fx.core.seal_for_update().expect("seal");
    let core = &fixture.fx.core;
    let toggled = fixture
        .fx
        .toggle(&["Standard:A"], false)
        .expect_err("toggle");
    assert!(is_busy(&toggled), "{toggled:?}");
    let renamed = core.mods_rename(&rename).expect_err("rename");
    assert!(is_busy(&renamed), "{renamed:?}");
    let applied = core
        .profiles_apply(&ApplyRequest {
            profile_id: profile,
            fingerprint: diff.fingerprint,
            apply_while_running: false,
            direction: ApplyDirection::On,
        })
        .expect_err("apply");
    assert!(is_busy(&applied), "{applied:?}");
    assert!(matches!(
        install_error(fixture.start(&inspection)),
        InstallError::Busy
    ));
    assert_eq!(fixture.state(), before);
    let listed = core.mods_list().expect("scans keep working");
    assert!(
        listed
            .content
            .rows
            .iter()
            .any(|row| row.key == "Standard:A")
    );
    fixture.fx.snapshot();

    core.unseal_after_update();
    assert!(!core.is_sealed_for_update());
    fixture
        .fx
        .toggle(&["Standard:A"], false)
        .expect("toggle after unseal");
    assert!(fixture.exists("Mods/A/A.pak-x"));
}

#[test]
fn s6_the_seal_waits_for_a_running_mutation() {
    let fixture = fixture(BASE);
    let written = fixture.fx.paths.data_dir.join("s6.json");
    let (held_tx, held_rx) = mpsc::channel::<()>();
    let started = thread::scope(|scope| {
        let core = &fixture.fx.core;
        let scope_value = scope_of(&fixture);
        let written = &written;
        scope.spawn(move || {
            let guard = core.ops.write(scope_value).expect("write guard");
            guard.write_json(written, b"{}\n").expect("write");
            held_tx.send(()).expect("held");
            thread::sleep(Duration::from_millis(300));
            drop(guard);
        });
        held_rx.recv().expect("guard held");
        let started = Instant::now();
        core.seal_for_update().expect("seal");
        started
    });
    assert!(started.elapsed() >= Duration::from_millis(200));
    assert!(fixture.fx.core.is_sealed_for_update());
    assert_eq!(std::fs::read(&written).expect("written"), b"{}\n");
}

#[test]
fn s7_the_sealed_gate_holds_the_app_lock_across_processes() {
    let fixture = fixture(BASE);
    fixture.fx.core.seal_for_update().expect("seal");
    let other = OpGate::open(&fixture.fx.paths).expect("second gate");
    assert!(matches!(
        other.write(scope_of(&fixture)),
        Err(FsError::Busy)
    ));
    assert!(matches!(other.read(), Err(FsError::Busy)));
    drop(other);
    fixture.fx.core.unseal_after_update();
    let other = OpGate::open(&fixture.fx.paths).expect("third gate");
    drop(other.write(scope_of(&fixture)).expect("free after unseal"));
}

#[test]
fn s8_without_a_game_the_app_scope_is_used() {
    let (_dir, root) = support::temp_root();
    let options = CoreOptions {
        save_root: None,
        game: GameOptions {
            steam_detection: false,
            fixed_location: None,
            steam_hints: None,
            processes: None,
        },
        ..CoreOptions::default()
    };
    let core = Core::open_with(AppPaths::under(&root.join("app")), options).expect("core");
    core.update_preflight().expect("preflight");
    core.seal_for_update().expect("seal");
    assert!(core.is_sealed_for_update());
    core.unseal_after_update();
    assert!(!core.is_sealed_for_update());
}

#[test]
fn s9_seal_gate_with_an_entry_seals_nothing() {
    let fixture = fixture(BASE);
    crash_a_rename(&fixture);
    let core = &fixture.fx.core;
    let guard = core.ops.write(scope_of(&fixture)).expect("guard");
    assert!(!guard.seal_gate().expect("seal_gate"));
    assert!(!core.ops.is_sealed());
    drop(
        core.ops
            .write(scope_of(&fixture))
            .expect("the guard was released"),
    );
    drop(core.ops.read().expect("reads work"));
    assert_eq!(journal_entries(&fixture.fx.paths.data_dir).len(), 1);
}
