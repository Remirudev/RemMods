//! Scan rules: one row per mod folder, roots, LogicMods states, unfinished-install folders and sizes.

mod mods_support;
mod support;

use std::ffi::OsStr;

use mods_support::{DATA, fixture, keys, paks_tree, row, scan};
use remmods_core::mods::{ModKind, ModState, ScanContent};
use remmods_core::safe_fs::names::is_install_temp_name;

const HEX: &str = "5f3ccf7c5cff4a1e9b20d6e8c1a7b394";

fn scanned(entries: &[(&str, &[u8])]) -> ScanContent {
    let (_dir, paks) = fixture();
    paks_tree(&paks, entries);
    scan(&paks).content
}

fn state(content: &ScanContent, key: &str) -> ModState {
    row(content, key).state
}

// Rows and roots.

#[test]
fn empty_mods_folder_lists_nothing() {
    let content = scanned(&[("Mods/", b""), ("~mods/", b""), ("LogicMods/", b"")]);
    assert!(content.rows.is_empty());
}

#[test]
fn each_mod_folder_is_one_row_with_its_state() {
    let content = scanned(&[
        ("Mods/Kazuya/kazuya.pak", DATA),
        ("Mods/Jin/jin.pak-x", DATA),
    ]);
    assert_eq!(content.rows.len(), 2);
    assert_eq!(state(&content, "Standard:Kazuya"), ModState::Enabled);
    assert_eq!(state(&content, "Standard:Jin"), ModState::Disabled);
}

#[test]
fn file_without_extension_in_mods_is_not_a_row() {
    let content = scanned(&[("Mods/readme", DATA)]);
    assert!(content.rows.is_empty());
}

#[test]
fn nested_payload_decides_row_state() {
    let content = scanned(&[
        ("Mods/Arena/Content/Paks/arena.pak", DATA),
        ("Mods/Dojo/Content/Paks/dojo.pak-x", DATA),
    ]);
    assert_eq!(state(&content, "Standard:Arena"), ModState::Enabled);
    assert_eq!(state(&content, "Standard:Dojo"), ModState::Disabled);
}

#[test]
fn rows_from_all_three_roots() {
    let content = scanned(&[
        ("LogicMods/Menu/menu.pak", DATA),
        ("~mods/Jin/jin.pak", DATA),
        ("Mods/Kazuya/kazuya.pak", DATA),
    ]);
    assert_eq!(
        keys(&content),
        ["Standard:Kazuya", "Legacy:Jin", "Logic:Menu"]
    );
    assert_eq!(row(&content, "Standard:Kazuya").kind, ModKind::Standard);
    let legacy = row(&content, "Legacy:Jin");
    assert_eq!(legacy.kind, ModKind::Legacy);
    assert_eq!(legacy.root, "~mods");
    assert_eq!(row(&content, "Logic:Menu").kind, ModKind::Logic);
}

/// RemMods reads a Logic row's state from its paks; the folder prefix alone marks it Disabled.
#[test]
fn logic_rows_without_paks_and_with_prefix() {
    let content = scanned(&[
        ("LogicMods/Menu/init.lua", DATA),
        ("LogicMods/.disabled_Hud/init.lua", DATA),
    ]);
    let menu = row(&content, "Logic:Menu");
    assert_eq!(menu.state, ModState::NoPakFiles);
    assert!(!menu.logic_prefixed);
    let hud = row(&content, "Logic:Hud");
    assert_eq!(hud.state, ModState::Disabled);
    assert!(hud.logic_prefixed);
}

#[test]
fn paks_without_roots_lists_nothing() {
    let content = scanned(&[("Backup/", b""), ("Backup/Kazuya/kazuya.pak", DATA)]);
    assert!(content.rows.is_empty());
}

// Unfinished-install folders.

#[test]
fn unfinished_install_names() {
    for name in [format!("_Stage Pack_tmp_{HEX}"), format!("_tmp_{HEX}")] {
        assert!(is_install_temp_name(OsStr::new(&name)), "{name:?}");
    }
    for name in [
        format!("_Stage Pack_TMP_{HEX}"),
        format!("_Stage Pack_tmp_{}", &HEX[..31]),
        format!("_Stage Pack_tmp_{HEX}0"),
        format!("_Stage Pack-tmp-{HEX}"),
        format!("Stage Pack_tmp_{HEX}"),
    ] {
        assert!(!is_install_temp_name(OsStr::new(&name)), "{name:?}");
    }
}

#[test]
fn unfinished_install_folders_are_not_rows() {
    let in_mods = format!("Mods/_Kazuya_tmp_{HEX}/");
    let in_logic = format!("LogicMods/_Menu_tmp_{HEX}/");
    let content = scanned(&[
        ("Mods/Kazuya/kazuya.pak", DATA),
        (in_mods.as_str(), b""),
        (in_logic.as_str(), b""),
    ]);
    assert_eq!(keys(&content), ["Standard:Kazuya"]);
}

// Sizes.

#[test]
fn row_size_counts_every_file() {
    let content = scanned(&[
        ("Mods/Big/big.pak", &[7; 120]),
        ("Mods/Big/big.utoc", &[7; 80]),
        ("Mods/Mixed/x.pak-x", &[1; 12]),
        ("Mods/Mixed/notes.txt", &[2; 30]),
        ("Mods/Mixed/modinfo.json", b"{}"),
        ("Mods/Mixed/.cache", &[3; 5]),
    ]);
    assert_eq!(row(&content, "Standard:Big").size, Some(200));
    assert_eq!(row(&content, "Standard:Mixed").size, Some(49));
}
