//! Ports of ORIG `ModServiceRollbackTests` (spec §8.2). The "one file is locked" pair runs on
//! both OSes through an injected failure at the third op; the real share-mode lock is in
//! `platform_windows_fs.rs`.
#![allow(clippy::expect_used)]

mod mods_support;
mod support;

use mods_support::toggle::ToggleFixture;
use mods_support::{DATA, input};
use remmods_core::mods::{list_toggle_files, plan_row};
use remmods_core::safe_fs::{BatchOptions, FaultPlan, FsError, RetryMode, Scope};

/// Plans the row like a toggle does and runs the batch with the third op failing.
fn toggle_failing_at_third_op(fixture: &ToggleFixture, key: &str, enable: bool) -> FsError {
    let row = fixture.row(key);
    let scan_input = input(&fixture.paks);
    let files = list_toggle_files(&row, &scan_input).expect("files");
    let plan = plan_row(&row, &files, enable, None).expect("plan");
    assert_eq!(plan.ops.len(), 3);
    let app_dirs = [fixture.paths.data_dir.as_path()];
    let scope = Scope::for_paks(&fixture.paks, &app_dirs).expect("scope");
    let guard = fixture.core.ops.write(scope).expect("write guard");
    let options = BatchOptions {
        label: "toggle",
        retry: RetryMode::Never,
        fault: Some(FaultPlan {
            fail_before: Some(2),
            ..FaultPlan::default()
        }),
    };
    guard
        .rename_batch(&plan.ops, &options)
        .expect_err("injected failure")
}

#[test]
fn deactivate_leaves_the_mod_untouched_when_one_file_is_locked() {
    let fixture = ToggleFixture::new(&[
        ("Mods/mod/a.pak", DATA),
        ("Mods/mod/b.ucas", DATA),
        ("Mods/mod/z.utoc", DATA),
    ]);
    let before = fixture.names();
    let error = toggle_failing_at_third_op(&fixture, "Standard:mod", false);
    assert!(matches!(error, FsError::Batch(_)), "{error:?}");
    assert_eq!(fixture.names(), before);
}

#[test]
fn activate_leaves_the_mod_untouched_when_one_file_is_locked() {
    let fixture = ToggleFixture::new(&[
        ("Mods/mod/a.pak-x", DATA),
        ("Mods/mod/b.ucas-x", DATA),
        ("Mods/mod/z.utoc-x", DATA),
    ]);
    let before = fixture.names();
    let error = toggle_failing_at_third_op(&fixture, "Standard:mod", true);
    assert!(matches!(error, FsError::Batch(_)), "{error:?}");
    assert_eq!(fixture.names(), before);
}

#[test]
fn deactivate_still_works_when_nothing_is_locked() {
    let fixture = ToggleFixture::new(&[("Mods/mod/a.pak", DATA), ("Mods/mod/z.utoc", DATA)]);
    fixture.toggle(&["Standard:mod"], false).expect("disable");
    assert!(fixture.exists("Mods/mod/a.pak-x"));
    assert!(fixture.exists("Mods/mod/z.utoc-x"));
}
