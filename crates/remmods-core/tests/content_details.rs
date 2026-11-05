//! `Core::mods_contents`: asset lists, adds vs replaces and shared assets (SPEC-P2B §5.3 rules 4–5).
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod mods_support;
mod support;

use std::fs;

use mods_support::toggle::ToggleFixture;
use mods_support::{DATA, MOUNT, listing, toc};
use remmods_core::CoreError;
use remmods_core::content::{
    AssetEntry, AssetOrigin, BaseIndexState, CONTENTS_MAX_ENTRIES, ModContents,
};
use remmods_core::mods::ModsError;
use remmods_pak::fixture::TocBuilder;

const BASE: &str = "pakchunk0-Windows.utoc";

fn base_toc(files: &[&str]) -> Vec<u8> {
    toc("../../../", files)
}

/// Builds the base index first, as the app's background check does after a scan.
fn contents(fixture: &ToggleFixture, key: &str) -> ModContents {
    fixture.core.mods_base_refresh().expect("mods_base_refresh");
    let row = fixture.row(key);
    fixture
        .core
        .mods_contents(row.mod_ref.as_str())
        .expect("mods_contents")
}

fn entry<'a>(contents: &'a ModContents, path: &str) -> &'a AssetEntry {
    contents
        .entries
        .iter()
        .find(|e| e.path == path)
        .unwrap_or_else(|| panic!("no entry {path}"))
}

fn origins(contents: &ModContents) -> Vec<(&str, AssetOrigin)> {
    contents
        .entries
        .iter()
        .map(|e| (e.path.as_str(), e.origin))
        .collect()
}

#[test]
fn replaces_when_in_base_index() {
    let base = base_toc(&["Polaris/Content/Smoke/Base.uasset"]);
    let mod_toc = toc(MOUNT, &["Smoke/Base.uasset", "Smoke/New.uasset"]);
    let fixture = ToggleFixture::new(&[
        (BASE, &base),
        ("Mods/A/A.pak", DATA),
        ("Mods/A/A.utoc", &mod_toc),
    ]);
    let found = contents(&fixture, "Standard:A");
    assert_eq!(found.base, BaseIndexState::Complete);
    assert_eq!(
        origins(&found),
        [
            ("Polaris/Content/Smoke/Base.uasset", AssetOrigin::Replaces),
            ("Polaris/Content/Smoke/New.uasset", AssetOrigin::Adds),
        ]
    );
    assert_eq!((found.total, found.truncated), (2, false));
}

#[test]
fn adds_when_absent_and_base_complete() {
    let base = base_toc(&["Polaris/Content/Other.uasset"]);
    let mod_toc = toc(
        MOUNT,
        &["Character/Item/Base_Item/ker/bdf/BI_ker_bdf_x.uasset"],
    );
    let fixture = ToggleFixture::new(&[
        (BASE, &base),
        ("Mods/A/A.pak", DATA),
        ("Mods/A/A.utoc", &mod_toc),
    ]);
    let found = contents(&fixture, "Standard:A");
    let only = &found.entries[0];
    assert_eq!(only.origin, AssetOrigin::Adds);
    assert_eq!(only.container, "A.utoc");
    assert!(only.live);
    assert_eq!(only.character.as_deref(), Some("ker"));
    assert_eq!(only.slot.as_deref(), Some("bdf"));
}

#[test]
fn unknown_when_base_partial() {
    let base = base_toc(&["Polaris/Content/Smoke/Base.uasset"]);
    let mod_toc = toc(MOUNT, &["Smoke/Base.uasset", "Smoke/New.uasset"]);
    let fixture = ToggleFixture::new(&[
        (BASE, &base),
        ("pakchunk1-Windows.utoc", DATA),
        ("Mods/A/A.pak", DATA),
        ("Mods/A/A.utoc", &mod_toc),
    ]);
    let found = contents(&fixture, "Standard:A");
    assert_eq!(found.base, BaseIndexState::Partial);
    assert_eq!(
        origins(&found),
        [
            ("Polaris/Content/Smoke/Base.uasset", AssetOrigin::Replaces),
            ("Polaris/Content/Smoke/New.uasset", AssetOrigin::Unknown),
        ]
    );
}

