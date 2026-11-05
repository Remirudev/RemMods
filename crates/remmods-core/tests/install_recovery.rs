//! Crashes during an install (P4 spec §8.2 D9, D10): the next session's write guard rolls the
//! commit batch back and removes the dead owner's staging folder.
#![allow(clippy::expect_used, clippy::disallowed_methods)]

mod archive_support;
mod install_support;
mod mods_support;
mod support;

use std::io::Write;

use install_support::{fixture, journal_entries};
use mods_support::manage::HEX;
use remmods_core::safe_fs::{FaultPlan, Scope};

fn scope_of(fixture: &install_support::InstallFixture) -> Scope {
    let app_dirs = [
        fixture.fx.paths.data_dir.as_path(),
        fixture.fx.paths.config_dir.as_path(),
    ];
    Scope::for_paks(&fixture.fx.paks, &app_dirs).expect("scope")
}

#[test]
fn d9_crash_during_commit_is_rolled_back() {
    // Set-aside folder, a.pak → -x, staging → Cool, x.pak-x → x.pak.
    for k in 0..=4 {
        let mut fixture = fixture(&[("Mods/Cool/a.pak", b"old")]);
        let zip = fixture.zip("Cool.zip", &[("Cool/x.pak", b"pak")]);
        let inspection = fixture.inspect(&zip);
        let before = fixture.state();
        fixture.fx.core.inject_batch_fault(FaultPlan {
            crash_after: Some(k),
            ..FaultPlan::default()
        });
        assert!(fixture.start(&inspection).is_err(), "crash after {k}");
        let fresh = fixture.fx.second_core();
        drop(std::mem::replace(&mut fixture.fx.core, fresh));
        let report = {
            let guard = fixture
                .fx
                .core
                .ops
                .write(scope_of(&fixture))
                .expect("guard");
            guard.recovery().clone()
        };
        assert_eq!(report.discarded, 1, "crash after {k}: {report:?}");
        assert_eq!(report.stuck, 0, "crash after {k}: {report:?}");
        assert_eq!(fixture.state(), before, "crash after {k}");
        assert!(fixture.temp_folders("Mods").is_empty());
    }
}

#[test]
fn d10_crash_during_extraction_discards_staging() {
    let mut fixture = fixture(&[("Mods/Other/o.pak", b"o")]);
    let before = fixture.state();
    {
        let guard = fixture
            .fx
            .core
            .ops
            .write(scope_of(&fixture))
            .expect("guard");
        let staging = guard
            .create_staging(&fixture.fx.paks.join("Mods"), HEX)
            .expect("staging");
        let mut file = staging
            .create_file(&["Cool".to_owned(), "x.pak-x".to_owned()])
            .expect("staged file");
        file.write_all(b"partial").expect("write");
        drop(file);
        drop(guard);
        drop(staging);
    }
    assert_eq!(journal_entries(&fixture.fx.paths.data_dir).len(), 1);
    let fresh = fixture.fx.second_core();
    drop(std::mem::replace(&mut fixture.fx.core, fresh));
    let report = {
        let guard = fixture
            .fx
            .core
            .ops
            .write(scope_of(&fixture))
            .expect("guard");
        guard.recovery().clone()
    };
    assert_eq!(report.discarded, 1);
    assert!(journal_entries(&fixture.fx.paths.data_dir).is_empty());
    assert_eq!(fixture.state(), before);
}
