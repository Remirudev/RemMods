//! The Repair list (spec §5.6, §8.2 P-R1..P-R4).
#![allow(clippy::expect_used)]

mod mods_support;
mod support;

use std::collections::HashSet;

use mods_support::DATA;
use mods_support::manage::{HEX, core_with_trash};
use proptest::prelude::*;
use remmods_core::mods::{RepairAction as A, RepairKind as K, repair_items, suggest_name};
use remmods_core::safe_fs::names::{fold_key, validate_name};

#[test]
fn repair_items_table() {
    let leftover = format!("Mods/_Old_tmp_{HEX}");
    let live_temp = format!("Mods/_Half_tmp_{HEX}");
    let fixture = core_with_trash(&[
        (&format!("{leftover}/Old.pak-x"), DATA),
        (&format!("{live_temp}/Half.pak"), DATA),
        ("LogicMods/A/a.pak", DATA),
        ("LogicMods/.disabled_A/a.pak-x", DATA),
        ("Mods/Bad./b.pak", DATA),
        ("LogicMods/.disabled_D/d.pak", DATA),
        ("LogicMods/E/e.pak-x", DATA),
    ]);
    let snapshot = fixture.snapshot();
    let items = repair_items(&snapshot.content, 1);
    let table: Vec<(K, String, Vec<A>, Option<String>)> = items
        .iter()
        .map(|item| {
            (
                item.kind,
                item.label.clone(),
                item.actions.clone(),
                item.suggested_name.clone(),
            )
        })
        .collect();
    let expected = vec![
        (
            K::Leftover,
            leftover.clone(),
            vec![A::Trash, A::Reveal],
            None,
        ),
        (K::TempFolderLive, live_temp.clone(), vec![A::Reveal], None),
        (
            K::AmbiguousKey,
            "A".into(),
            vec![A::Rename, A::Trash, A::Reveal],
            None,
        ),
        (
            K::AmbiguousKey,
            "A".into(),
            vec![A::Rename, A::Trash, A::Reveal],
            None,
        ),
        (
            K::InvalidName,
            "Bad.".into(),
            vec![A::Rename, A::Trash],
            Some("Bad".into()),
        ),
        (
            K::LogicDisabledButLoaded,
            "D".into(),
            vec![A::FinishDisable, A::Reveal],
            None,
        ),
        (
            K::LogicEnabledNotLoaded,
            "E".into(),
            vec![A::FinishEnable, A::Reveal],
            None,
        ),
        (K::StuckJournal, String::new(), vec![A::OpenLogs], None),
    ];
    assert_eq!(table, expected);
    let ids: HashSet<&str> = items.iter().map(|item| item.id.as_str()).collect();
    assert_eq!(ids.len(), items.len(), "ids are unique");
    let without_stuck = repair_items(&snapshot.content, 0);
    assert_eq!(
        without_stuck[0].actions,
        [A::Trash, A::DeletePermanently, A::Reveal]
    );
    assert!(
        without_stuck
            .iter()
            .all(|item| item.kind != K::StuckJournal)
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]
    #[test]
    fn suggest_name_passes_validate(name in "\\PC{0,300}|[ .<>:\"/\\\\|?*\u{202E}\u{1}a-zA-Z_]{0,40}") {
        let suggested = suggest_name(&name, &HashSet::new());
        prop_assert!(validate_name(&suggested).is_ok(), "{:?} → {:?}", name, suggested);
    }
}

#[test]
fn suggest_name_avoids_taken() {
    let taken: HashSet<String> = ["cool", "cool (2)"]
        .iter()
        .map(|name| fold_key(name))
        .collect();
    assert_eq!(suggest_name("Cool", &taken), "Cool (3)");
    assert_eq!(suggest_name("CON", &HashSet::new()), "CON_");
    assert_eq!(suggest_name("con.txt", &HashSet::new()), "con_.txt");
    assert_eq!(suggest_name(".disabled_X", &HashSet::new()), "X");
    assert_eq!(suggest_name("a. .", &HashSet::new()), "a");
    assert_eq!(suggest_name("", &HashSet::new()), "Mod");
    let temp = format!("_x_tmp_{HEX}");
    assert_eq!(suggest_name(&temp, &HashSet::new()), format!("{temp}_"));
    let long = "\u{65E5}".repeat(100);
    let cut = suggest_name(&long, &HashSet::new());
    assert!(cut.len() <= 255 && validate_name(&cut).is_ok());
}

#[test]
fn stuck_journal_item() {
    let fixture = core_with_trash(&[("Mods/Cool/Cool.pak", DATA)]);
    let content = fixture.snapshot().content.clone();
    assert!(repair_items(&content, 0).is_empty());
    let items = repair_items(&content, 2);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "stuck");
    assert_eq!(items[0].target, None);
    let list = fixture.core.repair_list().expect("repair list");
    assert!(list.items.is_empty());
}