#[test]
fn unavailable_without_base_containers() {
    let mod_toc = toc(MOUNT, &["Smoke/New.uasset"]);
    let fixture = ToggleFixture::new(&[("Mods/A/A.pak", DATA), ("Mods/A/A.utoc", &mod_toc)]);
    let found = contents(&fixture, "Standard:A");
    assert_eq!(found.base, BaseIndexState::Unavailable);
    assert_eq!(found.entries[0].origin, AssetOrigin::Unknown);
    let garbage = ToggleFixture::new(&[
        (BASE, DATA),
        ("global.utoc", b""),
        ("Mods/A/A.pak", DATA),
        ("Mods/A/A.utoc", &mod_toc),
    ]);
    assert_eq!(
        contents(&garbage, "Standard:A").base,
        BaseIndexState::Unavailable
    );
}

#[test]
fn global_without_index_counts_as_read() {
    let base = base_toc(&["Polaris/Content/Smoke/Base.uasset"]);
    let global = TocBuilder::new("../../../").flags(0x00).build();
    let mod_toc = toc(MOUNT, &["Smoke/New.uasset"]);
    let fixture = ToggleFixture::new(&[
        (BASE, &base),
        ("global.utoc", &global),
        ("Mods/A/A.pak", DATA),
        ("Mods/A/A.utoc", &mod_toc),
    ]);
    let found = contents(&fixture, "Standard:A");
    assert_eq!(found.base, BaseIndexState::Complete);
    assert_eq!(found.entries[0].origin, AssetOrigin::Adds);
}

#[test]
fn base_index_is_case_insensitive() {
    let base = base_toc(&["Polaris/Content/Smoke/BASE.uasset"]);
    let mod_toc = toc(MOUNT, &["smoke/base.uasset"]);
    let fixture = ToggleFixture::new(&[
        (BASE, &base),
        ("Mods/A/A.pak", DATA),
        ("Mods/A/A.utoc", &mod_toc),
    ]);
    let found = contents(&fixture, "Standard:A");
    assert_eq!(found.entries[0].origin, AssetOrigin::Replaces);
}

#[test]
fn base_index_rebuilt_when_base_changes() {
    let base = base_toc(&["Polaris/Content/Smoke/Other.uasset"]);
    let mod_toc = toc(MOUNT, &["Smoke/New.uasset"]);
    let fixture = ToggleFixture::new(&[
        (BASE, &base),
        ("Mods/A/A.pak", DATA),
        ("Mods/A/A.utoc", &mod_toc),
    ]);
    assert_eq!(
        contents(&fixture, "Standard:A").entries[0].origin,
        AssetOrigin::Adds
    );
    let grown = base_toc(&[
        "Polaris/Content/Smoke/Other.uasset",
        "Polaris/Content/Smoke/New.uasset",
    ]);
    fs::write(fixture.paks.join(BASE), grown).expect("rewrite base");
    assert_eq!(
        contents(&fixture, "Standard:A").entries[0].origin,
        AssetOrigin::Replaces
    );
}

#[test]
fn shared_with_lists_enabled_others() {
    let a = toc(
        MOUNT,
        &[
            "Smoke/Shared.uasset",
            "Smoke/Shared.ubulk",
            "Smoke/A.uasset",
        ],
    );
    let b = toc(MOUNT, &["Smoke/Shared.uasset"]);
    let base = base_toc(&["Polaris/Content/Smoke/Shared.uasset"]);
    let fixture = ToggleFixture::new(&[
        (BASE, &base),
        ("Mods/A/A.pak", DATA),
        ("Mods/A/A.utoc", &a),
        ("Mods/B/B.pak", DATA),
        ("Mods/B/B.utoc", &b),
        ("Mods/C/C.pak-x", DATA),
        ("Mods/C/C.utoc-x", &b),
    ]);
    let b_ref = fixture.row("Standard:B").mod_ref;
    let found = contents(&fixture, "Standard:A");
    assert_eq!(
        entry(&found, "Polaris/Content/Smoke/Shared.uasset").shared_with,
        std::slice::from_ref(&b_ref)
    );
    assert_eq!(
        entry(&found, "Polaris/Content/Smoke/Shared.ubulk").shared_with,
        [b_ref]
    );
    assert!(
        entry(&found, "Polaris/Content/Smoke/A.uasset")
            .shared_with
            .is_empty()
    );
}

