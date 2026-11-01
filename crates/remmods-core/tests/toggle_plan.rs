//! plan_row and record_matches (spec §5.3, PLAN §7) on scanned TempDir trees.
#![allow(clippy::expect_used)]

mod mods_support;
mod support;

use std::path::Path;

use mods_support::{DATA, fixture, input, paks_tree, row, scan};
use remmods_core::mods::{
    DisableRecord, ModFiles, RecordChange, RowPlan, list_toggle_files, plan_row, record_matches,
};
use tempfile::TempDir;

struct Planned {
    _dir: TempDir,
    ops: Vec<(String, String, bool)>,
    record: RecordChange,
    files: ModFiles,
}

fn plan(
    entries: &[(&str, &[u8])],
    key: &str,
    enable: bool,
    record: Option<&DisableRecord>,
) -> Planned {
    let (dir, paks) = fixture();
    paks_tree(&paks, entries);
    let content = scan(&paks).content;
    let target = row(&content, key);
    let files = list_toggle_files(target, &input(&paks)).expect("list files");
    let RowPlan { ops, record } = plan_row(target, &files, enable, record).expect("plan");
    let ops = ops
        .iter()
        .map(|op| (rel(&paks, &op.from), rel(&paks, &op.to), op.is_dir))
        .collect();
    Planned {
        _dir: dir,
        ops,
        record,
        files,
    }
}

