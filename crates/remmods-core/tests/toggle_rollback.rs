//! Toggle rollback (spec §8.2): a batch that fails part-way undoes every rename. The failure is an
//! injected fault at the third op, so it runs on both OSes; the real share-mode lock is in
//! `platform_windows_fs.rs`.
#![allow(clippy::expect_used)]

mod mods_support;
mod support;

use mods_support::toggle::ToggleFixture;
use mods_support::{DATA, input};
use remmods_core::mods::{list_toggle_files, plan_row};
use remmods_core::safe_fs::{BatchOptions, FaultPlan, FsError, RetryMode, Scope};

const LILI: &str = "Standard:Lili";

/// `Mods/Lili/body.{pak,ucas,utoc}`, every file live or every file switched off.
fn lili(live: bool) -> ToggleFixture {
    let suffix = if live { "" } else { "-x" };
    let files: Vec<String> = ["pak", "ucas", "utoc"]
        .iter()
        .map(|extension| format!("Mods/Lili/body.{extension}{suffix}"))
        .collect();
    let entries: Vec<(&str, &[u8])> = files.iter().map(|rel| (rel.as_str(), DATA)).collect();
    ToggleFixture::new(&entries)
}

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

fn assert_rolled_back(fixture: &ToggleFixture, enable: bool) {
    let before = fixture.names();
    let error = toggle_failing_at_third_op(fixture, LILI, enable);
    assert!(matches!(error, FsError::Batch(_)), "{error:?}");
    assert_eq!(fixture.names(), before, "every rename is undone");
}

#[test]
fn failed_batch_rolls_back_in_both_directions() {
    for (live, enable) in [(true, false), (false, true)] {
        let fixture = lili(live);
        assert_rolled_back(&fixture, enable);
        let pak = if live {
            "Mods/Lili/body.pak"
        } else {
            "Mods/Lili/body.pak-x"
        };
        assert!(fixture.exists(pak), "{pak} after enable={enable}");
    }
}

/// The same three companions below `Content/Paks` go through one batch without a fault.
#[test]
fn nested_companions_switch_off_together() {
    let nested =
        ["pak", "ucas", "utoc"].map(|extension| format!("Mods/Lili/Content/Paks/body.{extension}"));
    let entries: Vec<(&str, &[u8])> = nested.iter().map(|rel| (rel.as_str(), DATA)).collect();
    let fixture = ToggleFixture::new(&entries);
    fixture.toggle(&[LILI], false).expect("disable");
    for rel in &nested {
        assert!(fixture.exists(&format!("{rel}-x")), "{rel}-x");
        assert!(!fixture.exists(rel), "{rel}");
    }
}
