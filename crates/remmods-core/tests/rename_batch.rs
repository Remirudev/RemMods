#![allow(clippy::expect_used)]

mod support;

use remmods_core::safe_fs::{
    BatchError, FaultPlan, FsError, RenameMethod, RenameOp, ScopeViolation, StuckOp,
    link_unlink_noreplace, placeholder_noreplace,
};

use support::{FsFixture, fail_before, options};

fn pak_mod() -> FsFixture {
    let fixture = support::fs_fixture();
    support::write_tree(
        &fixture.paks,
        &[
            ("Mods/Cool/Cool.pak", b"pak"),
            ("Mods/Cool/Cool.ucas", b"ucas"),
            ("Mods/Cool/Cool.utoc", b"utoc"),
            ("LogicMods/X/X.pak", b"xpak"),
            ("LogicMods/X/X.ucas", b"xucas"),
        ],
    );
    fixture
}

fn disable_cool(fixture: &FsFixture) -> Vec<RenameOp> {
    ["pak", "ucas", "utoc"]
        .iter()
        .map(|ext| {
            fixture.op(
                &format!("Mods/Cool/Cool.{ext}"),
                &format!("Mods/Cool/Cool.{ext}-x"),
                false,
            )
        })
        .collect()
}

fn disable_logic(fixture: &FsFixture) -> Vec<RenameOp> {
    vec![
        fixture.op("LogicMods/X", "LogicMods/.disabled_X", true),
        fixture.op(
            "LogicMods/.disabled_X/X.pak",
            "LogicMods/.disabled_X/X.pak-x",
            false,
        ),
        fixture.op(
            "LogicMods/.disabled_X/X.ucas",
            "LogicMods/.disabled_X/X.ucas-x",
            false,
        ),
    ]
}

fn batch_error(result: Result<impl std::fmt::Debug, FsError>) -> BatchError {
    match result {
        Err(FsError::Batch(error)) => *error,
        other => panic!("expected a batch error, got {other:?}"),
    }
}

/// Runs `ops` expecting a preflight failure; checks that nothing changed and no journal exists.
fn assert_refused(fixture: &FsFixture, ops: &[RenameOp]) -> FsError {
    let before = fixture.paks_snapshot();
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");

    let error = guard
        .rename_batch(ops, &options(None))
        .expect_err("refused");

    assert_eq!(fixture.paks_snapshot(), before);
    assert!(!fixture.journal_path().exists(), "journal file created");
    error
}

#[test]
fn a_batch_renames_every_file_atomically_and_clears_the_journal() {
    let fixture = pak_mod();
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");

    let report = guard
        .rename_batch(&disable_cool(&fixture), &options(None))
        .expect("batch");

    assert_eq!(report.done, 3);
    assert_eq!(report.methods, vec![RenameMethod::Atomic; 3]);
    assert_eq!(
        support::names_in(&fixture.path("Mods/Cool")),
        ["Cool.pak-x", "Cool.ucas-x", "Cool.utoc-x"]
    );
    assert_eq!(support::journal_entries(&fixture.journal_path()), 0);
}

#[test]
fn a_folder_op_runs_first_and_later_ops_use_the_new_folder() {
    let fixture = pak_mod();
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");

    guard
        .rename_batch(&disable_logic(&fixture), &options(None))
        .expect("batch");

    assert!(!fixture.path("LogicMods/X").exists());
    assert_eq!(
        support::names_in(&fixture.path("LogicMods/.disabled_X")),
        ["X.pak-x", "X.ucas-x"]
    );
}

#[test]
fn a_missing_source_is_refused_before_anything_changes() {
    let fixture = pak_mod();
    let mut ops = disable_cool(&fixture);
    ops.push(fixture.op("Mods/Cool/Gone.pak", "Mods/Cool/Gone.pak-x", false));

    let error = assert_refused(&fixture, &ops);

    assert!(matches!(error, FsError::SourceMissing { .. }), "{error}");
}

#[test]
fn an_existing_target_is_refused() {
    let fixture = pak_mod();
    support::write_tree(&fixture.paks, &[("Mods/Cool/Cool.utoc-x", b"old")]);

    let error = assert_refused(&fixture, &disable_cool(&fixture));

    assert!(matches!(error, FsError::TargetExists { .. }), "{error}");
}

#[test]
fn a_fold_equal_sibling_counts_as_an_existing_target() {
    let fixture = support::fs_fixture();
    support::write_tree(
        &fixture.paks,
        &[
            ("Mods/Foo/Foo.pak", b"pak"),
            ("Mods/Foo/Foo.PAK-X", b"other"),
        ],
    );
    let ops = [fixture.op("Mods/Foo/Foo.pak", "Mods/Foo/Foo.pak-x", false)];

    let error = assert_refused(&fixture, &ops);

    assert!(matches!(error, FsError::TargetExists { .. }), "{error}");
}

#[test]
fn duplicate_and_chained_targets_are_invalid() {
    let fixture = pak_mod();
    let duplicate = [
        fixture.op("Mods/Cool/Cool.pak", "Mods/Cool/New.pak", false),
        fixture.op("Mods/Cool/Cool.ucas", "Mods/Cool/new.PAK", false),
    ];
    let chained = [
        fixture.op("Mods/Cool/Cool.pak", "Mods/Cool/Cool.ucas", false),
        fixture.op("Mods/Cool/Cool.ucas", "Mods/Cool/Cool.ucas-x", false),
    ];

    let first = assert_refused(&fixture, &duplicate);
    let second = assert_refused(&fixture, &chained);

    assert!(matches!(
        first,
        FsError::InvalidBatch {
            reason: "duplicate_target"
        }
    ));
    assert!(matches!(
        second,
        FsError::InvalidBatch { reason: "chained" }
    ));
}