#[test]
fn shared_shoes_stay_listed_but_are_no_outfit() {
    let shoes = "Character/Item/Base_Item/ker/sho/BI_ker_sho_x.uasset";
    let game_shoes = format!("Polaris/Content/{shoes}");
    let a = toc(MOUNT, &[shoes, "Smoke/A.uasset"]);
    let b = toc(MOUNT, &[shoes]);
    let base = base_toc(&[game_shoes.as_str()]);
    let fixture = ToggleFixture::new(&[
        (BASE, &base),
        ("Mods/A/A.pak", DATA),
        ("Mods/A/A.utoc", &a),
        ("Mods/B/B.pak", DATA),
        ("Mods/B/B.utoc", &b),
    ]);
    let found = contents(&fixture, "Standard:A");
    let b_ref = fixture.row("Standard:B").mod_ref;
    let links = fixture.row("Standard:A").content_conflicts;
    assert_eq!(links.len(), 1);
    assert_eq!((links[0].shared, links[0].outfit), (1, 0));
    assert_eq!(entry(&found, &game_shoes).shared_with, [b_ref]);
}

#[test]
fn off_entries_have_no_shared_with() {
    let shared = toc(MOUNT, &["Smoke/Shared.uasset"]);
    let base = base_toc(&["Polaris/Content/Smoke/Shared.uasset"]);
    let fixture = ToggleFixture::new(&[
        (BASE, &base),
        ("Mods/A/A.pak", DATA),
        ("Mods/A/A.utoc", &shared),
        ("Mods/A/Off/A_Off.utoc-x", &shared),
        ("Mods/B/B.pak", DATA),
        ("Mods/B/B.utoc", &shared),
    ]);
    let found = contents(&fixture, "Standard:A");
    let pairs: Vec<(&str, bool, usize)> = found
        .entries
        .iter()
        .map(|e| (e.container.as_str(), e.live, e.shared_with.len()))
        .collect();
    assert_eq!(pairs, [("A.utoc", true, 1), ("Off/A_Off.utoc-x", false, 0)]);
}

#[test]
fn truncated_at_max_entries() {
    let names: Vec<String> = (0..CONTENTS_MAX_ENTRIES + 3)
        .map(|i| format!("Many/F{i:05}.uasset"))
        .collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let big = toc(MOUNT, &refs);
    let fixture = ToggleFixture::new(&[("Mods/A/A.pak", DATA), ("Mods/A/A.utoc", &big)]);
    let found = contents(&fixture, "Standard:A");
    assert_eq!(found.total, 5003);
    assert!(found.truncated);
    assert_eq!(found.entries.len(), CONTENTS_MAX_ENTRIES);
    assert_eq!(found.entries[0].path, "Polaris/Content/Many/F00000.uasset");
}

#[test]
fn unknown_ref_is_stale() {
    let fixture = ToggleFixture::new(&[("Mods/A/A.pak", DATA)]);
    fixture.snapshot();
    assert!(matches!(
        fixture.core.mods_contents("v1.nope"),
        Err(CoreError::Mods(ModsError::Stale))
    ));
}

#[test]
fn contents_never_write() {
    let base = base_toc(&["Polaris/Content/Smoke/Base.uasset"]);
    let shared = toc(MOUNT, &["Smoke/Base.uasset", "Smoke/Shared.uasset"]);
    let fixture = ToggleFixture::new(&[
        (BASE, &base),
        ("global.utoc", DATA),
        ("Mods/A/A.pak", DATA),
        ("Mods/A/A.utoc", &shared),
        ("Mods/B/B.pak-x", DATA),
        ("Mods/B/B.utoc-x", &shared),
        ("Mods/C/C.pak", DATA),
        ("Mods/C/C.utoc", DATA),
        ("Mods/Loose.pak", DATA),
        ("Mods/Loose.utoc", &shared),
    ]);
    let game = fixture.root.join("game");
    let before = listing(&game);
    fixture.core.mods_base_refresh().expect("mods_base_refresh");
    for row in fixture.snapshot().content.rows.iter() {
        fixture
            .core
            .mods_contents(row.mod_ref.as_str())
            .expect("mods_contents");
    }
    assert_eq!(listing(&game), before);
}

