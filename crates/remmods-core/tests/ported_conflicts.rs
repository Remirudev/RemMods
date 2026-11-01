//! Ported from the original's ConflictDetectorTests and upstream main's loose-copy tests, plus
//! RemMods' additions and the original's projection (spec §8.1).

mod mods_support;
mod support;

use std::collections::BTreeSet;

use mods_support::{DATA, fixture, paks_tree, row, scan};
use remmods_core::mods::{
    ModIssue, ModKind, ModState, ScanContent, UnmanagedKind, original_conflict_names,
    original_enabled,
};

fn content_of(entries: &[(&str, &[u8])]) -> ScanContent {
    let (_dir, paks) = fixture();
    paks_tree(&paks, entries);
    scan(&paks).content
}

fn conflict_keys(content: &ScanContent, key: &str) -> Vec<String> {
    row(content, key)
        .conflicts_with
        .iter()
        .map(|link| link.key.clone())
        .collect()
}

fn names(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn distinct_names_do_not_conflict() {
    let content = content_of(&[("Mods/mod_a/a.pak", DATA), ("Mods/mod_b/b.pak", DATA)]);
    assert!(content.rows.iter().all(|r| r.conflicts_with.is_empty()));
    assert_eq!(original_conflict_names(&content), names(&[]));
}

#[test]
fn shared_pak_links_both() {
    let content = content_of(&[
        ("Mods/mod_a/shared.pak", DATA),
        ("Mods/mod_b/shared.pak", DATA),
    ]);
    assert_eq!(
        conflict_keys(&content, "Standard:mod_a"),
        ["Standard:mod_b"]
    );
    assert_eq!(
        conflict_keys(&content, "Standard:mod_b"),
        ["Standard:mod_a"]
    );
    let link = &row(&content, "Standard:mod_a").conflicts_with[0];
    assert_eq!(link.paks, ["shared.pak"]);
    assert_eq!(link.name, "mod_b");
    assert_eq!(
        original_conflict_names(&content),
        names(&["mod_a", "mod_b"])
    );
}

#[test]
fn switched_off_pak_does_not_conflict() {
    let content = content_of(&[
        ("Mods/mod_a/shared.pak", DATA),
        ("Mods/mod_b/shared.pak-x", DATA),
    ]);
    assert!(content.rows.iter().all(|r| r.conflicts_with.is_empty()));
    assert_eq!(original_conflict_names(&content), names(&[]));
}

#[test]
fn empty_paks_has_no_conflicts() {
    let content = content_of(&[]);
    assert!(content.rows.is_empty());
    assert_eq!(original_conflict_names(&content), names(&[]));
}

#[test]
fn only_the_sharing_pair_conflicts() {
    let content = content_of(&[
        ("Mods/a/unique_a.pak", DATA),
        ("Mods/a/common.pak", DATA),
        ("Mods/b/unique_b.pak", DATA),
        ("Mods/b/common.pak", DATA),
        ("Mods/c/unique_c.pak", DATA),
    ]);
    assert_eq!(conflict_keys(&content, "Standard:a"), ["Standard:b"]);
    assert_eq!(conflict_keys(&content, "Standard:b"), ["Standard:a"]);
    assert!(conflict_keys(&content, "Standard:c").is_empty());
    assert_eq!(original_conflict_names(&content), names(&["a", "b"]));
}

#[test]
fn nested_and_flat_paks_conflict() {
    let content = content_of(&[
        ("Mods/nested/Content/Paks/common.pak", DATA),
        ("Mods/flat/common.pak", DATA),
    ]);
    assert_eq!(
        conflict_keys(&content, "Standard:nested"),
        ["Standard:flat"]
    );
    assert_eq!(
        original_conflict_names(&content),
        names(&["flat", "nested"])
    );
}

#[test]
fn one_mod_never_conflicts_with_itself() {
    let content = content_of(&[
        ("Mods/solo/common.pak", DATA),
        ("Mods/solo/alt/common.pak", DATA),
    ]);
    assert!(conflict_keys(&content, "Standard:solo").is_empty());
    assert_eq!(original_conflict_names(&content), names(&[]));
}

#[test]
fn case_differences_still_conflict() {
    let content = content_of(&[
        ("Mods/upper/Shared.pak", DATA),
        ("Mods/lower/sub/shared.PAK", DATA),
    ]);
    assert_eq!(
        conflict_keys(&content, "Standard:upper"),
        ["Standard:lower"]
    );
    assert_eq!(
        row(&content, "Standard:lower").conflicts_with[0].paks,
        ["shared.PAK"]
    );
    assert_eq!(
        original_conflict_names(&content),
        names(&["lower", "upper"])
    );
}

#[test]
fn roots_and_logic_take_part() {
    let content = content_of(&[
        ("Mods/X/a.pak", DATA),
        ("~mods/Y/a.pak", DATA),
        ("LogicMods/Z/a.pak", DATA),
    ]);
    assert_eq!(
        conflict_keys(&content, "Standard:X"),
        ["Legacy:Y", "Logic:Z"]
    );
    assert_eq!(original_conflict_names(&content), names(&["x", "y", "z"]));
}

#[test]
fn prefixed_logic_with_live_pak_conflicts_but_not_for_the_original() {
    let content = content_of(&[
        ("Mods/X/a.pak", DATA),
        ("LogicMods/.disabled_P/a.pak", DATA),
    ]);
    assert_eq!(conflict_keys(&content, "Standard:X"), ["Logic:P"]);
    assert_eq!(conflict_keys(&content, "Logic:P"), ["Standard:X"]);
    assert_eq!(original_conflict_names(&content), names(&[]));
}

#[test]
fn companion_files_never_conflict() {
    let content = content_of(&[
        ("Mods/A/main.pak", DATA),
        ("Mods/A/a.ucas", DATA),
        ("Mods/B/other.pak", DATA),
        ("Mods/B/a.ucas", DATA),
    ]);
    assert!(content.rows.iter().all(|r| r.conflicts_with.is_empty()));
}

#[test]
fn loose_rows_are_not_in_the_original_view() {
    let content = content_of(&[("Mods/Folder/a.pak", DATA), ("Mods/a.pak", DATA)]);
    assert_eq!(conflict_keys(&content, "Loose:Mods/a"), ["Standard:Folder"]);
    assert_eq!(original_conflict_names(&content), names(&[]));
}

#[test]
fn original_enabled_per_state() {
    let content = content_of(&[
        ("Mods/On/a.pak", DATA),
        ("Mods/Part/b.pak", DATA),
        ("Mods/Part/c.pak-x", DATA),
        ("Mods/Off/d.pak-x", DATA),
        ("Mods/None/readme.txt", DATA),
        ("LogicMods/L/readme.txt", DATA),
        ("LogicMods/.disabled_P/e.pak", DATA),
        ("Mods/Loose.pak", DATA),
    ]);
    let enabled = |key: &str| original_enabled(row(&content, key));
    assert_eq!(enabled("Standard:On"), Some(true));
    assert_eq!(enabled("Standard:Part"), Some(true));
    assert_eq!(enabled("Standard:Off"), Some(false));
    assert_eq!(enabled("Standard:None"), Some(false));
    assert_eq!(enabled("Logic:L"), Some(true));
    assert_eq!(enabled("Logic:P"), Some(false));
    assert_eq!(enabled("Loose:Mods/Loose"), None);
    let mut unknown = row(&content, "Standard:On").clone();
    unknown.state = ModState::Unknown;
    assert_eq!(original_enabled(&unknown), None);
    assert_eq!(row(&content, "Loose:Mods/Loose").kind, ModKind::Loose);
}

fn loose_copy(content: &ScanContent, key: &str) -> Option<Vec<String>> {
    row(content, key)
        .issues
        .iter()
        .find_map(|issue| match issue {
            ModIssue::LooseCopy { files } => Some(files.clone()),
            _ => None,
        })
}

#[test]
fn disabled_mod_copied_at_paks_top_level() {
    let content = content_of(&[("Mods/mod_a/a.pak-x", DATA), ("a.pak", DATA)]);
    let stray = &content.unmanaged[0];
    assert_eq!(stray.kind, UnmanagedKind::PaksFile);
    assert_eq!(stray.rel_path, "a.pak");
    assert_eq!(stray.copy_of, ["mod_a"]);
    assert_eq!(
        loose_copy(&content, "Standard:mod_a"),
        Some(vec!["a.pak".to_owned()])
    );
}

#[test]
fn companion_copies_count() {
    let content = content_of(&[
        ("Mods/mod_a/a.pak", DATA),
        ("Mods/mod_a/a.ucas", DATA),
        ("Mods/mod_a/a.utoc", DATA),
        ("A.UCAS", DATA),
        ("a.utoc", DATA),
    ]);
    assert_eq!(content.unmanaged.len(), 2);
    assert!(content.unmanaged.iter().all(|e| e.copy_of == ["mod_a"]));
    assert_eq!(
        loose_copy(&content, "Standard:mod_a"),
        Some(vec!["a.ucas".to_owned(), "a.utoc".to_owned()])
    );
}

#[test]
fn game_files_and_switched_off_files_are_no_copies() {
    let content = content_of(&[
        ("Mods/mod_a/pakchunk0-WindowsNoEditor.pak", DATA),
        ("Mods/mod_a/a.pak", DATA),
        ("pakchunk0-WindowsNoEditor.pak", DATA),
        ("a.pak-x", DATA),
        ("a.txt", DATA),
    ]);
    assert!(content.unmanaged.is_empty());
    assert_eq!(loose_copy(&content, "Standard:mod_a"), None);
}

#[test]
fn no_roots_no_copies() {
    let content = content_of(&[("a.pak", DATA)]);
    assert_eq!(content.unmanaged.len(), 1);
    assert!(content.unmanaged[0].copy_of.is_empty());
}

#[test]
fn copies_in_non_root_folders() {
    let content = content_of(&[("Mods/Cool/cool.pak", DATA), ("Backup/Cool/cool.pak", DATA)]);
    let backup = &content.unmanaged[0];
    assert_eq!(backup.kind, UnmanagedKind::Folder);
    assert_eq!(backup.rel_path, "Backup");
    assert_eq!(backup.copy_of, ["Cool"]);
    assert_eq!(
        loose_copy(&content, "Standard:Cool"),
        Some(vec!["cool.pak".to_owned()])
    );
}
