//! Ports of ORIG `ModServiceTests` (toggle subset, spec §8.2). Delete and rename tests wait for
//! P3. ORIG's `ActivateMod`/`DeactivateMod` take a folder; here a toggle takes the row's ref and
//! key, and the "returned path" is the row in the published snapshot.

mod mods_support;
mod support;

use mods_support::DATA;
use mods_support::toggle::ToggleFixture;
use remmods_core::mods::{ModState, ToggleRequest, ToggleTarget};

fn pak_mod(name: &str, enabled: bool) -> ToggleFixture {
    let file = format!(
        "Mods/{name}/test{}",
        if enabled { ".pak" } else { ".pak-x" }
    );
    ToggleFixture::new(&[(file.as_str(), DATA)])
}

fn nested_mod(name: &str, enabled: bool) -> ToggleFixture {
    let ext = if enabled { ".pak" } else { ".pak-x" };
    let file = format!("Mods/{name}/Content/Paks/test{ext}");
    ToggleFixture::new(&[(file.as_str(), DATA)])
}

/// ORIG's Logic fixture holds only `main.lua`; RemMods needs a pak to toggle (NoPakFiles is
/// blocked), so one is added.
fn logic_mod(enabled: bool) -> ToggleFixture {
    let folder = if enabled {
        "LogicMods/logic_mod"
    } else {
        "LogicMods/.disabled_logic_mod"
    };
    let pak = format!(
        "{folder}/logic_mod{}",
        if enabled { ".pak" } else { ".pak-x" }
    );
    let lua = format!("{folder}/main.lua");
    ToggleFixture::new(&[(lua.as_str(), b"print('x')"), (pak.as_str(), DATA)])
}

fn is_enabled(fixture: &ToggleFixture, key: &str) -> bool {
    fixture.row(key).state == ModState::Enabled
}

fn files_ending(fixture: &ToggleFixture, suffix: &str) -> usize {
    fixture
        .names()
        .iter()
        .filter(|(rel, kind, _)| *kind == 'f' && rel.ends_with(suffix))
        .count()
}

#[test]
fn is_mod_enabled_returns_false_when_only_disabled_files_present() {
    let fixture = pak_mod("mod", false);
    assert!(!is_enabled(&fixture, "Standard:mod"));
}

#[test]
fn activate_mod_renames_disabled_files_to_enabled() {
    let fixture = pak_mod("mod", false);
    fixture.toggle(&["Standard:mod"], true).expect("enable");
    assert!(is_enabled(&fixture, "Standard:mod"));
    assert_eq!(files_ending(&fixture, ".pak-x"), 0);
}

#[test]
fn deactivate_mod_renames_enabled_files_to_disabled() {
    let fixture = pak_mod("mod", true);
    fixture.toggle(&["Standard:mod"], false).expect("disable");
    assert!(!is_enabled(&fixture, "Standard:mod"));
    assert_eq!(files_ending(&fixture, ".pak-x"), 1);
}

#[test]
fn activate_mod_then_deactivate_mod_toggles_both_ways() {
    let fixture = pak_mod("mod", false);
    fixture.toggle(&["Standard:mod"], true).expect("enable");
    assert!(is_enabled(&fixture, "Standard:mod"));
    fixture.toggle(&["Standard:mod"], false).expect("disable");
    assert!(!is_enabled(&fixture, "Standard:mod"));
}

#[test]
fn activate_mod_handles_ucas_and_utoc() {
    let fixture = ToggleFixture::new(&[
        ("Mods/multi_ext/test.pak-x", DATA),
        ("Mods/multi_ext/test.ucas-x", DATA),
        ("Mods/multi_ext/test.utoc-x", DATA),
    ]);
    fixture
        .toggle(&["Standard:multi_ext"], true)
        .expect("enable");
    for ext in [".pak", ".ucas", ".utoc"] {
        assert_eq!(files_ending(&fixture, ext), 1, "{ext}");
        assert_eq!(files_ending(&fixture, &format!("{ext}-x")), 0, "{ext}-x");
    }
}

#[test]
fn is_mod_enabled_returns_true_for_enabled_file_in_nested_subfolder() {
    let fixture = nested_mod("nested_enabled", true);
    assert!(is_enabled(&fixture, "Standard:nested_enabled"));
}

#[test]
fn is_mod_enabled_returns_false_for_disabled_file_in_nested_subfolder() {
    let fixture = nested_mod("nested_disabled", false);
    assert!(!is_enabled(&fixture, "Standard:nested_disabled"));
}

#[test]
fn activate_mod_renames_files_in_nested_subfolder() {
    let fixture = nested_mod("nested_mod", false);
    fixture
        .toggle(&["Standard:nested_mod"], true)
        .expect("enable");
    assert!(is_enabled(&fixture, "Standard:nested_mod"));
    assert_eq!(files_ending(&fixture, ".pak-x"), 0);
}

#[test]
fn deactivate_mod_renames_files_in_nested_subfolder() {
    let fixture = nested_mod("nested_mod", true);
    fixture
        .toggle(&["Standard:nested_mod"], false)
        .expect("disable");
    assert!(!is_enabled(&fixture, "Standard:nested_mod"));
    assert!(fixture.exists("Mods/nested_mod/Content/Paks/test.pak-x"));
}

