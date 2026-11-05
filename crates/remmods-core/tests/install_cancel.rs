//! One install at a time, cancel, and a game folder switched mid-install (P4 spec §8.2 D11, D12,
//! D22). The `after_stage` hook parks the job until the test thread has acted.
#![allow(clippy::expect_used, clippy::disallowed_methods)]

mod archive_support;
mod install_support;
mod mods_support;
mod support;

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use install_support::{InstallFixture, fixture, install_error, journal_entries};
use remmods_core::game::LocationChoice;
use remmods_core::install::{InstallError, InstallHooks};

const COOL: &[(&str, &[u8])] = &[("Cool/x.pak", b"pak")];

/// A hook that reports "staged" and waits for "go".
fn parking_hook(fixture: &InstallFixture) -> (Receiver<()>, Sender<()>) {
    let (staged_tx, staged_rx) = mpsc::channel::<()>();
    let (go_tx, go_rx) = mpsc::channel::<()>();
    let staged_tx = Mutex::new(staged_tx);
    let go_rx = Mutex::new(go_rx);
    fixture.fx.core.inject_install_hooks(InstallHooks {
        after_stage: Some(Arc::new(move |_| {
            staged_tx
                .lock()
                .expect("lock")
                .send(())
                .expect("report staged");
            go_rx.lock().expect("lock").recv().expect("wait for go");
        })),
        ..InstallHooks::default()
    });
    (staged_rx, go_tx)
}

#[test]
fn d11_d12_busy_then_cancel() {
    let fixture = fixture(&[("Mods/Other/o.pak", b"o")]);
    let zip = fixture.zip("Cool.zip", COOL);
    let inspection = fixture.inspect(&zip);
    let before = fixture.state();
    let (staged, go) = parking_hook(&fixture);
    thread::scope(|scope| {
        let first = scope.spawn(|| fixture.start(&inspection));
        staged.recv().expect("first job staged");
        let second = install_error(fixture.start(&inspection));
        assert!(matches!(second, InstallError::Busy));
        assert!(fixture.fx.core.install_cancel());
        go.send(()).expect("release");
        let first = install_error(first.join().expect("join"));
        assert!(matches!(first, InstallError::Cancelled));
    });
    assert!(!fixture.fx.core.install_cancel());
    assert_eq!(fixture.state(), before);
    assert!(fixture.temp_folders("Mods").is_empty());
    fixture
        .fx
        .core
        .inject_install_hooks(InstallHooks::default());
    fixture.start(&inspection).expect("the job slot is free");
    assert!(fixture.exists("Mods/Cool/x.pak"));
}

#[test]
fn d22_game_folder_switched_during_extraction() {
    let fixture = fixture(&[("Mods/Other/o.pak", b"o")]);
    let other = install_support::fixture(&[("Mods/Else/e.pak", b"e")]);
    let zip = fixture.zip("Cool.zip", COOL);
    let inspection = fixture.inspect(&zip);
    let before = fixture.state();
    let other_before = other.state();
    let (staged, go) = parking_hook(&fixture);
    thread::scope(|scope| {
        let job = scope.spawn(|| fixture.start(&inspection));
        staged.recv().expect("staged");
        fixture
            .fx
            .core
            .game_set_location(LocationChoice::Path {
                path: other.fx.game_root.clone(),
            })
            .expect("switch the game folder");
        go.send(()).expect("release");
        let error = install_error(job.join().expect("join"));
        assert!(
            matches!(
                error,
                InstallError::Blocked {
                    reason: remmods_core::install::InstallBlock::TargetChanged
                }
            ),
            "{error:?}"
        );
    });
    assert_eq!(fixture.state().tree, before.tree);
    assert!(fixture.temp_folders("Mods").is_empty());
    assert!(journal_entries(&fixture.fx.paths.data_dir).is_empty());
    assert_eq!(other.state().tree, other_before.tree);
}
