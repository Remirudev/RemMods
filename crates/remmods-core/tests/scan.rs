//! scan_paks rules (spec §4.5, §4.6) and ModsService (§4.8).

mod mods_support;
mod support;

use std::sync::Arc;
use std::thread;

use mods_support::{DATA, fixture, input, keys, paks_tree, row, scan, smoke_tree};
use remmods_core::mods::{
    ModInfoOrigin, ModIssue, ModKind, ModState, ModsService, NameProblem, ScanContent,
    ScanSnapshot, ToggleBlock, UnmanagedKind, paks_identity,
};

fn content_of(entries: &[(&str, &[u8])]) -> ScanContent {
    let (_dir, paks) = fixture();
    paks_tree(&paks, entries);
    scan(&paks).content
}

fn has(content: &ScanContent, key: &str, issue: &ModIssue) -> bool {
    row(content, key).issues.contains(issue)
}

#[test]
fn live_and_switched_off_paks_are_partial() {
    let content = content_of(&[("Mods/P/a.pak", DATA), ("Mods/P/b.pak-x", DATA)]);
    let partial = row(&content, "Standard:P");
    assert_eq!(partial.state, ModState::Partial);
    assert_eq!(partial.payload.live_pak, 1);
    assert_eq!(partial.payload.off_pak, 1);
}

#[test]
fn loose_group_becomes_one_row() {
    let content = content_of(&[
        ("Mods/Loose.pak", DATA),
        ("Mods/Loose.ucas", DATA),
        ("Mods/Loose.utoc", DATA),
    ]);
    let loose = row(&content, "Loose:Mods/Loose");
    assert_eq!(loose.kind, ModKind::Loose);
    assert_eq!(loose.state, ModState::Enabled);
    assert_eq!(loose.size, Some(12));
    assert_eq!(loose.info.origin, ModInfoOrigin::Missing);
    assert_eq!(content.rows.len(), 1);
}

#[test]
fn loose_case_variants_share_a_group() {
    let content = content_of(&[("Mods/Foo.pak", DATA), ("Mods/foo.ucas", DATA)]);
    assert_eq!(keys(&content), ["Loose:Mods/Foo"]);
    assert!(has(
        &content,
        "Loose:Mods/Foo",
        &ModIssue::LooseCaseVariants
    ));
}

#[test]
fn a_lone_sig_is_not_a_row() {
    let content = content_of(&[("Mods/Alone.sig", DATA)]);
    assert!(content.rows.is_empty());
}

#[test]
fn archives_only_in_paks_and_roots() {
    let content = content_of(&[
        ("Top.zip", DATA),
        ("Mods/Pack.7z", DATA),
        ("Mods/Inner/Nested.rar", DATA),
        ("Mods/Inner/a.pak", DATA),
    ]);
    let archives: Vec<&str> = content
        .archives
        .iter()
        .map(|a| a.rel_path.as_str())
        .collect();
    assert_eq!(archives, ["Mods/Pack.7z", "Top.zip"]);
    assert_eq!(row(&content, "Standard:Inner").size, Some(8));
}

#[test]
fn game_files_are_ignored_and_strays_reported() {
    let content = content_of(&[
        ("pakchunk0-Windows.pak", DATA),
        ("pakchunk12-WindowsNoEditor.utoc", DATA),
        ("global.ucas", DATA),
        ("Stray.pak", DATA),
        ("Backup/x/y.pak", DATA),
    ]);
    let unmanaged: Vec<(UnmanagedKind, &str, u32)> = content
        .unmanaged
        .iter()
        .map(|e| (e.kind, e.rel_path.as_str(), e.live_files))
        .collect();
    assert_eq!(
        unmanaged,
        [
            (UnmanagedKind::Folder, "Backup", 1),
            (UnmanagedKind::PaksFile, "Stray.pak", 1)
        ]
    );
}

#[test]
fn disabled_prefix_in_pak_roots_is_an_ordinary_name() {
    let content = content_of(&[("Mods/.disabled_X/a.pak", DATA)]);
    let ordinary = row(&content, "Standard:.disabled_X");
    assert_eq!(ordinary.state, ModState::Enabled);
    assert!(ordinary.issues.contains(&ModIssue::DisabledPrefixName));
}

#[test]
fn logic_state_table() {
    let content = content_of(&[
        ("LogicMods/E/a.pak", DATA),
        ("LogicMods/P/a.pak", DATA),
        ("LogicMods/P/b.pak-x", DATA),
        ("LogicMods/D/b.pak-x", DATA),
        ("LogicMods/N/main.lua", DATA),
        ("LogicMods/.disabled_Q/b.pak-x", DATA),
        ("LogicMods/.disabled_L/a.pak", DATA),
    ]);
    let state = |key: &str| row(&content, key).state;
    assert_eq!(state("Logic:E"), ModState::Enabled);
    assert_eq!(state("Logic:P"), ModState::Partial);
    assert_eq!(state("Logic:D"), ModState::Disabled);
    assert!(has(&content, "Logic:D", &ModIssue::LogicEnabledNotLoaded));
    assert_eq!(state("Logic:N"), ModState::NoPakFiles);
    assert_eq!(state("Logic:Q"), ModState::Disabled);
    assert_eq!(row(&content, "Logic:Q").issues, []);
    assert_eq!(state("Logic:L"), ModState::Disabled);
    assert!(has(&content, "Logic:L", &ModIssue::LogicDisabledButLoaded));
}

