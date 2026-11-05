//! Pak conflicts between rows and loose copies of mod paks (spec §8.1).

mod mods_support;
mod support;

use mods_support::{DATA, fixture, paks_tree, row, scan};
use remmods_core::mods::{ModIssue, ModKind, ScanContent, UnmanagedKind};

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

fn no_row_is_linked(content: &ScanContent) -> bool {
    content.rows.iter().all(|row| row.conflicts_with.is_empty())
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

// Links between rows.

#[test]
fn empty_paks_folder_has_no_rows() {
    let content = content_of(&[]);
    assert!(content.rows.is_empty());
    assert!(content.unmanaged.is_empty());
}

#[test]
fn separate_paks_leave_rows_unlinked() {
    let content = content_of(&[
        ("Mods/Kazuya/kazuya_red.pak", DATA),
        ("Mods/Jin/jin_blue.pak", DATA),
    ]);
    assert_eq!(content.rows.len(), 2);
    assert!(no_row_is_linked(&content));
}

#[test]
fn same_pak_name_links_both_rows() {
    let content = content_of(&[
        ("Mods/Kazuya/outfit.pak", DATA),
        ("Mods/Jin/outfit.pak", DATA),
    ]);
    assert_eq!(conflict_keys(&content, "Standard:Jin"), ["Standard:Kazuya"]);
    assert_eq!(conflict_keys(&content, "Standard:Kazuya"), ["Standard:Jin"]);
    let link = &row(&content, "Standard:Kazuya").conflicts_with[0];
    assert_eq!(link.name, "Jin");
    assert_eq!(link.paks, ["outfit.pak"]);
}

#[test]
fn disabled_copy_does_not_link() {
    let content = content_of(&[
        ("Mods/Kazuya/outfit.pak", DATA),
        ("Mods/Jin/outfit.pak-x", DATA),
    ]);
    assert!(no_row_is_linked(&content));
}

#[test]
fn only_rows_sharing_a_pak_are_linked() {
    let content = content_of(&[
        ("Mods/Lili/lili_hair.pak", DATA),
        ("Mods/Lili/stage.pak", DATA),
        ("Mods/Reina/reina_coat.pak", DATA),
        ("Mods/Reina/stage.pak", DATA),
        ("Mods/Asuka/asuka_gloves.pak", DATA),
    ]);
    assert!(conflict_keys(&content, "Standard:Asuka").is_empty());
    assert_eq!(conflict_keys(&content, "Standard:Lili"), ["Standard:Reina"]);
    assert_eq!(conflict_keys(&content, "Standard:Reina"), ["Standard:Lili"]);
    let link = &row(&content, "Standard:Reina").conflicts_with[0];
    assert_eq!(link.paks, ["stage.pak"]);
}

#[test]
fn nested_payload_links_with_flat_payload() {
    let content = content_of(&[
        ("Mods/StageFlat/stage.pak", DATA),
        ("Mods/StageNested/Content/Paks/stage.pak", DATA),
    ]);
    assert_eq!(
        conflict_keys(&content, "Standard:StageFlat"),
        ["Standard:StageNested"]
    );
    assert_eq!(
        conflict_keys(&content, "Standard:StageNested"),
        ["Standard:StageFlat"]
    );
}

#[test]
fn two_copies_inside_one_mod_are_not_a_conflict() {
    let content = content_of(&[
        ("Mods/Hud/hud.pak", DATA),
        ("Mods/Hud/variant/hud.pak", DATA),
    ]);
    assert_eq!(content.rows.len(), 1);
    assert!(conflict_keys(&content, "Standard:Hud").is_empty());
}

#[test]
fn names_differing_in_case_still_link() {
    let content = content_of(&[
        ("Mods/Lower/extra/outfit.PAK", DATA),
        ("Mods/Upper/Outfit.pak", DATA),
    ]);
    assert_eq!(
        conflict_keys(&content, "Standard:Lower"),
        ["Standard:Upper"]
    );
    assert_eq!(
        conflict_keys(&content, "Standard:Upper"),
        ["Standard:Lower"]
    );
    // Each row's link lists the shared pak as that row spells it on disk.
    assert_eq!(
        row(&content, "Standard:Upper").conflicts_with[0].paks,
        ["Outfit.pak"]
    );
    assert_eq!(
        row(&content, "Standard:Lower").conflicts_with[0].paks,
        ["outfit.PAK"]
    );
}

#[test]
fn all_three_roots_take_part() {
    let content = content_of(&[
        ("LogicMods/Menu/rival_stage.pak", DATA),
        ("Mods/Kazuya/rival_stage.pak", DATA),
        ("~mods/Jin/rival_stage.pak", DATA),
    ]);
    assert_eq!(
        conflict_keys(&content, "Standard:Kazuya"),
        ["Legacy:Jin", "Logic:Menu"]
    );
    assert_eq!(
        conflict_keys(&content, "Logic:Menu"),
        ["Legacy:Jin", "Standard:Kazuya"]
    );
}

#[test]
fn prefixed_logic_folder_with_live_pak_still_links() {
    let content = content_of(&[
        ("LogicMods/.disabled_Menu/menu.pak", DATA),
        ("Mods/Kazuya/menu.pak", DATA),
    ]);
    assert!(row(&content, "Logic:Menu").logic_prefixed);
    assert_eq!(conflict_keys(&content, "Logic:Menu"), ["Standard:Kazuya"]);
    assert_eq!(conflict_keys(&content, "Standard:Kazuya"), ["Logic:Menu"]);
}

#[test]
fn shared_companion_names_do_not_link() {
    let content = content_of(&[
        ("Mods/Jin/jin.pak", DATA),
        ("Mods/Jin/effects.ucas", DATA),
        ("Mods/Jin/effects.utoc", DATA),
        ("Mods/Kazuya/kazuya.pak", DATA),
        ("Mods/Kazuya/effects.ucas", DATA),
        ("Mods/Kazuya/effects.utoc", DATA),
    ]);
    assert!(no_row_is_linked(&content));
}

#[test]
fn loose_pak_links_with_folder_mod() {
    let content = content_of(&[("Mods/outfit.pak", DATA), ("Mods/Kazuya/outfit.pak", DATA)]);
    assert_eq!(row(&content, "Loose:Mods/outfit").kind, ModKind::Loose);
    assert_eq!(
        conflict_keys(&content, "Loose:Mods/outfit"),
        ["Standard:Kazuya"]
    );
}

// Loose copies of mod files at the Paks top level or in other folders.

#[test]
fn top_level_copy_of_disabled_pak_is_flagged() {
    let content = content_of(&[("reina.pak", DATA), ("Mods/Reina/reina.pak-x", DATA)]);
    assert_eq!(content.unmanaged.len(), 1);
    let copy = &content.unmanaged[0];
    assert_eq!(copy.kind, UnmanagedKind::PaksFile);
    assert_eq!(copy.rel_path, "reina.pak");
    assert_eq!(copy.copy_of, ["Reina"]);
    assert_eq!(
        loose_copy(&content, "Standard:Reina"),
        Some(vec!["reina.pak".to_owned()])
    );
}

#[test]
fn top_level_companion_copies_are_flagged() {
    let content = content_of(&[
        ("Mods/Lili/lili.pak", DATA),
        ("Mods/Lili/lili.ucas", DATA),
        ("Mods/Lili/lili.utoc", DATA),
        ("LILI.UCAS", DATA),
        ("lili.utoc", DATA),
    ]);
    assert_eq!(content.unmanaged.len(), 2);
    for copy in &content.unmanaged {
        assert_eq!(copy.kind, UnmanagedKind::PaksFile, "{}", copy.rel_path);
        assert_eq!(copy.copy_of, ["Lili"], "{}", copy.rel_path);
    }
    assert_eq!(
        loose_copy(&content, "Standard:Lili"),
        Some(vec!["lili.ucas".to_owned(), "lili.utoc".to_owned()])
    );
}

#[test]
fn game_paks_disabled_and_other_files_are_not_copies() {
    let content = content_of(&[
        ("pakchunk3-Windows.pak", DATA),
        ("jin.pak-x", DATA),
        ("jin.txt", DATA),
        ("Mods/Jin/pakchunk3-Windows.pak", DATA),
        ("Mods/Jin/jin.pak", DATA),
        ("Mods/Jin/jin.txt", DATA),
    ]);
    assert!(content.unmanaged.is_empty());
    assert_eq!(loose_copy(&content, "Standard:Jin"), None);
}

#[test]
fn top_level_pak_without_roots_is_not_a_copy() {
    let content = content_of(&[("stray.pak", DATA)]);
    assert!(content.rows.is_empty());
    assert_eq!(content.unmanaged.len(), 1);
    assert_eq!(content.unmanaged[0].rel_path, "stray.pak");
    assert!(content.unmanaged[0].copy_of.is_empty());
}

#[test]
fn copy_in_another_folder_is_flagged() {
    let content = content_of(&[
        ("Old/Asuka/asuka.pak", DATA),
        ("Mods/Asuka/asuka.pak", DATA),
    ]);
    assert_eq!(content.unmanaged.len(), 1);
    let folder = &content.unmanaged[0];
    assert_eq!(folder.kind, UnmanagedKind::Folder);
    assert_eq!(folder.rel_path, "Old");
    assert_eq!(folder.copy_of, ["Asuka"]);
    assert_eq!(
        loose_copy(&content, "Standard:Asuka"),
        Some(vec!["asuka.pak".to_owned()])
    );
}
