//! Toggling rows: `-x` suffixes, nested payloads, the LogicMods prefix and payload companions (spec §8.2).

mod mods_support;
mod support;

use mods_support::DATA;
use mods_support::toggle::ToggleFixture;
use remmods_core::mods::{ModRow, ModState, ToggleOutcome, ToggleRequest, ToggleTarget};

const KAZUYA: &str = "Standard:Kazuya";
const REINA: &str = "Standard:Reina";
const MENU: &str = "Logic:Menu";
const MENU_SCRIPT: &[u8] = b"-- menu";

fn off_suffix(live: bool) -> &'static str {
    if live { "" } else { "-x" }
}

/// `Mods/Kazuya/kazuya.pak`, live or switched off.
fn kazuya(live: bool) -> ToggleFixture {
    let pak = format!("Mods/Kazuya/kazuya.pak{}", off_suffix(live));
    ToggleFixture::new(&[(pak.as_str(), DATA)])
}

/// A mod that keeps its pak in `Content/Paks`, the layout some mods ship with.
fn stage(folder: &str, live: bool) -> ToggleFixture {
    let pak = format!("Mods/{folder}/Content/Paks/stage.pak{}", off_suffix(live));
    ToggleFixture::new(&[(pak.as_str(), DATA)])
}

/// `LogicMods/Menu` with a pak and a script; switched off it carries the `.disabled_` prefix.
fn menu(live: bool) -> ToggleFixture {
    let folder = if live {
        "LogicMods/Menu"
    } else {
        "LogicMods/.disabled_Menu"
    };
    let pak = format!("{folder}/menu.pak{}", off_suffix(live));
    let script = format!("{folder}/init.lua");
    ToggleFixture::new(&[(pak.as_str(), DATA), (script.as_str(), MENU_SCRIPT)])
}

/// Every payload kind next to a text file.
fn reina() -> ToggleFixture {
    ToggleFixture::new(&[
        ("Mods/Reina/reina.pak", b"pak"),
        ("Mods/Reina/reina.ucas", b"ucas"),
        ("Mods/Reina/reina.utoc", b"utoc"),
        ("Mods/Reina/reina.sig", b"sig"),
        ("Mods/Reina/notes.txt", b"notes"),
    ])
}

fn state(fixture: &ToggleFixture, key: &str) -> ModState {
    fixture.row(key).state
}

fn switch(fixture: &ToggleFixture, key: &str, enable: bool) -> ToggleOutcome {
    fixture
        .toggle(&[key], enable)
        .unwrap_or_else(|error| panic!("toggle {key} to {enable}: {error:?}"))
}

fn published_row(outcome: &ToggleOutcome, key: &str) -> ModRow {
    outcome
        .refreshed
        .snapshot
        .content
        .rows
        .iter()
        .find(|row| row.key == key)
        .cloned()
        .unwrap_or_else(|| panic!("no published row {key}"))
}

fn files_named_with(fixture: &ToggleFixture, ending: &str) -> Vec<String> {
    fixture
        .names()
        .into_iter()
        .filter(|(rel, kind, _)| *kind == 'f' && rel.ends_with(ending))
        .map(|(rel, _, _)| rel)
        .collect()
}

fn file_len(fixture: &ToggleFixture, rel: &str) -> Option<u64> {
    fixture
        .names()
        .into_iter()
        .find(|(path, kind, _)| *kind == 'f' && path == rel)
        .map(|(_, _, len)| len)
}

// Flat pak rows.

#[test]
fn row_with_only_disabled_paks_is_disabled() {
    assert_eq!(state(&kazuya(false), KAZUYA), ModState::Disabled);
    assert_eq!(state(&kazuya(true), KAZUYA), ModState::Enabled);
}

#[test]
fn enabling_removes_every_x_suffix() {
    let fixture = kazuya(false);
    switch(&fixture, KAZUYA, true);
    assert_eq!(state(&fixture, KAZUYA), ModState::Enabled);
    assert!(fixture.exists("Mods/Kazuya/kazuya.pak"));
    assert!(files_named_with(&fixture, "-x").is_empty());
}

#[test]
fn disabling_adds_the_x_suffix() {
    let fixture = kazuya(true);
    switch(&fixture, KAZUYA, false);
    assert_eq!(state(&fixture, KAZUYA), ModState::Disabled);
    assert_eq!(
        files_named_with(&fixture, ".pak-x"),
        ["Mods/Kazuya/kazuya.pak-x"]
    );
    assert!(!fixture.exists("Mods/Kazuya/kazuya.pak"));
}

#[test]
fn pak_row_toggles_on_and_off() {
    let fixture = kazuya(false);
    for enable in [true, false, true] {
        switch(&fixture, KAZUYA, enable);
        let expected = if enable {
            ModState::Enabled
        } else {
            ModState::Disabled
        };
        assert_eq!(state(&fixture, KAZUYA), expected, "after enable={enable}");
    }
}

#[test]
fn enabling_restores_ucas_and_utoc() {
    let fixture = ToggleFixture::new(&[
        ("Mods/Jin/jin.pak-x", DATA),
        ("Mods/Jin/jin.ucas-x", DATA),
        ("Mods/Jin/jin.utoc-x", DATA),
    ]);
    switch(&fixture, "Standard:Jin", true);
    for name in ["jin.pak", "jin.ucas", "jin.utoc"] {
        assert!(fixture.exists(&format!("Mods/Jin/{name}")), "{name}");
        assert!(!fixture.exists(&format!("Mods/Jin/{name}-x")), "{name}-x");
    }
}