#[test]
fn activate_then_deactivate_toggles_nested_mod_both_ways() {
    let fixture = nested_mod("nested_toggle", false);
    fixture
        .toggle(&["Standard:nested_toggle"], true)
        .expect("enable");
    assert!(is_enabled(&fixture, "Standard:nested_toggle"));
    fixture
        .toggle(&["Standard:nested_toggle"], false)
        .expect("disable");
    assert!(!is_enabled(&fixture, "Standard:nested_toggle"));
}

#[test]
fn deactivate_mod_renames_logic_folder_with_disabled_prefix() {
    let fixture = logic_mod(true);
    fixture
        .toggle(&["Logic:logic_mod"], false)
        .expect("disable");
    assert!(!fixture.exists("LogicMods/logic_mod"));
    assert!(fixture.exists("LogicMods/.disabled_logic_mod"));
    assert!(!is_enabled(&fixture, "Logic:logic_mod"));
}

#[test]
fn activate_mod_strips_disabled_prefix_from_logic_folder() {
    let fixture = logic_mod(false);
    fixture.toggle(&["Logic:logic_mod"], true).expect("enable");
    assert!(fixture.exists("LogicMods/logic_mod"));
    assert!(is_enabled(&fixture, "Logic:logic_mod"));
}

/// Inverted: ORIG only renames the Logic folder; RemMods also adds `-x` to its paks (PLAN §7), so
/// a game that ignores the prefix does not load them. Other files stay as they are.
#[test]
fn logic_mod_also_uses_pak_rename() {
    let fixture = logic_mod(true);
    fixture
        .toggle(&["Logic:logic_mod"], false)
        .expect("disable");
    assert!(fixture.exists("LogicMods/.disabled_logic_mod/main.lua"));
    assert!(fixture.exists("LogicMods/.disabled_logic_mod/logic_mod.pak-x"));
}

fn payload_mod() -> ToggleFixture {
    ToggleFixture::new(&[
        ("Mods/companions/Mod.pak", b"pak"),
        ("Mods/companions/Mod.ucas", b"ucas"),
        ("Mods/companions/Mod.utoc", b"utoc"),
        ("Mods/companions/Mod.sig", b"sig"),
        ("Mods/companions/Mod.txt", b"readme"),
    ])
}

/// ORIG also suffixes `Mod.txt`; RemMods leaves `.txt` alone on disable (PLAN §7) and only
/// strips a `.txt-x` it finds on enable.
#[test]
fn deactivate_mod_suffixes_all_companions_as_atomic_block() {
    let fixture = payload_mod();
    fixture
        .toggle(&["Standard:companions"], false)
        .expect("disable");
    for name in [
        "Mod.pak-x",
        "Mod.ucas-x",
        "Mod.utoc-x",
        "Mod.sig-x",
        "Mod.txt",
    ] {
        assert!(fixture.exists(&format!("Mods/companions/{name}")), "{name}");
    }
    for name in ["Mod.pak", "Mod.ucas", "Mod.utoc", "Mod.sig"] {
        assert!(
            !fixture.exists(&format!("Mods/companions/{name}")),
            "{name}"
        );
    }
}

#[test]
fn activate_mod_restores_all_companions_from_disabled_block() {
    let fixture = payload_mod();
    fixture
        .toggle(&["Standard:companions"], false)
        .expect("disable");
    fixture
        .toggle(&["Standard:companions"], true)
        .expect("enable");
    for name in ["Mod.pak", "Mod.ucas", "Mod.utoc", "Mod.sig", "Mod.txt"] {
        assert!(fixture.exists(&format!("Mods/companions/{name}")), "{name}");
    }
    assert_eq!(files_ending(&fixture, "-x"), 0);
}

#[test]
fn pak_mod_activate_deactivate_returns_same_folder_path() {
    let fixture = pak_mod("pakmod", true);
    let before = fixture.row("Standard:pakmod");
    let off = fixture
        .toggle(&["Standard:pakmod"], false)
        .expect("disable");
    let row = off
        .refreshed
        .snapshot
        .content
        .rows
        .iter()
        .find(|row| row.key == "Standard:pakmod")
        .cloned()
        .expect("row after disable");
    assert_eq!(row.path, before.path);
    assert_eq!(row.mod_ref, before.mod_ref);
    fixture.toggle(&["Standard:pakmod"], true).expect("enable");
    assert_eq!(fixture.row("Standard:pakmod").path, before.path);
}

#[test]
fn toggle_lifecycle_returned_path_remains_usable_for_follow_up_ops() {
    let fixture = logic_mod(true);
    let off = fixture
        .toggle(&["Logic:logic_mod"], false)
        .expect("disable");
    let row = off
        .refreshed
        .snapshot
        .content
        .rows
        .iter()
        .find(|row| row.key == "Logic:logic_mod")
        .cloned()
        .expect("row after disable");
    assert!(row.logic_prefixed);
    let request = ToggleRequest {
        targets: vec![ToggleTarget {
            mod_ref: row.mod_ref,
            key: row.key,
        }],
        enable: true,
        apply_while_running: false,
    };
    fixture
        .core
        .mods_toggle(&request)
        .expect("follow-up toggle");
    assert!(fixture.exists("LogicMods/logic_mod/logic_mod.pak"));
}
