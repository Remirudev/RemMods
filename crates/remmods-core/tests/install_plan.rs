//! Pure install planning (P4 spec §8.2 P1–P8, P11 fix_name, P18, P19): root detection,
//! wrappers and candidates, name fixes, source classification (the symlink case is in
//! `platform_linux_install.rs`).
#![allow(clippy::expect_used, clippy::disallowed_methods)]

mod support;

use std::fs;

use proptest::prelude::*;
use remmods_core::install::{
    IssueCode, PlanFile, RootReason, SourceKind, SourceReject, classify_source, detect_root,
    fix_name, plan_candidates,
};
use remmods_core::mods::RootKind;

fn paths(list: &[&str]) -> Vec<Vec<String>> {
    list.iter()
        .map(|path| path.split('/').map(str::to_owned).collect())
        .collect()
}

fn plan_files(list: &[&str]) -> Vec<PlanFile> {
    paths(list)
        .into_iter()
        .map(|path| PlanFile { path })
        .collect()
}

#[test]
fn p1_path_segment_legacy() {
    let detection = detect_root(&paths(&["Content/Paks/~mods/Cool/x.pak", "readme.txt"]));
    assert_eq!(detection.kind, RootKind::Legacy);
    assert_eq!(detection.reason, RootReason::PathSegment);
    assert_eq!(
        detection.stripped[0],
        Some(vec!["Cool".to_owned(), "x.pak".to_owned()])
    );
    assert_eq!(detection.stripped[1], None);
    assert_eq!(detection.outside_root, 1);
    assert!(detection.blocks.is_empty());
}

#[test]
fn p2_logic_segment_and_mixed_roots() {
    let detection = detect_root(&paths(&["LogicMods/X/x.pak"]));
    assert_eq!(detection.kind, RootKind::Logic);
    let mixed = detect_root(&paths(&["Mods/A/a.pak", "LogicMods/B/b.pak"]));
    assert!(mixed.blocks.contains(&IssueCode::MixedRoots));
    let partial = detect_root(&paths(&["Mods/A/a.pak", "B/b.pak"]));
    assert!(partial.blocks.contains(&IssueCode::MixedRoots));
}

#[test]
fn p3_config_lua_and_script_mods() {
    let logic = detect_root(&paths(&["Cool/config.lua", "Cool/x.pak"]));
    assert_eq!(
        (logic.kind, logic.reason),
        (RootKind::Logic, RootReason::ConfigLua)
    );
    assert!(logic.blocks.is_empty());
    for script in [&["Scripts/main.lua"][..], &["dlls/main.dll", "x.pak"][..]] {
        let detection = detect_root(&paths(script));
        assert!(
            detection.blocks.contains(&IssueCode::ScriptMod),
            "{script:?}"
        );
    }
}

#[test]
fn p4_default_standard() {
    let detection = detect_root(&paths(&["Cool/x.pak"]));
    assert_eq!(
        (detection.kind, detection.reason),
        (RootKind::Standard, RootReason::Default)
    );
}

#[test]
fn p5_wrappers_are_stripped() {
    for list in [&["Cool/Cool/x.pak"][..], &["Pack v2/Cool/x.pak"][..]] {
        let plan = plan_candidates(&plan_files(list), "Archive");
        assert_eq!(plan.candidates.len(), 1);
        assert_eq!(plan.candidates[0].name, "Cool");
        assert_eq!(plan.candidates[0].files[0].1, vec!["x.pak".to_owned()]);
    }
}

#[test]
fn p6_top_level_payload_takes_the_archive_stem() {
    let plan = plan_candidates(&plan_files(&["x.pak", "x.ucas"]), "My Mod");
    assert_eq!(plan.candidates.len(), 1);
    assert_eq!(plan.candidates[0].name, "My Mod");
    assert!(plan.candidates[0].inner.is_empty());
}

#[test]
fn p7_variants_and_separate_mods() {
    let plan = plan_candidates(
        &plan_files(&["Red/x.pak", "Blue/x.pak", "Preview/p.png"]),
        "Pack",
    );
    let names: Vec<&str> = plan.candidates.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["Blue", "Red"]);
    assert_eq!(plan.groups.len(), 1);
    assert!(plan.candidates[0].selected && !plan.candidates[1].selected);
    assert_eq!(plan.extra_files, vec![2]);
    let separate = plan_candidates(&plan_files(&["A/a.pak", "B/b.pak"]), "Pack");
    assert!(separate.groups.is_empty());
    assert!(
        separate
            .candidates
            .iter()
            .all(|candidate| candidate.selected)
    );
}

#[test]
fn p8_no_payload() {
    let detection = detect_root(&paths(&["readme.txt"]));
    assert!(detection.blocks.contains(&IssueCode::NoPayload));
}

