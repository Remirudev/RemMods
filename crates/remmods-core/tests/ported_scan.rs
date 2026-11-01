//! Ported from the original's ModServiceTests, InstallTempFolderTests and ModInfoSizeTests.

mod mods_support;
mod support;

use std::ffi::OsStr;

use mods_support::{DATA, fixture, keys, paks_tree, row, scan};
use remmods_core::mods::{ModKind, ModState};
use remmods_core::safe_fs::names::is_install_temp_name;

const HEX: &str = "0123456789abcdef0123456789abcdef";

#[test]
fn empty_mods_folder_has_no_rows() {
    let (_dir, paks) = fixture();
    paks_tree(&paks, &[("Mods/", b"")]);
    assert!(scan(&paks).content.rows.is_empty());
}

#[test]
fn one_row_per_subdirectory_with_states() {
    let (_dir, paks) = fixture();
    paks_tree(
        &paks,
        &[
            ("Mods/mod_a/test.pak", DATA),
            ("Mods/mod_b/test.pak-x", DATA),
        ],
    );
    let content = scan(&paks).content;
    assert_eq!(content.rows.len(), 2);
    assert_eq!(row(&content, "Standard:mod_a").state, ModState::Enabled);
    assert_eq!(row(&content, "Standard:mod_b").state, ModState::Disabled);
}

#[test]
fn a_plain_file_is_not_a_row() {
    let (_dir, paks) = fixture();
    paks_tree(&paks, &[("Mods/mod", DATA)]);
    assert!(scan(&paks).content.rows.is_empty());
}

#[test]
fn nested_payload_decides_state() {
    let (_dir, paks) = fixture();
    paks_tree(
        &paks,
        &[
            ("Mods/on/Content/Paks/test.pak", DATA),
            ("Mods/off/Content/Paks/test.pak-x", DATA),
        ],
    );
    let content = scan(&paks).content;
    assert_eq!(row(&content, "Standard:on").state, ModState::Enabled);
    assert_eq!(row(&content, "Standard:off").state, ModState::Disabled);
}

#[test]
fn three_roots_aggregate() {
    let (_dir, paks) = fixture();
    paks_tree(
        &paks,
        &[
            ("Mods/std_mod/a.pak", DATA),
            ("~mods/legacy_mod/a.pak", DATA),
            ("LogicMods/logic_mod/a.pak", DATA),
        ],
    );
    let content = scan(&paks).content;
    assert_eq!(
        keys(&content),
        ["Standard:std_mod", "Legacy:legacy_mod", "Logic:logic_mod"]
    );
    assert_eq!(row(&content, "Legacy:legacy_mod").root, "~mods");
    assert_eq!(row(&content, "Legacy:legacy_mod").kind, ModKind::Legacy);
    assert_eq!(row(&content, "Logic:logic_mod").kind, ModKind::Logic);
}

#[test]
fn logic_rows_without_paks_and_prefixed() {
    let (_dir, paks) = fixture();
    paks_tree(
        &paks,
        &[
            ("LogicMods/enabled_logic/main.lua", DATA),
            ("LogicMods/.disabled_disabled_logic/main.lua", DATA),
        ],
    );
    let content = scan(&paks).content;
    // The original shows this one as enabled; RemMods reads the paks (documented divergence).
    assert_eq!(
        row(&content, "Logic:enabled_logic").state,
        ModState::NoPakFiles
    );
    let disabled = row(&content, "Logic:disabled_logic");
    assert_eq!(disabled.state, ModState::Disabled);
    assert!(disabled.logic_prefixed);
}

#[test]
fn no_roots_no_rows() {
    let (_dir, paks) = fixture();
    paks_tree(&paks, &[("Other/", b"")]);
    assert!(scan(&paks).content.rows.is_empty());
}

#[test]
fn install_temp_names() {
    assert!(is_install_temp_name(OsStr::new(&format!(
        "_Cool Skin_tmp_{HEX}"
    ))));
    for name in [
        "Cool Skin".to_owned(),
        "_hidden_mod".to_owned(),
        "my_tmp_mod".to_owned(),
        "_mod_tmp_notahexstring".to_owned(),
        format!("mod_tmp_{HEX}"),
        String::new(),
    ] {
        assert!(!is_install_temp_name(OsStr::new(&name)), "{name:?}");
    }
}

#[test]
fn temp_folders_are_never_rows() {
    let (_dir, paks) = fixture();
    let half = format!("Mods/_Half_tmp_{HEX}/");
    let half_logic = format!("LogicMods/_HalfLogic_tmp_{HEX}/");
    paks_tree(
        &paks,
        &[
            ("Mods/RealMod/a.pak", DATA),
            (half.as_str(), b""),
            (half_logic.as_str(), b""),
        ],
    );
    let content = scan(&paks).content;
    assert_eq!(keys(&content), ["Standard:RealMod"]);
}

#[test]
fn size_sums_every_file() {
    let (_dir, paks) = fixture();
    paks_tree(
        &paks,
        &[
            ("Mods/Two/a.pak", &[0; 100]),
            ("Mods/Two/b.pak", &[0; 50]),
            ("Mods/All/a.pak-x", &[0; 10]),
            ("Mods/All/readme.txt", &[0; 20]),
            ("Mods/All/modinfo.json", b"{}"),
            ("Mods/All/.hidden", &[0; 3]),
        ],
    );
    let content = scan(&paks).content;
    assert_eq!(row(&content, "Standard:Two").size, Some(150));
    assert_eq!(row(&content, "Standard:All").size, Some(35));
}
