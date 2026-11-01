//! `Core::mods_disable_all` (spec §5.5 step 4, A10).

mod mods_support;
mod support;

use mods_support::DATA;
use mods_support::toggle::ToggleFixture;
use remmods_core::mods::{ModIssue, ModState, ToggleBlock};

#[test]
fn enabled_partial_and_loaded_logic_rows_go_off_in_one_batch() {
    let fixture = ToggleFixture::new(&[
        ("Mods/On/on.pak", DATA),
        ("Mods/Half/a.pak", DATA),
        ("Mods/Half/b.pak-x", DATA),
        ("Mods/Off/off.pak-x", DATA),
        ("LogicMods/Tool/Tool.pak", DATA),
        ("LogicMods/.disabled_Loaded/Loaded.pak", DATA),
        ("Mods/Loose.pak", DATA),
    ]);
    let loaded = fixture.row("Logic:Loaded");
    assert!(loaded.issues.contains(&ModIssue::LogicDisabledButLoaded));
    let outcome = fixture.core.mods_disable_all(false).expect("disable all");
    assert!(outcome.skipped.is_empty());
    assert_eq!(outcome.unmanaged, 0);
    for row in &outcome.refreshed.snapshot.content.rows {
        assert_eq!(row.state, ModState::Disabled, "{}", row.key);
        assert_eq!(row.payload.live_pak, 0, "{}", row.key);
    }
    assert!(fixture.exists("LogicMods/.disabled_Tool/Tool.pak-x"));
    assert!(fixture.exists("LogicMods/.disabled_Loaded/Loaded.pak-x"));
    assert!(fixture.exists("Mods/Off/off.pak-x"));
}

#[test]
fn blocked_rows_with_live_paks_are_skipped() {
    let fixture = ToggleFixture::new(&[
        ("LogicMods/X/a.pak", DATA),
        ("LogicMods/.disabled_X/a.pak-x", DATA),
        ("Mods/Empty/readme.txt", DATA),
        ("Mods/On/on.pak", DATA),
    ]);
    let outcome = fixture.core.mods_disable_all(false).expect("disable all");
    let skipped: Vec<(&str, ToggleBlock)> = outcome
        .skipped
        .iter()
        .map(|row| (row.key.as_str(), row.reason))
        .collect();
    assert_eq!(skipped, [("Logic:X", ToggleBlock::Ambiguous)]);
    assert!(fixture.exists("LogicMods/X/a.pak"));
    assert!(fixture.exists("Mods/On/on.pak-x"));
}

#[test]
fn unmanaged_entries_are_counted_and_untouched() {
    let fixture = ToggleFixture::new(&[
        ("Stray.pak", DATA),
        ("Other/x.pak", DATA),
        ("Other/y.pak-x", DATA),
        (
            "Mods/_Half_tmp_0123456789abcdef0123456789abcdef/Half.pak",
            DATA,
        ),
        ("Mods/On/on.pak", DATA),
    ]);
    let before: Vec<_> = fixture
        .names()
        .into_iter()
        .filter(|(rel, _, _)| !rel.starts_with("Mods/On"))
        .collect();
    let outcome = fixture.core.mods_disable_all(false).expect("disable all");
    assert_eq!(outcome.unmanaged, 3);
    let after: Vec<_> = fixture
        .names()
        .into_iter()
        .filter(|(rel, _, _)| !rel.starts_with("Mods/On"))
        .collect();
    assert_eq!(after, before);
    assert!(fixture.exists("Mods/On/on.pak-x"));
}