#[test]
fn p11_fix_name_rules() {
    assert_eq!(fix_name("CON.pak"), "CON_.pak");
    assert_eq!(fix_name("a:b.pak"), "a_b.pak");
    assert_eq!(fix_name("a:b.ucas"), "a_b.ucas");
    assert_eq!(fix_name("name."), "name");
    assert_eq!(fix_name("bad\u{202E}name"), "bad_name");
    let long = format!("{}.pak", "x".repeat(250));
    assert_eq!(long.len(), 254);
    let fixed = fix_name(&long);
    assert_eq!(fixed.len(), 253);
    assert!(fixed.ends_with(".pak"));
    let long_text = format!("{}.txt", "é".repeat(200));
    let fixed = fix_name(&long_text);
    assert!(fixed.len() <= 255 && fixed.ends_with(".txt"));
}

#[test]
fn p19_classify_source() {
    let (_dir, root) = support::temp_root();
    let paks = root.join("Paks");
    support::write_tree(
        &root,
        &[
            ("Cool.ZIP", b"z"),
            ("tool.exe", b"e"),
            ("Paks/Mods/Inside/", b""),
            ("Folder/", b""),
        ],
    );
    assert_eq!(
        classify_source(&root.join("Cool.ZIP"), Some(&paks)),
        Ok(SourceKind::Zip)
    );
    assert_eq!(
        classify_source(&root.join("tool.exe"), Some(&paks)),
        Err(SourceReject::UnsupportedType)
    );
    assert_eq!(
        classify_source(&root.join("missing.7z"), Some(&paks)),
        Err(SourceReject::Missing)
    );
    assert_eq!(
        classify_source(&paks.join("Mods").join("Inside"), Some(&paks)),
        Err(SourceReject::InsidePaks)
    );
    assert_eq!(
        classify_source(&root, Some(&paks)),
        Err(SourceReject::InsidePaks)
    );
    assert_eq!(
        classify_source(&root.join("Folder"), Some(&paks)),
        Ok(SourceKind::Folder)
    );
}

fn sample() -> impl Strategy<Value = Vec<String>> {
    let parts = prop::sample::select(vec![
        "Mods",
        "~mods",
        "Cool",
        "cool",
        "Red",
        "Blue",
        "x.pak",
        "x.ucas",
        "readme.txt",
        "config.lua",
        "Wrap",
    ]);
    prop::collection::vec(prop::collection::vec(parts, 1..4), 1..8).prop_map(|list| {
        list.into_iter()
            .map(|parts| parts.join("/"))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    })
}

proptest! {
    #[test]
    fn p18_install_plan_props(list in sample(), seed in any::<u64>()) {
        let refs: Vec<&str> = list.iter().map(String::as_str).collect();
        let mut shuffled = refs.clone();
        let len = shuffled.len();
        for i in (1..len).rev() {
            let j = usize::try_from(seed.rotate_left(u32::try_from(i).unwrap_or(0)) % (i as u64 + 1)).unwrap_or(0);
            shuffled.swap(i, j);
        }
        let a = detect_root(&paths(&refs));
        let b = detect_root(&paths(&shuffled));
        prop_assert_eq!((a.kind, a.reason, a.outside_root, &a.blocks), (b.kind, b.reason, b.outside_root, &b.blocks));
        let mut kept_a: Vec<_> = a.stripped.iter().flatten().cloned().collect();
        let mut kept_b: Vec<_> = b.stripped.iter().flatten().cloned().collect();
        kept_a.sort();
        kept_b.sort();
        prop_assert_eq!(&kept_a, &kept_b);
        let files_a: Vec<PlanFile> = kept_a.iter().cloned().map(|path| PlanFile { path }).collect();
        let mut files_b = files_a.clone();
        files_b.reverse();
        let plan_a = plan_candidates(&files_a, "Stem");
        let plan_b = plan_candidates(&files_b, "Stem");
        let view = |plan: &remmods_core::install::CandidatePlan, files: &[PlanFile]| {
            plan.candidates
                .iter()
                .map(|c| {
                    let members: Vec<_> = c.files.iter().map(|(index, rel)| (files[*index].path.clone(), rel.clone())).collect();
                    (c.id, c.name.clone(), c.inner.clone(), members, c.group, c.selected)
                })
                .collect::<Vec<_>>()
        };
        prop_assert_eq!(view(&plan_a, &files_a), view(&plan_b, &files_b));
        prop_assert_eq!(&plan_a.groups, &plan_b.groups);
        let extra = |plan: &remmods_core::install::CandidatePlan, files: &[PlanFile]| {
            plan.extra_files.iter().map(|index| files[*index].path.clone()).collect::<Vec<_>>()
        };
        prop_assert_eq!(extra(&plan_a, &files_a), extra(&plan_b, &files_b));
    }
}

#[test]
fn p19_seven_zip_without_paks() {
    let (_dir, root) = support::temp_root();
    fs::write(root.join("a.7z"), b"x").expect("write");
    assert_eq!(
        classify_source(&root.join("a.7z"), None),
        Ok(SourceKind::SevenZip)
    );
}