// Paks below `Content/Paks` inside the mod folder.

#[test]
fn nested_payload_state_and_toggle_cycle() {
    for (live, expected) in [(true, ModState::Enabled), (false, ModState::Disabled)] {
        let fixture = stage("Arena", live);
        assert_eq!(state(&fixture, "Standard:Arena"), expected, "live={live}");
    }
    let live_pak = "Mods/Arena/Content/Paks/stage.pak";
    let off_pak = "Mods/Arena/Content/Paks/stage.pak-x";
    let fixture = stage("Arena", false);
    for (enable, present, absent, expected) in [
        (true, live_pak, off_pak, ModState::Enabled),
        (false, off_pak, live_pak, ModState::Disabled),
        (true, live_pak, off_pak, ModState::Enabled),
    ] {
        switch(&fixture, "Standard:Arena", enable);
        assert_eq!(
            state(&fixture, "Standard:Arena"),
            expected,
            "enable={enable}"
        );
        assert!(fixture.exists(present), "{present} after enable={enable}");
        assert!(!fixture.exists(absent), "{absent} after enable={enable}");
    }
}

// LogicMods folders.

#[test]
fn disabling_logic_folder_adds_prefix() {
    let fixture = menu(true);
    switch(&fixture, MENU, false);
    assert!(fixture.exists("LogicMods/.disabled_Menu"));
    assert!(!fixture.exists("LogicMods/Menu"));
    let row = fixture.row(MENU);
    assert_eq!(row.state, ModState::Disabled);
    assert!(row.logic_prefixed);
}

#[test]
fn enabling_logic_folder_drops_prefix() {
    let fixture = menu(false);
    switch(&fixture, MENU, true);
    assert!(fixture.exists("LogicMods/Menu"));
    assert!(!fixture.exists("LogicMods/.disabled_Menu"));
    let row = fixture.row(MENU);
    assert_eq!(row.state, ModState::Enabled);
    assert!(!row.logic_prefixed);
}

/// RemMods also adds `-x` to the paks of a prefixed folder, so they stay off even where only
/// the file names count. Other files keep their names and bytes.
#[test]
fn disabled_logic_folder_also_renames_its_paks() {
    let fixture = menu(true);
    switch(&fixture, MENU, false);
    assert!(fixture.exists("LogicMods/.disabled_Menu/menu.pak-x"));
    assert!(!fixture.exists("LogicMods/.disabled_Menu/menu.pak"));
    assert_eq!(
        file_len(&fixture, "LogicMods/.disabled_Menu/init.lua"),
        Some(MENU_SCRIPT.len() as u64)
    );
    assert!(files_named_with(&fixture, ".lua-x").is_empty());
}

// Payload companions.

/// RemMods leaves `.txt` alone on disable and strips a `.txt-x` it finds on enable.
#[test]
fn disabling_suffixes_every_payload_file_but_text() {
    let fixture = reina();
    switch(&fixture, REINA, false);
    for name in ["reina.pak", "reina.ucas", "reina.utoc", "reina.sig"] {
        assert!(fixture.exists(&format!("Mods/Reina/{name}-x")), "{name}-x");
        assert!(!fixture.exists(&format!("Mods/Reina/{name}")), "{name}");
    }
    assert_eq!(
        file_len(&fixture, "Mods/Reina/notes.txt"),
        Some(b"notes".len() as u64)
    );
}

#[test]
fn enabling_restores_every_payload_file() {
    let fixture = reina();
    switch(&fixture, REINA, false);
    switch(&fixture, REINA, true);
    for name in [
        "reina.pak",
        "reina.ucas",
        "reina.utoc",
        "reina.sig",
        "notes.txt",
    ] {
        assert!(fixture.exists(&format!("Mods/Reina/{name}")), "{name}");
    }
    assert!(files_named_with(&fixture, "-x").is_empty());
    assert_eq!(state(&fixture, REINA), ModState::Enabled);
}

// What a toggle hands back.

#[test]
fn toggle_keeps_row_path_and_ref() {
    let fixture = kazuya(true);
    let before = fixture.row(KAZUYA);
    let disabled = published_row(&switch(&fixture, KAZUYA, false), KAZUYA);
    assert_eq!(disabled.state, ModState::Disabled);
    assert_eq!(disabled.path, before.path);
    assert_eq!(disabled.mod_ref, before.mod_ref);
    let enabled = published_row(&switch(&fixture, KAZUYA, true), KAZUYA);
    assert_eq!(enabled.path, before.path);
}

#[test]
fn ref_from_published_snapshot_drives_next_toggle() {
    let fixture = menu(true);
    let disabled = published_row(&switch(&fixture, MENU, false), MENU);
    assert!(disabled.logic_prefixed);
    let request = ToggleRequest {
        targets: vec![ToggleTarget {
            mod_ref: disabled.mod_ref,
            key: disabled.key,
        }],
        enable: true,
        apply_while_running: false,
    };
    fixture
        .core
        .mods_toggle(&request)
        .expect("toggle with the published ref");
    assert!(fixture.exists("LogicMods/Menu/menu.pak"));
    assert!(fixture.exists("LogicMods/Menu/init.lua"));
}
