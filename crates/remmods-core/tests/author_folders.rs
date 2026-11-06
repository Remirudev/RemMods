//! Author folders (PLAN §7): a folder in a root with no mod of its own, whose subfolders are mods.
#![allow(clippy::expect_used)]

mod mods_support;
mod support;

use mods_support::manage::{HEX, core_with_trash, mods_error, tree};
use mods_support::toggle::ToggleFixture;
use mods_support::{DATA, fixture, input, keys, paks_tree, row, scan};
use remmods_core::mods::{
    DeleteRequest, MetadataRequest, ModInfoEdit, ModIssue, ModKind, ModState, ModsError,
    RenameOutcome, RenameRequest, ScanContent, read_details, resolve_ref,
};

const PRD: &[(&str, &[u8])] = &[
    ("LogicMods/PRD/Anna/Anna.pak", DATA),
    ("LogicMods/PRD/Anna/Anna.ucas", DATA),
    ("LogicMods/PRD/Reina/Reina.pak", DATA),
];

fn content_of(entries: &[(&str, &[u8])]) -> ScanContent {
    let (_dir, paks) = fixture();
    paks_tree(&paks, entries);
    scan(&paks).content
}

fn rename(
    fixture: &ToggleFixture,
    key: &str,
    new_name: &str,
) -> Result<RenameOutcome, remmods_core::CoreError> {
    fixture.core.mods_rename(&RenameRequest {
        target: fixture.target(key),
        new_name: new_name.to_owned(),
        apply_while_running: false,
    })
}

#[test]
fn each_subfolder_is_a_row_of_its_own() {
    let content = content_of(PRD);
    assert_eq!(keys(&content), ["Logic:PRD/Anna", "Logic:PRD/Reina"]);
    let anna = row(&content, "Logic:PRD/Anna");
    assert_eq!(anna.kind, ModKind::Logic);
    assert_eq!(anna.name, "Anna");
    assert_eq!(anna.folder_name, "Anna");
    assert_eq!(anna.root, "LogicMods/PRD");
    assert_eq!(anna.state, ModState::Enabled);
    assert_eq!(anna.payload.live_pak, 1);
    assert_eq!(anna.size, Some(8));
    assert!(anna.issues.is_empty(), "{:?}", anna.issues);
    assert_eq!(anna.toggle_block, None);
}

#[test]
fn every_root_can_hold_author_folders() {
    let content = content_of(&[
        ("Mods/Me/A/a.pak", DATA),
        ("Mods/Me/B/b.pak", DATA),
        ("~mods/You/C/c.pak", DATA),
        ("~mods/You/D/d.pak", DATA),
    ]);
    assert_eq!(
        keys(&content),
        [
            "Standard:Me/A",
            "Standard:Me/B",
            "Legacy:You/C",
            "Legacy:You/D"
        ]
    );
}

#[test]
fn the_folder_is_the_author_unless_modinfo_names_one() {
    let content = content_of(&[
        ("Mods/Me/A/a.pak", DATA),
        ("Mods/Me/B/b.pak", DATA),
        (
            "Mods/Me/B/modinfo.json",
            br#"{"Author": "Someone else"}"#.as_slice(),
        ),
    ]);
    let plain = row(&content, "Standard:Me/A");
    assert_eq!(plain.info.author, "Me");
    assert!(!plain.info.defaulted.author);
    let named = row(&content, "Standard:Me/B");
    assert_eq!(named.info.author, "Someone else");
}

#[test]
fn a_folder_with_its_own_paks_stays_one_mod() {
    let content = content_of(&[
        ("Mods/Pack/Pack.pak", DATA),
        ("Mods/Pack/A/a.pak", DATA),
        ("Mods/Pack/B/b.pak", DATA),
    ]);
    assert_eq!(keys(&content), ["Standard:Pack"]);
    assert_eq!(row(&content, "Standard:Pack").root, "Mods");
    assert_eq!(row(&content, "Standard:Pack").info.author, "Unknown");
}

#[test]
fn a_wrapper_folder_stays_one_mod() {
    let content = content_of(&[
        ("Mods/Wrap/Wrap/a.pak", DATA),
        ("Mods/Wrap/notes.txt", DATA),
    ]);
    assert_eq!(keys(&content), ["Standard:Wrap"]);
}

#[test]
fn a_file_that_marks_a_mod_keeps_the_folder_one_mod() {
    for marker in ["Mods/Tool/modinfo.json", "Mods/Tool/Config.lua"] {
        let content = content_of(&[
            (marker, b"{}"),
            ("Mods/Tool/A/a.pak", DATA),
            ("Mods/Tool/B/b.pak", DATA),
        ]);
        assert_eq!(keys(&content), ["Standard:Tool"], "{marker}");
    }
}