fn rel(paks: &Path, path: &Path) -> String {
    path.strip_prefix(paks)
        .expect("below Paks")
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn file_op(from: &str, to: &str) -> (String, String, bool) {
    (from.to_owned(), to.to_owned(), false)
}

fn dir_op(from: &str, to: &str) -> (String, String, bool) {
    (from.to_owned(), to.to_owned(), true)
}

fn record(files: &[&str]) -> DisableRecord {
    DisableRecord {
        files: files.iter().map(|file| (*file).to_owned()).collect(),
        at: "2026-11-01T10:00:00.000Z".to_owned(),
    }
}

fn recorded_files(change: &RecordChange) -> Vec<String> {
    match change {
        RecordChange::Set(record) => record.files.clone(),
        other => panic!("expected Set, got {other:?}"),
    }
}

#[test]
fn pak_disable_renames_every_depth_and_keeps_text() {
    let planned = plan(
        &[
            ("Mods/Cool/Cool.PAK", DATA),
            ("Mods/Cool/Cool.Sig", DATA),
            ("Mods/Cool/Sub/Deep/Cool_P.ucas", DATA),
            ("Mods/Cool/Sub/Deep/Cool_P.UTOC", DATA),
            ("Mods/Cool/readme.txt", DATA),
            ("Mods/Cool/notes.md", DATA),
        ],
        "Standard:Cool",
        false,
        None,
    );
    let mut ops = planned.ops;
    ops.sort();
    assert_eq!(
        ops,
        [
            file_op("Mods/Cool/Cool.PAK", "Mods/Cool/Cool.PAK-x"),
            file_op("Mods/Cool/Cool.Sig", "Mods/Cool/Cool.Sig-x"),
            file_op(
                "Mods/Cool/Sub/Deep/Cool_P.UTOC",
                "Mods/Cool/Sub/Deep/Cool_P.UTOC-x"
            ),
            file_op(
                "Mods/Cool/Sub/Deep/Cool_P.ucas",
                "Mods/Cool/Sub/Deep/Cool_P.ucas-x"
            ),
        ]
    );
    let mut files = recorded_files(&planned.record);
    files.sort();
    assert_eq!(
        files,
        [
            "Cool.PAK",
            "Cool.Sig",
            "Sub/Deep/Cool_P.UTOC",
            "Sub/Deep/Cool_P.ucas"
        ]
    );
}

#[test]
fn enable_with_matching_record_leaves_a_hand_disabled_variant_off() {
    let entries: &[(&str, &[u8])] = &[
        ("Mods/Cool/Cool.pak-x", DATA),
        ("Mods/Cool/Cool_alt.pak-x", DATA),
    ];
    let saved = record(&["Cool.pak"]);
    let planned = plan(entries, "Standard:Cool", true, Some(&saved));
    assert!(record_matches(&saved, &planned.files));
    assert_eq!(
        planned.ops,
        [file_op("Mods/Cool/Cool.pak-x", "Mods/Cool/Cool.pak")]
    );
    assert_eq!(planned.record, RecordChange::Clear);
}

#[test]
fn enable_with_a_stale_record_strips_every_suffix() {
    let entries: &[(&str, &[u8])] = &[
        ("Mods/Cool/Cool.pak-x", DATA),
        ("Mods/Cool/Cool.UCAS-X", DATA),
        ("Mods/Cool/readme.txt-x", DATA),
    ];
    let saved = record(&["Gone.pak"]);
    let planned = plan(entries, "Standard:Cool", true, Some(&saved));
    assert!(!record_matches(&saved, &planned.files));
    let mut ops = planned.ops;
    ops.sort();
    assert_eq!(
        ops,
        [
            file_op("Mods/Cool/Cool.UCAS-X", "Mods/Cool/Cool.UCAS"),
            file_op("Mods/Cool/Cool.pak-x", "Mods/Cool/Cool.pak"),
            file_op("Mods/Cool/readme.txt-x", "Mods/Cool/readme.txt"),
        ]
    );
}

#[test]
fn partial_disable_records_only_live_files() {
    let planned = plan(
        &[("Mods/P/a.pak", DATA), ("Mods/P/b.pak-x", DATA)],
        "Standard:P",
        false,
        None,
    );
    assert_eq!(planned.ops, [file_op("Mods/P/a.pak", "Mods/P/a.pak-x")]);
    assert_eq!(recorded_files(&planned.record), ["a.pak"]);
}

#[test]
fn logic_disable_renames_the_folder_first() {
    let planned = plan(
        &[
            ("LogicMods/X/X.pak", DATA),
            ("LogicMods/X/Sub/X.utoc", DATA),
        ],
        "Logic:X",
        false,
        None,
    );
    assert_eq!(
        planned.ops,
        [
            dir_op("LogicMods/X", "LogicMods/.disabled_X"),
            file_op(
                "LogicMods/.disabled_X/Sub/X.utoc",
                "LogicMods/.disabled_X/Sub/X.utoc-x"
            ),
            file_op(
                "LogicMods/.disabled_X/X.pak",
                "LogicMods/.disabled_X/X.pak-x"
            ),
        ]
    );
}

#[test]
fn logic_enable_strips_the_prefix_first() {
    let planned = plan(
        &[("LogicMods/.disabled_X/X.pak-x", DATA)],
        "Logic:X",
        true,
        None,
    );
    assert_eq!(
        planned.ops,
        [
            dir_op("LogicMods/.disabled_X", "LogicMods/X"),
            file_op("LogicMods/X/X.pak-x", "LogicMods/X/X.pak"),
        ]
    );
}

#[test]
fn logic_disabled_but_loaded_fix_only_appends_the_suffix() {
    let planned = plan(
        &[("LogicMods/.disabled_X/X.pak", DATA)],
        "Logic:X",
        false,
        None,
    );
    assert_eq!(
        planned.ops,
        [file_op(
            "LogicMods/.disabled_X/X.pak",
            "LogicMods/.disabled_X/X.pak-x"
        )]
    );
}

#[test]
fn logic_enabled_not_loaded_repair_only_strips() {
    let planned = plan(&[("LogicMods/X/X.pak-x", DATA)], "Logic:X", true, None);
    assert_eq!(
        planned.ops,
        [file_op("LogicMods/X/X.pak-x", "LogicMods/X/X.pak")]
    );
}

#[test]
fn loose_group_toggles_only_its_files() {
    let planned = plan(
        &[
            ("Mods/Loose.pak", DATA),
            ("Mods/Loose.ucas", DATA),
            ("Mods/loose.utoc", DATA),
            ("Mods/Other.pak", DATA),
            ("Mods/Loose.txt", DATA),
        ],
        "Loose:Mods/Loose",
        false,
        None,
    );
    let mut ops = planned.ops;
    ops.sort();
    assert_eq!(
        ops,
        [
            file_op("Mods/Loose.pak", "Mods/Loose.pak-x"),
            file_op("Mods/Loose.ucas", "Mods/Loose.ucas-x"),
            file_op("Mods/loose.utoc", "Mods/loose.utoc-x"),
        ]
    );
}

#[test]
fn rows_already_in_the_requested_state_plan_nothing() {
    let on = plan(&[("Mods/On/a.pak", DATA)], "Standard:On", true, None);
    assert!(on.ops.is_empty());
    assert_eq!(on.record, RecordChange::Clear);
    let off = plan(&[("Mods/Off/a.pak-x", DATA)], "Standard:Off", false, None);
    assert!(off.ops.is_empty());
    assert_eq!(off.record, RecordChange::Keep);
    let logic_off = plan(
        &[("LogicMods/.disabled_L/a.pak-x", DATA)],
        "Logic:L",
        false,
        None,
    );
    assert!(logic_off.ops.is_empty());
    let logic_on = plan(&[("LogicMods/L/a.pak", DATA)], "Logic:L", true, None);
    assert!(logic_on.ops.is_empty());
}

#[test]
fn record_matches_needs_exact_off_names_and_no_live_twin() {
    let planned = plan(
        &[
            ("Mods/R/a.pak-x", DATA),
            ("Mods/R/b.pak-X", DATA),
            ("Mods/R/c.pak", DATA),
            ("Mods/R/c.pak-x", DATA),
        ],
        "Standard:R",
        true,
        None,
    );
    let files = &planned.files;
    assert!(record_matches(&record(&["a.pak"]), files));
    assert!(!record_matches(&record(&["b.pak"]), files), "-X is not -x");
    assert!(!record_matches(&record(&["c.pak"]), files), "c.pak is live");
    assert!(!record_matches(&record(&["a.pak", "gone.pak"]), files));
    assert!(!record_matches(&record(&[]), files));
}