#[test]
fn a_cross_folder_op_is_invalid() {
    let fixture = pak_mod();
    let ops = [fixture.op("Mods/Cool/Cool.pak", "Mods/Cool.pak", false)];

    let error = assert_refused(&fixture, &ops);

    assert!(matches!(
        error,
        FsError::InvalidBatch {
            reason: "cross_folder"
        }
    ));
}

#[test]
fn an_op_under_a_later_ops_source_violates_the_order() {
    let fixture = pak_mod();
    let ops = [
        fixture.op("LogicMods/X/X.pak", "LogicMods/X/X.pak-x", false),
        fixture.op("LogicMods/X", "LogicMods/.disabled_X", true),
    ];

    let error = assert_refused(&fixture, &ops);

    assert!(matches!(error, FsError::InvalidBatch { reason: "order" }));
}

#[test]
fn an_out_of_scope_path_is_refused() {
    let fixture = pak_mod();
    support::write_tree(&fixture.paks, &[("base.pak", b"base")]);
    let ops = [fixture.op("base.pak", "base.pak-x", false)];

    let error = assert_refused(&fixture, &ops);

    assert!(matches!(
        error,
        FsError::OutOfScope {
            reason: ScopeViolation::OutsideRoots,
            ..
        }
    ));
}

#[test]
fn a_failure_undoes_every_done_op() {
    let fixture = pak_mod();
    let before = fixture.paks_snapshot();
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");

    let error = batch_error(guard.rename_batch(&disable_cool(&fixture), &options(fail_before(2))));

    assert_eq!(error.failed, 2);
    assert_eq!(error.rolled_back, 2);
    assert_eq!(error.stuck, Vec::<StuckOp>::new());
    assert_eq!(fixture.paks_snapshot(), before);
    assert_eq!(support::journal_entries(&fixture.journal_path()), 0);
}

#[test]
fn a_failing_rollback_is_stuck_and_earlier_ops_are_still_undone() {
    let fixture = pak_mod();
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");
    let fault = FaultPlan {
        fail_before: Some(2),
        fail_rollback_of: Some(1),
        ..FaultPlan::default()
    };

    let error = batch_error(guard.rename_batch(&disable_cool(&fixture), &options(Some(fault))));

    assert_eq!(error.rolled_back, 1);
    assert_eq!(
        error.stuck,
        vec![StuckOp {
            from: fixture.path("Mods/Cool/Cool.ucas"),
            to: fixture.path("Mods/Cool/Cool.ucas-x"),
        }]
    );
    assert_eq!(
        support::names_in(&fixture.path("Mods/Cool")),
        ["Cool.pak", "Cool.ucas-x", "Cool.utoc"]
    );
    let journal = support::journal_json(&fixture.journal_path()).expect("journal kept");
    assert_eq!(journal["entries"][0]["stuck"], true);
}

#[test]
fn a_case_only_rename_goes_through_a_temp_name() {
    let fixture = support::fs_fixture();
    support::write_tree(&fixture.paks, &[("Mods/Foo/foo.pak", b"pak")]);
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");
    let ops = [fixture.op("Mods/Foo/foo.pak", "Mods/Foo/Foo.pak", false)];

    let report = guard.rename_batch(&ops, &options(None)).expect("batch");

    assert_eq!(report.done, 2);
    assert_eq!(support::names_in(&fixture.path("Mods/Foo")), ["Foo.pak"]);
}

#[test]
fn a_failure_in_the_second_half_of_a_case_pair_restores_the_old_name() {
    let fixture = support::fs_fixture();
    support::write_tree(&fixture.paks, &[("Mods/Foo/foo.pak", b"pak")]);
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");
    let ops = [fixture.op("Mods/Foo/foo.pak", "Mods/Foo/Foo.pak", false)];

    let error = batch_error(guard.rename_batch(&ops, &options(fail_before(1))));

    assert_eq!(error.rolled_back, 1);
    assert_eq!(support::names_in(&fixture.path("Mods/Foo")), ["foo.pak"]);
}

#[test]
fn a_logic_batch_failing_at_its_last_op_leaves_the_disk_as_it_was() {
    let fixture = pak_mod();
    let before = fixture.paks_snapshot();
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");

    let error = batch_error(guard.rename_batch(&disable_logic(&fixture), &options(fail_before(2))));

    assert_eq!(error.rolled_back, 2);
    assert_eq!(fixture.paks_snapshot(), before);
}

#[test]
fn preflight_writes_nothing() {
    let fixture = pak_mod();
    let before = fixture.paks_snapshot();
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");

    guard.preflight(&disable_cool(&fixture)).expect("preflight");

    assert_eq!(fixture.paks_snapshot(), before);
    assert!(!fixture.journal_path().exists());
}

#[test]
fn forced_fallbacks_refuse_out_of_scope_paths() {
    let fixture = pak_mod();
    support::write_tree(&fixture.paks, &[("base.pak", b"base"), ("Base/", b"")]);
    let scope = fixture.scope();

    let link = link_unlink_noreplace(&scope, &fixture.path("base.pak"), &fixture.path("b.pak"));
    let folder = placeholder_noreplace(&scope, &fixture.path("Base"), &fixture.path("B"));

    assert!(matches!(link, Err(FsError::OutOfScope { .. })));
    assert!(matches!(folder, Err(FsError::OutOfScope { .. })));
    assert!(fixture.path("base.pak").exists());
    assert!(fixture.path("Base").exists());
}
