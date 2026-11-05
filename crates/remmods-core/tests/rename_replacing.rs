//! `WriteGuard::rename_batch_replacing` (P4 spec §8.2 D20): a target vacated by an earlier op of
//! the same batch is allowed; `rename_batch` keeps refusing it.
#![allow(clippy::expect_used)]

mod support;

use remmods_core::safe_fs::FsError;
use support::{FsFixture, fail_before, fs_fixture, options, write_tree};

const ASIDE: &str = "Mods/_Cool_tmp_0123456789abcdef0123456789abcdef";
const STAGING: &str = "Mods/_install_tmp_0123456789abcdef0123456789abcdef";

fn with_old_and_staging(old: &str) -> FsFixture {
    let fixture = fs_fixture();
    write_tree(
        &fixture.paks,
        &[
            (&format!("Mods/{old}/a.pak"), b"old"),
            (&format!("{STAGING}/x.pak-x"), b"new"),
        ],
    );
    fixture
}

#[test]
fn d20_vacated_target_passes_and_rolls_back() {
    let fixture = with_old_and_staging("Cool");
    let gate = fixture.gate();
    let ops = [
        fixture.op("Mods/Cool", ASIDE, true),
        fixture.op(STAGING, "Mods/Cool", true),
    ];
    let before = fixture.paks_snapshot();
    let guard = gate.write(fixture.scope()).expect("guard");
    let failed = guard.rename_batch_replacing(&ops, &options(fail_before(1)));
    assert!(failed.is_err());
    assert_eq!(fixture.paks_snapshot(), before);
    let refused = guard.rename_batch(&ops, &options(None));
    assert!(matches!(
        refused,
        Err(FsError::InvalidBatch { reason: "chained" })
    ));
    guard
        .rename_batch_replacing(&ops, &options(None))
        .expect("replacing batch");
    assert!(fixture.path("Mods/Cool/x.pak-x").exists());
    assert!(fixture.path(&format!("{ASIDE}/a.pak")).exists());
}

#[test]
fn d20_a_later_ops_source_is_still_rejected() {
    let fixture = with_old_and_staging("Cool");
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("guard");
    let ops = [
        fixture.op(STAGING, "Mods/Cool", true),
        fixture.op("Mods/Cool", ASIDE, true),
    ];
    let refused = guard.rename_batch_replacing(&ops, &options(None));
    assert!(
        matches!(refused, Err(FsError::InvalidBatch { .. })),
        "{refused:?}"
    );
}

#[test]
fn d20_case_variant_replace() {
    let fixture = with_old_and_staging("cool");
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("guard");
    let ops = [
        fixture.op("Mods/cool", ASIDE, true),
        fixture.op(STAGING, "Mods/Cool", true),
    ];
    guard
        .rename_batch_replacing(&ops, &options(None))
        .expect("case variant replace");
    assert_eq!(
        support::names_in(&fixture.path("Mods")),
        vec![
            "Cool".to_owned(),
            ASIDE.trim_start_matches("Mods/").to_owned()
        ]
    );
}