#[test]
fn bare_disabled_prefix_has_an_empty_name() {
    let content = content_of(&[("LogicMods/.disabled_/a.pak-x", DATA)]);
    let empty = row(&content, "Logic:");
    assert_eq!(empty.name, "");
    assert!(empty.issues.contains(&ModIssue::EmptyName));
}

#[test]
fn toggle_block_first_match_wins() {
    let content = content_of(&[
        ("LogicMods/X/a.pak", DATA),
        ("LogicMods/.disabled_X/a.pak-x", DATA),
        ("LogicMods/.disabled_/a.pak-x", DATA),
        ("LogicMods/Ok/a.pak", DATA),
        ("Mods/Empty/readme.txt", DATA),
        ("Mods/On/a.pak", DATA),
        ("Mods/Off/a.pak-x", DATA),
        ("Mods/Half/a.pak", DATA),
        ("Mods/Half/b.pak-x", DATA),
    ]);
    for ambiguous in content.rows.iter().filter(|row| row.key == "Logic:X") {
        assert_eq!(ambiguous.toggle_block, Some(ToggleBlock::Ambiguous));
    }
    let expected = [
        ("Logic:", Some(ToggleBlock::InvalidLogicName)),
        ("Logic:Ok", None),
        ("Standard:Empty", Some(ToggleBlock::NoPakFiles)),
        ("Standard:On", None),
        ("Standard:Off", None),
        ("Standard:Half", None),
    ];
    for (key, block) in expected {
        assert_eq!(row(&content, key).toggle_block, block, "{key}");
    }
}

#[test]
fn prefixed_and_plain_logic_folders_are_ambiguous() {
    let content = content_of(&[
        ("LogicMods/X/a.pak", DATA),
        ("LogicMods/.disabled_X/a.pak-x", DATA),
    ]);
    assert_eq!(content.rows.len(), 2);
    for logic in &content.rows {
        assert!(logic.ambiguous);
        assert!(logic.issues.contains(&ModIssue::AmbiguousKey));
    }
}

#[test]
fn ambiguous_logic_rows_keep_their_order() {
    let entries: &[(&str, &[u8])] = &[
        ("LogicMods/X/a.pak", DATA),
        ("LogicMods/.disabled_X/a.pak-x", DATA),
        ("LogicMods/.DISABLED_X/a.pak-x", DATA),
    ];
    let folders = |content: &ScanContent| -> Vec<String> {
        content.rows.iter().map(|r| r.folder_name.clone()).collect()
    };
    let first = folders(&content_of(entries));
    // A case-insensitive file system (Windows) keeps `.DISABLED_X` and `.disabled_X` as one folder.
    let expected: &[&str] = if first.len() == 3 {
        &[".DISABLED_X", ".disabled_X", "X"]
    } else {
        &[".disabled_X", "X"]
    };
    assert_eq!(first, expected);
    for _ in 0..3 {
        assert_eq!(folders(&content_of(entries)), first);
    }
}