#[test]
fn shared_with_skips_identical_copies() {
    let a = TocBuilder::new(MOUNT)
        .file_with_hash("Smoke/Same.uasset", [1; 20])
        .file_with_hash("Smoke/Other.uasset", [2; 20])
        .build();
    let b = TocBuilder::new(MOUNT)
        .file_with_hash("Smoke/Same.uasset", [1; 20])
        .file_with_hash("Smoke/Other.uasset", [3; 20])
        .build();
    let base = base_toc(&[
        "Polaris/Content/Smoke/Same.uasset",
        "Polaris/Content/Smoke/Other.uasset",
    ]);
    let fixture = ToggleFixture::new(&[
        (BASE, &base),
        ("Mods/A/A.pak", DATA),
        ("Mods/A/A.utoc", &a),
        ("Mods/B/B.pak", DATA),
        ("Mods/B/B.utoc", &b),
    ]);
    let b_ref = fixture.row("Standard:B").mod_ref;
    let found = contents(&fixture, "Standard:A");
    assert!(
        entry(&found, "Polaris/Content/Smoke/Same.uasset")
            .shared_with
            .is_empty()
    );
    assert_eq!(
        entry(&found, "Polaris/Content/Smoke/Other.uasset").shared_with,
        [b_ref]
    );
}

#[test]
fn shared_with_only_for_game_files() {
    let shared = toc(MOUNT, &["Smoke/Game.uasset", "Smoke/Library.uasset"]);
    let base = base_toc(&["Polaris/Content/Smoke/Game.uasset"]);
    let fixture = ToggleFixture::new(&[
        (BASE, &base),
        ("Mods/A/A.pak", DATA),
        ("Mods/A/A.utoc", &shared),
        ("Mods/B/B.pak", DATA),
        ("Mods/B/B.utoc", &shared),
    ]);
    let found = contents(&fixture, "Standard:A");
    assert_eq!(
        entry(&found, "Polaris/Content/Smoke/Game.uasset")
            .shared_with
            .len(),
        1
    );
    assert!(
        entry(&found, "Polaris/Content/Smoke/Library.uasset")
            .shared_with
            .is_empty()
    );
}

#[test]
fn loading_until_the_base_index_is_built() {
    let shared = toc(MOUNT, &["Smoke/Game.uasset"]);
    let base = base_toc(&["Polaris/Content/Smoke/Game.uasset"]);
    let fixture = ToggleFixture::new(&[
        (BASE, &base),
        ("Mods/A/A.pak", DATA),
        ("Mods/A/A.utoc", &shared),
        ("Mods/B/B.pak", DATA),
        ("Mods/B/B.utoc", &shared),
    ]);
    let first = fixture.snapshot();
    let a = fixture.row("Standard:A");
    assert!(a.content_conflicts.is_empty());
    let loading = fixture
        .core
        .mods_contents(a.mod_ref.as_str())
        .expect("mods_contents");
    assert_eq!(loading.base, BaseIndexState::Loading);
    assert_eq!(loading.entries[0].origin, AssetOrigin::Unknown);
    assert!(loading.entries[0].shared_with.is_empty());

    let refreshed = fixture
        .core
        .mods_base_refresh()
        .expect("mods_base_refresh")
        .expect("a new revision");
    assert!(refreshed.snapshot.revision > first.revision);
    assert_eq!(fixture.row("Standard:A").content_conflicts.len(), 1);
    assert!(
        fixture
            .core
            .mods_base_refresh()
            .expect("mods_base_refresh")
            .is_none()
    );
    assert_eq!(
        contents(&fixture, "Standard:A").base,
        BaseIndexState::Complete
    );
}