#[test]
fn a_disabled_logic_mod_stays_in_its_author_folder() {
    let content = content_of(&[
        ("LogicMods/PRD/Anna/Anna.pak", DATA),
        ("LogicMods/PRD/.disabled_Reina/Reina.pak-x", DATA),
    ]);
    assert_eq!(keys(&content), ["Logic:PRD/Anna", "Logic:PRD/Reina"]);
    let reina = row(&content, "Logic:PRD/Reina");
    assert_eq!(reina.name, "Reina");
    assert_eq!(reina.folder_name, ".disabled_Reina");
    assert_eq!(reina.state, ModState::Disabled);
    assert!(reina.logic_prefixed);
}

#[test]
fn a_disabled_folder_is_not_an_author_folder() {
    let content = content_of(&[
        ("LogicMods/.disabled_PRD/A/a.pak-x", DATA),
        ("LogicMods/.disabled_PRD/B/b.pak-x", DATA),
    ]);
    assert_eq!(keys(&content), ["Logic:PRD"]);
    assert_eq!(row(&content, "Logic:PRD").state, ModState::Disabled);
}

#[test]
fn plain_and_prefixed_siblings_are_ambiguous_in_an_author_folder_too() {
    let content = content_of(&[
        ("LogicMods/PRD/Anna/Anna.pak", DATA),
        ("LogicMods/PRD/.disabled_Anna/Anna.pak-x", DATA),
        ("LogicMods/PRD/Reina/Reina.pak", DATA),
    ]);
    let ambiguous = content.rows.iter().filter(|row| row.ambiguous).count();
    assert_eq!(ambiguous, 2);
    assert!(!row(&content, "Logic:PRD/Reina").ambiguous);
}

#[test]
fn the_same_mod_name_under_two_authors_is_no_clash() {
    let content = content_of(&[
        ("Mods/Me/Maid/a.pak", DATA),
        ("Mods/Me/Other/b.pak", DATA),
        ("Mods/You/MAID/c.pak", DATA),
        ("Mods/You/Third/d.pak", DATA),
    ]);
    for key in ["Standard:Me/Maid", "Standard:You/MAID"] {
        let found = row(&content, key);
        assert!(!found.ambiguous);
        let collides = |issue: &ModIssue| matches!(issue, ModIssue::CaseCollision { .. });
        assert!(!found.issues.iter().any(collides), "{key}");
    }
}

#[test]
fn a_subfolder_without_paks_is_a_row_and_archives_are_listed() {
    let content = content_of(&[
        ("Mods/Me/A/a.pak", DATA),
        ("Mods/Me/B/b.pak", DATA),
        ("Mods/Me/Spare/readme.txt", DATA),
        ("Mods/Me/A.zip", DATA),
    ]);
    let spare = row(&content, "Standard:Me/Spare");
    assert_eq!(spare.state, ModState::NoPakFiles);
    assert_eq!(content.archives.len(), 1);
    assert_eq!(content.archives[0].rel_path, "Mods/Me/A.zip");
}

#[test]
fn a_set_aside_folder_is_a_leftover_and_no_mod() {
    let leftover = format!("_delete_tmp_{HEX}");
    let content = content_of(&[
        ("Mods/Me/A/a.pak", DATA),
        ("Mods/Me/B/b.pak", DATA),
        (&format!("Mods/Me/{leftover}/c.pak-x"), DATA),
    ]);
    assert_eq!(keys(&content), ["Standard:Me/A", "Standard:Me/B"]);
    assert_eq!(content.leftovers.len(), 1);
    assert_eq!(content.leftovers[0].rel_path, format!("Mods/Me/{leftover}"));

    let content = content_of(&[
        ("Mods/Me/A/a.pak", DATA),
        (&format!("Mods/Me/{leftover}/c.pak-x"), DATA),
    ]);
    assert_eq!(keys(&content), ["Standard:Me"]);
}

#[test]
fn refs_lead_back_to_the_mod_folder() {
    let (_dir, paks) = fixture();
    paks_tree(&paks, PRD);
    let content = scan(&paks).content;
    let anna = row(&content, "Logic:PRD/Anna");
    let input = input(&paks);
    let path = resolve_ref(&input, &anna.mod_ref).expect("resolve");
    assert_eq!(path, anna.path);
    assert!(path.ends_with("PRD/Anna"), "{}", path.display());
    let details = read_details(&input, &anna.mod_ref).expect("details");
    assert_eq!(details.files.len(), 2);
}