#[test]
fn modinfo_casing_variant_is_read() {
    let content = content_of(&[
        ("Mods/M/a.pak", DATA),
        ("Mods/M/ModInfo.json", br#"{"Author":"Rem"}"#),
    ]);
    let info = &row(&content, "Standard:M").info;
    assert_eq!(info.author, "Rem");
    assert_eq!(info.origin, ModInfoOrigin::File);
}

#[test]
fn same_names_in_different_roots_do_not_collide() {
    let content = content_of(&[("Mods/Foo.pak", DATA), ("~mods/foo.pak", DATA)]);
    assert_eq!(keys(&content), ["Loose:Mods/Foo", "Loose:~mods/foo"]);
    for loose in &content.rows {
        assert!(
            !loose
                .issues
                .iter()
                .any(|issue| matches!(issue, ModIssue::CaseCollision { .. }))
        );
    }
}

#[test]
fn loose_keys_name_the_root_kind() {
    let content = content_of(&[("Mods/X.pak", DATA), ("~mods/X.pak", DATA)]);
    assert_eq!(keys(&content), ["Loose:Mods/X", "Loose:~mods/X"]);
}

#[test]
fn long_names_are_not_portable() {
    let logic_name = "L".repeat(246);
    let pak_name = format!("{}.pak", "x".repeat(250));
    let logic_path = format!("LogicMods/{logic_name}/a.pak");
    let inner_path = format!("Mods/Inner/{pak_name}");
    let loose_path = format!("~mods/{pak_name}");
    let content = content_of(&[
        (logic_path.as_str(), DATA),
        (inner_path.as_str(), DATA),
        (loose_path.as_str(), DATA),
    ]);
    let too_long = ModIssue::NameNotPortable {
        problem: NameProblem::TooLong,
    };
    assert!(has(&content, &format!("Logic:{logic_name}"), &too_long));
    assert!(has(
        &content,
        "Standard:Inner",
        &ModIssue::NonPortableFileNames { count: 1 }
    ));
    let loose_key = format!("Loose:~mods/{}", "x".repeat(250));
    assert!(has(&content, &loose_key, &too_long));
}

#[test]
fn every_directory_is_read_once() {
    let (_dir, paks) = fixture();
    let out = paks.parent().expect("parent").join("smoke");
    let smoke_paks = smoke_tree(&out);
    let stats = scan(&smoke_paks).stats;
    // Paks, Mods, ~mods, LogicMods, Alpha, Beta, Gamma, Gamma/Content, Gamma/Content/Paks, Delta,
    // Epsilon, Epsilon/Sub, Empty, _Half_tmp, _Old_tmp, Legacy, Tool, .disabled_Off.
    assert_eq!(stats.dirs_read, 18);
}

#[test]
fn order_is_deterministic() {
    let (_dir, paks) = fixture();
    let out = paks.parent().expect("parent").join("smoke");
    let smoke_paks = smoke_tree(&out);
    let first = scan(&smoke_paks).content;
    assert_eq!(scan(&smoke_paks).content, first);
    assert_eq!(
        keys(&first),
        [
            "Standard:Alpha",
            "Standard:Beta",
            "Standard:Delta",
            "Standard:Empty",
            "Standard:Epsilon",
            "Standard:Gamma",
            "Legacy:Legacy",
            "Logic:Off",
            "Logic:Tool",
            "Loose:Mods/Loose",
        ]
    );
}

#[test]
fn long_nested_paths_warn() {
    let deep = format!("Mods/Deep/{}/{}.pak", "d".repeat(120), "f".repeat(100));
    let content = content_of(&[(deep.as_str(), DATA)]);
    let longest = row(&content, "Standard:Deep")
        .issues
        .iter()
        .find_map(|issue| match issue {
            ModIssue::PathTooLong { longest } => Some(*longest),
            _ => None,
        });
    // 49 + 1 + "Mods\Deep\" (10) + 120 + 1 + 104 + 2 for "-x".
    assert_eq!(longest, Some(287));
}

#[test]
fn scanned_at_is_rfc3339_with_milliseconds() {
    let (_dir, paks) = fixture();
    let service = ModsService::new(paks.with_file_name("mod-meta.json"));
    let snapshot: Arc<ScanSnapshot> = service.list(&input(&paks)).expect("list");
    let text = snapshot.scanned_at_rfc3339();
    assert!(text.ends_with('Z'), "{text}");
    let fraction = text.rsplit('.').next().expect("fraction");
    assert_eq!(fraction.len(), 4, "{text}");
}

#[test]
fn service_publishes_revisions() {
    let (_dir, paks) = fixture();
    paks_tree(&paks, &[("Mods/A/a.pak", DATA)]);
    let service = ModsService::new(paks.with_file_name("mod-meta.json"));
    let scan_input = input(&paks);
    let first = service.refresh(&scan_input).expect("first");
    assert!(first.changed);
    assert_eq!(first.snapshot.revision, 1);
    let again = service.refresh(&scan_input).expect("again");
    assert!(!again.changed);
    assert_eq!(again.snapshot.revision, 1);
    paks_tree(&paks, &[("Mods/A/b.pak", DATA)]);
    let changed = service.refresh(&scan_input).expect("changed");
    assert!(changed.changed);
    assert_eq!(changed.snapshot.revision, 2);
    assert_eq!(service.last().map(|s| s.revision), Some(2));
    let other_identity = remmods_core::mods::ScanInput {
        paks_id: format!("{}-other", paks_identity(&paks, 0)),
        ..scan_input.clone()
    };
    let rescanned = service.list(&other_identity).expect("other identity");
    assert_eq!(rescanned.content.paks_id, other_identity.paks_id);
    assert_eq!(
        service.list(&scan_input).expect("back").content.paks_id,
        scan_input.paks_id
    );
}

#[test]
fn concurrent_refreshes_share_one_result() {
    let (_dir, paks) = fixture();
    paks_tree(&paks, &[("Mods/A/a.pak", DATA)]);
    let service = Arc::new(ModsService::new(paks.with_file_name("mod-meta.json")));
    let scan_input = input(&paks);
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let service = Arc::clone(&service);
            let scan_input = scan_input.clone();
            thread::spawn(move || service.refresh(&scan_input).expect("refresh"))
        })
        .collect();
    for handle in handles {
        let refreshed = handle.join().expect("join");
        assert_eq!(refreshed.snapshot.revision, 1);
    }
    assert_eq!(service.last().map(|s| s.revision), Some(1));
}