#[test]
fn a_logic_toggle_stays_in_the_author_folder() {
    let fixture = ToggleFixture::new(PRD);
    let before = fixture.names();
    fixture.toggle(&["Logic:PRD/Anna"], false).expect("disable");
    assert!(fixture.exists("LogicMods/PRD/.disabled_Anna/Anna.pak-x"));
    assert!(fixture.exists("LogicMods/PRD/.disabled_Anna/Anna.ucas-x"));
    assert!(fixture.exists("LogicMods/PRD/Reina/Reina.pak"));
    assert_eq!(fixture.row("Logic:PRD/Anna").state, ModState::Disabled);
    assert_eq!(fixture.row("Logic:PRD/Reina").state, ModState::Enabled);
    fixture.toggle(&["Logic:PRD/Anna"], true).expect("enable");
    assert_eq!(fixture.names(), before);
}

#[test]
fn a_pak_toggle_stays_in_the_author_folder() {
    let fixture = ToggleFixture::new(&[("Mods/Me/A/a.pak", DATA), ("Mods/Me/B/b.pak", DATA)]);
    let before = fixture.names();
    fixture.toggle(&["Standard:Me/A"], false).expect("disable");
    assert!(fixture.exists("Mods/Me/A/a.pak-x"));
    assert!(fixture.exists("Mods/Me/B/b.pak"));
    fixture.toggle(&["Standard:Me/A"], true).expect("enable");
    assert_eq!(fixture.names(), before);
}

#[test]
fn a_rename_stays_in_the_author_folder() {
    let fixture = core_with_trash(&[
        ("Mods/Me/Cool/Cool.pak", DATA),
        ("Mods/Me/Other/Other.pak", DATA),
        ("Mods/Free/Free.pak", DATA),
    ]);
    let renamed = rename(&fixture, "Standard:Me/Cool", "Fresh").expect("rename");
    assert_eq!(renamed.old_key, "Standard:Me/Cool");
    assert_eq!(renamed.new_key, "Standard:Me/Fresh");
    assert!(renamed.new_ref.is_some());
    assert!(fixture.exists("Mods/Me/Fresh/Cool.pak"));
    assert!(!fixture.exists("Mods/Fresh"));
    // Only the folder's own neighbours count: a name in use in the root is free here.
    rename(&fixture, "Standard:Me/Fresh", "Free").expect("free in this folder");
    assert!(fixture.exists("Mods/Me/Free/Cool.pak"));
    let before = tree(&fixture);
    assert!(matches!(
        mods_error(rename(&fixture, "Standard:Me/Free", "Other")),
        ModsError::TargetExists
    ));
    assert_eq!(tree(&fixture), before);
}

#[test]
fn a_delete_takes_only_that_mod_and_leaves_nothing_behind() {
    let fixture = core_with_trash(&[
        ("Mods/Me/A/a.pak", DATA),
        ("Mods/Me/B/b.pak", DATA),
        ("Mods/Me/C/c.pak", DATA),
    ]);
    let target = fixture.target("Standard:Me/A");
    let prepared = fixture.core.mods_delete_prepare(&target).expect("prepare");
    let outcome = fixture
        .core
        .mods_delete_permanently(&DeleteRequest {
            target,
            token: prepared.token,
            apply_while_running: false,
        })
        .expect("delete");
    assert_eq!(outcome.leftover, None);
    let names: Vec<String> = fixture
        .names()
        .into_iter()
        .map(|(name, _, _)| name)
        .collect();
    assert!(!names.iter().any(|name| name.contains("Mods/Me/A")));
    assert!(
        !names.iter().any(|name| name.contains("_tmp_")),
        "{names:?}"
    );
    assert!(fixture.exists("Mods/Me/B/b.pak"));
    assert!(fixture.exists("Mods/Me/C/c.pak"));
}

#[test]
fn metadata_is_written_into_the_mod_folder() {
    let fixture = core_with_trash(&[("Mods/Me/A/a.pak", DATA), ("Mods/Me/B/b.pak", DATA)]);
    let info = ModInfoEdit {
        version: "2.0".to_owned(),
        category: "Outfit".to_owned(),
        description: String::new(),
        author: "Somebody".to_owned(),
    };
    fixture
        .core
        .mods_set_metadata(&MetadataRequest {
            target: fixture.target("Standard:Me/A"),
            info,
        })
        .expect("write");
    assert!(fixture.exists("Mods/Me/A/modinfo.json"));
    assert!(!fixture.exists("Mods/Me/modinfo.json"));
    assert_eq!(fixture.row("Standard:Me/A").info.author, "Somebody");
    assert_eq!(fixture.row("Standard:Me/B").info.author, "Me");
}
