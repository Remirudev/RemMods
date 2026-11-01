//! Content summaries and content conflicts from crafted `.utoc` files (SPEC-P2B §8.2).

mod mods_support;
mod support;

use mods_support::{BASE_TOC, DATA, MOUNT, base_toc, fixture, paks_tree, row, scan_based, toc};
use remmods_core::content::{CONFLICT_SAMPLE, ContainerStatus, ContainerSummary};
use remmods_core::mods::{ModIssue, ModState, ScanContent};
use remmods_pak::fixture::TocBuilder;

/// The base game of these tests: the files the conflict tests share replace game files.
const GAME_FILES: &[&str] = &[
    "Polaris/Content/S/F1.uasset",
    "Polaris/Content/S/F2.uasset",
    "Polaris/Content/S/F3.uasset",
    "Polaris/Content/S/F4.uasset",
    "Polaris/Content/S/F5.uasset",
    "Polaris/Content/S/F6.uasset",
    "Polaris/Content/S/F7.uasset",
    "Polaris/Content/Smoke/Shared.uasset",
    "Polaris/Content/Common_umin/Materials/M_Skin.uasset",
    "Polaris/Content/X/Tex.uasset",
    "Polaris/Content/Ker/Upper.uasset",
    "Polaris/Content/Cmn/Common.uasset",
    "Polaris/Content/Smoke/One.uasset",
];

fn scanned_with_base(base: &[u8], entries: &[(&str, &[u8])]) -> ScanContent {
    let (_dir, paks) = fixture();
    paks_tree(&paks, &[(BASE_TOC, base)]);
    paks_tree(&paks, entries);
    scan_based(&paks).content
}

fn scanned(entries: &[(&str, &[u8])]) -> ScanContent {
    scanned_with_base(&base_toc(GAME_FILES), entries)
}

fn linked(content: &ScanContent, key: &str) -> Vec<(String, u32)> {
    row(content, key)
        .content_conflicts
        .iter()
        .map(|link| (link.key.clone(), link.shared))
        .collect()
}

fn link_to(key: &str, shared: u32) -> Vec<(String, u32)> {
    vec![(key.to_owned(), shared)]
}

#[test]
fn row_summary_counts_assets_and_containers() {
    let main = toc(MOUNT, &["X/One.uasset", "X/One.ubulk", "X/Two.uasset"]);
    let option = toc(MOUNT, &["X/Three.uasset"]);
    let content = scanned(&[
        ("Mods/A/A.pak", DATA),
        ("Mods/A/A.utoc", &main),
        ("Mods/A/Opt/Opt.pak-x", DATA),
        ("Mods/A/Opt/Opt.utoc-x", &option),
    ]);
    let summary = &row(&content, "Standard:A").content;
    assert_eq!(
        summary.containers,
        [
            ContainerSummary {
                rel_path: "A.utoc".to_owned(),
                live: true,
                status: ContainerStatus::Read,
                version: Some(5),
                files: 3,
            },
            ContainerSummary {
                rel_path: "Opt/Opt.utoc-x".to_owned(),
                live: false,
                status: ContainerStatus::Read,
                version: Some(5),
                files: 1,
            },
        ]
    );
    assert_eq!(summary.assets, 3);
    assert_eq!(summary.internal_overlaps, 0);
}

#[test]
fn hints_aggregate_per_row() {
    let first = toc(
        MOUNT,
        &[
            "Character/Item/Base_Item/ker/bdf/BI_ker_bdf_sb.uasset",
            "Smoke/Plain.uasset",
        ],
    );
    let second = toc(MOUNT, &["Character/Item/Base_Item/zbr/har/T_Hair.uasset"]);
    let content = scanned(&[
        ("Mods/H/H.pak", DATA),
        ("Mods/H/H.utoc", &first),
        ("Mods/H/H2.utoc-x", &second),
    ]);
    let summary = &row(&content, "Standard:H").content;
    assert_eq!(summary.characters, ["ker", "zbr"]);
    assert_eq!(summary.slots, ["bdf", "har"]);
}

#[test]
fn disabled_row_contents_are_listed_but_never_conflict() {
    let shared = toc(MOUNT, &["Smoke/Shared.uasset"]);
    let content = scanned(&[
        ("Mods/On/On.pak", DATA),
        ("Mods/On/On.utoc", &shared),
        ("Mods/Off/Off.pak-x", DATA),
        ("Mods/Off/Off.utoc-x", &shared),
    ]);
    let off = row(&content, "Standard:Off");
    assert_eq!(off.state, ModState::Disabled);
    assert_eq!(off.content.assets, 1);
    assert!(off.content_conflicts.is_empty());
    assert!(row(&content, "Standard:On").content_conflicts.is_empty());
}

#[test]
fn two_enabled_rows_sharing_an_asset_link_both_ways() {
    let shared: Vec<String> = (1..=7).map(|i| format!("S/F{i}.uasset")).collect();
    let mut a_files: Vec<&str> = shared.iter().map(String::as_str).collect();
    a_files.push("A/Own.uasset");
    let mut b_files: Vec<&str> = shared.iter().map(String::as_str).collect();
    b_files.push("B/Own.uasset");
    let a = toc(MOUNT, &a_files);
    let b = toc(MOUNT, &b_files);
    let c = toc(MOUNT, &["S/F1.uasset"]);
    let content = scanned(&[
        ("Mods/Cee/Cee.pak", DATA),
        ("Mods/Cee/Cee.utoc", &c),
        ("Mods/Bee/Bee.pak", DATA),
        ("Mods/Bee/Bee.utoc", &b),
        ("Mods/Aye/Aye.pak", DATA),
        ("Mods/Aye/Aye.utoc", &a),
    ]);
    assert_eq!(
        linked(&content, "Standard:Aye"),
        [
            ("Standard:Bee".to_owned(), 7),
            ("Standard:Cee".to_owned(), 1)
        ]
    );
    assert_eq!(
        linked(&content, "Standard:Bee"),
        [
            ("Standard:Aye".to_owned(), 7),
            ("Standard:Cee".to_owned(), 1)
        ]
    );
    assert_eq!(
        linked(&content, "Standard:Cee"),
        [
            ("Standard:Aye".to_owned(), 1),
            ("Standard:Bee".to_owned(), 1)
        ]
    );
    let aye = row(&content, "Standard:Aye");
    let to_bee = &aye.content_conflicts[0];
    assert_eq!(to_bee.sample.len(), CONFLICT_SAMPLE);
    let expected: Vec<String> = (1..=5)
        .map(|i| format!("Polaris/Content/S/F{i}.uasset"))
        .collect();
    assert_eq!(to_bee.sample, expected);
    assert_eq!(to_bee.name, "Bee");
    assert_eq!(to_bee.other, row(&content, "Standard:Bee").mod_ref);
}

#[test]
fn case_variant_paths_conflict() {
    let upper = toc(MOUNT, &["Common_umin/Materials/M_Skin.uasset"]);
    let lower = toc(MOUNT, &["Common_umin/materials/M_Skin.uasset"]);
    let content = scanned(&[
        ("Mods/Up/Up.pak", DATA),
        ("Mods/Up/Up.utoc", &upper),
        ("Mods/Low/Low.pak", DATA),
        ("Mods/Low/Low.utoc", &lower),
    ]);
    assert_eq!(linked(&content, "Standard:Up"), link_to("Standard:Low", 1));
    assert_eq!(
        row(&content, "Standard:Low").content_conflicts[0].sample,
        ["Polaris/Content/Common_umin/materials/M_Skin.uasset"]
    );
    assert_eq!(
        row(&content, "Standard:Up").content_conflicts[0].sample,
        ["Polaris/Content/Common_umin/Materials/M_Skin.uasset"]
    );
}

#[test]
fn uasset_and_ubulk_count_once() {
    let both = toc(MOUNT, &["X/Tex.uasset", "X/Tex.ubulk"]);
    let one = toc(MOUNT, &["X/Tex.ubulk"]);
    let content = scanned(&[
        ("Mods/Both/Both.pak", DATA),
        ("Mods/Both/Both.utoc", &both),
        ("Mods/One/One.pak", DATA),
        ("Mods/One/One.utoc", &one),
    ]);
    let both_row = row(&content, "Standard:Both");
    assert_eq!(both_row.content.assets, 1);
    assert_eq!(both_row.content.containers[0].files, 2);
    assert_eq!(
        linked(&content, "Standard:Both"),
        link_to("Standard:One", 1)
    );
    assert_eq!(
        both_row.content_conflicts[0].sample,
        ["Polaris/Content/X/Tex.uasset"]
    );
}

#[test]
fn same_row_overlap_is_internal_not_conflict() {
    let base = toc(MOUNT, &["Smoke/Base.uasset", "Smoke/Base.ubulk"]);
    let color = toc(MOUNT, &["Smoke/Base.uasset", "Smoke/Color.uasset"]);
    let content = scanned(&[
        ("Mods/A/A.pak", DATA),
        ("Mods/A/A.utoc", &base),
        ("Mods/A/Color/A_Color.pak", DATA),
        ("Mods/A/Color/A_Color.utoc", &color),
        ("Mods/A/Off/A_Off.utoc-x", &color),
    ]);
    let a = row(&content, "Standard:A");
    assert_eq!(a.content.internal_overlaps, 1);
    assert_eq!(a.content.assets, 2);
    assert!(a.content_conflicts.is_empty());
}

#[test]
fn partial_row_live_container_counts() {
    let shared = toc(MOUNT, &["Smoke/Shared.uasset"]);
    let other = toc(MOUNT, &["Smoke/Other.uasset"]);
    let content = scanned(&[
        ("Mods/P/P.pak", DATA),
        ("Mods/P/P.utoc", &shared),
        ("Mods/P/P_alt.pak-x", DATA),
        ("Mods/P/P_alt.utoc-x", &other),
        ("Mods/Q/Q.pak", DATA),
        ("Mods/Q/Q.utoc", &shared),
        ("Mods/R/R.pak", DATA),
        ("Mods/R/R.utoc", &other),
    ]);
    assert_eq!(row(&content, "Standard:P").state, ModState::Partial);
    assert_eq!(linked(&content, "Standard:P"), link_to("Standard:Q", 1));
    assert!(row(&content, "Standard:R").content_conflicts.is_empty());
}

#[test]
fn logic_prefixed_live_container_counts() {
    let shared = toc(MOUNT, &["Smoke/Shared.uasset"]);
    let content = scanned(&[
        ("LogicMods/.disabled_L/L.pak", DATA),
        ("LogicMods/.disabled_L/L.utoc", &shared),
        ("Mods/Q/Q.pak", DATA),
        ("Mods/Q/Q.utoc", &shared),
    ]);
    let logic = row(&content, "Logic:L");
    assert_eq!(logic.state, ModState::Disabled);
    assert!(logic.issues.contains(&ModIssue::LogicDisabledButLoaded));
    assert_eq!(linked(&content, "Logic:L"), link_to("Standard:Q", 1));
    assert_eq!(linked(&content, "Standard:Q"), link_to("Logic:L", 1));
}

#[test]
fn loose_row_contents() {
    let loose = toc(
        "../../../Polaris/Content/Smoke/",
        &["Shared.uasset", "Shared.ubulk"],
    );
    let content = scanned(&[
        ("Mods/Loose.pak", DATA),
        ("Mods/Loose.ucas", DATA),
        ("Mods/Loose.utoc", &loose),
    ]);
    let summary = &row(&content, "Loose:Mods/Loose").content;
    assert_eq!(summary.containers.len(), 1);
    assert_eq!(summary.containers[0].rel_path, "Loose.utoc");
    assert_eq!(summary.containers[0].files, 2);
    assert_eq!(summary.assets, 1);
}

#[test]
fn garbage_utoc_is_not_iostore_and_scan_succeeds() {
    let content = scanned(&[("Mods/G/G.pak", DATA), ("Mods/G/G.utoc", DATA)]);
    let summary = &row(&content, "Standard:G").content;
    assert_eq!(summary.containers[0].status, ContainerStatus::NotIoStore);
    assert_eq!(summary.containers[0].version, None);
    assert_eq!((summary.assets, summary.containers[0].files), (0, 0));
}

#[test]
fn truncated_utoc_is_not_iostore() {
    let bytes = toc(MOUNT, &["Smoke/A.uasset"]);
    let content = scanned(&[("Mods/T/T.pak", DATA), ("Mods/T/T.utoc", &bytes[..100])]);
    let status = row(&content, "Standard:T").content.containers[0].status;
    assert_eq!(status, ContainerStatus::NotIoStore);
}

#[test]
fn encrypted_utoc_is_unsupported() {
    let encrypted = TocBuilder::new(MOUNT)
        .file("Smoke/A.uasset")
        .flags(0x0B)
        .build();
    let mut future = toc(MOUNT, &["Smoke/A.uasset"]);
    future[16] = 9;
    let content = scanned(&[
        ("Mods/E/E.pak", DATA),
        ("Mods/E/E.utoc", &encrypted),
        ("Mods/E/F.utoc", &future),
    ]);
    let containers = &row(&content, "Standard:E").content.containers;
    assert_eq!(containers[0].status, ContainerStatus::Encrypted);
    assert_eq!(containers[1].status, ContainerStatus::UnsupportedVersion);
    assert_eq!(containers[1].version, Some(9));
}

#[test]
fn pak_name_conflicts_unchanged() {
    let one = toc(MOUNT, &["Smoke/One.uasset"]);
    let two = toc(MOUNT, &["Smoke/Two.uasset"]);
    let content = scanned(&[
        ("Mods/Delta/Shared.pak", DATA),
        ("Mods/Delta/Shared.utoc", &one),
        ("Mods/Epsilon/Sub/shared.PAK", DATA),
        ("Mods/Epsilon/Sub/shared.utoc", &two),
        ("Mods/Zeta/Zeta.pak", DATA),
        ("Mods/Zeta/Zeta.utoc", &one),
    ]);
    let delta = row(&content, "Standard:Delta");
    assert_eq!(delta.conflicts_with.len(), 1);
    assert_eq!(delta.conflicts_with[0].key, "Standard:Epsilon");
    assert_eq!(
        linked(&content, "Standard:Delta"),
        link_to("Standard:Zeta", 1)
    );
    assert!(
        row(&content, "Standard:Epsilon")
            .content_conflicts
            .is_empty()
    );
    assert!(row(&content, "Standard:Zeta").conflicts_with.is_empty());
}

fn hashed(files: &[(&str, u8)]) -> Vec<u8> {
    files
        .iter()
        .fold(TocBuilder::new(MOUNT), |builder, &(path, byte)| {
            builder.file_with_hash(path, [byte; 20])
        })
        .build()
}

#[test]
fn identical_copies_never_conflict() {
    let a = hashed(&[
        ("Cmn/Common.uasset", 1),
        ("Cmn/Common.ubulk", 2),
        ("A/Own.uasset", 3),
    ]);
    let b = hashed(&[
        ("Cmn/Common.uasset", 1),
        ("Cmn/Common.ubulk", 2),
        ("B/Own.uasset", 4),
    ]);
    let content = scanned(&[
        ("Mods/Aye/Aye.pak", DATA),
        ("Mods/Aye/Aye.utoc", &a),
        ("Mods/Bee/Bee.pak", DATA),
        ("Mods/Bee/Bee.utoc", &b),
    ]);
    for key in ["Standard:Aye", "Standard:Bee"] {
        assert!(linked(&content, key).is_empty(), "{key}");
        assert_eq!(row(&content, key).content.shared_identical, 1, "{key}");
    }
}

#[test]
fn different_versions_conflict_and_identical_ones_are_counted() {
    let a = hashed(&[("Cmn/Common.uasset", 1), ("Ker/Upper.uasset", 5)]);
    let b = hashed(&[("Cmn/Common.uasset", 1), ("Ker/Upper.uasset", 6)]);
    let c = hashed(&[("Ker/Upper.uasset", 5)]);
    let content = scanned(&[
        ("Mods/Aye/Aye.pak", DATA),
        ("Mods/Aye/Aye.utoc", &a),
        ("Mods/Bee/Bee.pak", DATA),
        ("Mods/Bee/Bee.utoc", &b),
        ("Mods/Cee/Cee.pak", DATA),
        ("Mods/Cee/Cee.utoc", &c),
    ]);
    assert_eq!(linked(&content, "Standard:Aye"), link_to("Standard:Bee", 1));
    assert_eq!(
        linked(&content, "Standard:Bee"),
        [
            ("Standard:Aye".to_owned(), 1),
            ("Standard:Cee".to_owned(), 1)
        ]
    );
    assert_eq!(linked(&content, "Standard:Cee"), link_to("Standard:Bee", 1));
    assert_eq!(
        row(&content, "Standard:Aye").content_conflicts[0].sample,
        ["Polaris/Content/Ker/Upper.uasset"]
    );
    assert_eq!(row(&content, "Standard:Aye").content.shared_identical, 1);
    assert_eq!(row(&content, "Standard:Cee").content.shared_identical, 0);
}

#[test]
fn a_differing_bulk_file_makes_the_asset_differ() {
    let a = hashed(&[("Ker/Upper.uasset", 1), ("Ker/Upper.ubulk", 2)]);
    let b = hashed(&[("Ker/Upper.uasset", 1), ("Ker/Upper.ubulk", 3)]);
    let content = scanned(&[
        ("Mods/Aye/Aye.pak", DATA),
        ("Mods/Aye/Aye.utoc", &a),
        ("Mods/Bee/Bee.pak", DATA),
        ("Mods/Bee/Bee.utoc", &b),
    ]);
    assert_eq!(linked(&content, "Standard:Aye"), link_to("Standard:Bee", 1));
}

#[test]
fn unknown_hashes_count_as_different() {
    let a = hashed(&[("Ker/Upper.uasset", 1)]);
    let b = toc(MOUNT, &["Ker/Upper.uasset"]);
    let content = scanned(&[
        ("Mods/Aye/Aye.pak", DATA),
        ("Mods/Aye/Aye.utoc", &a),
        ("Mods/Bee/Bee.pak", DATA),
        ("Mods/Bee/Bee.utoc", &b),
    ]);
    assert_eq!(linked(&content, "Standard:Aye"), link_to("Standard:Bee", 1));
}

#[test]
fn differing_mod_files_absent_from_the_game_are_counted_not_linked() {
    let a = hashed(&[("Lib/Common.uasset", 1), ("Ker/Upper.uasset", 5)]);
    let b = hashed(&[("Lib/Common.uasset", 2), ("Ker/Upper.uasset", 5)]);
    let content = scanned(&[
        ("Mods/Aye/Aye.pak", DATA),
        ("Mods/Aye/Aye.utoc", &a),
        ("Mods/Bee/Bee.pak", DATA),
        ("Mods/Bee/Bee.utoc", &b),
    ]);
    for key in ["Standard:Aye", "Standard:Bee"] {
        assert!(linked(&content, key).is_empty(), "{key}");
        let summary = &row(&content, key).content;
        assert_eq!(summary.shared_other_versions, 1, "{key}");
        assert_eq!(summary.shared_identical, 1, "{key}");
    }
}

#[test]
fn without_a_usable_base_nothing_conflicts() {
    let a = hashed(&[("Ker/Upper.uasset", 5)]);
    let b = hashed(&[("Ker/Upper.uasset", 6)]);
    let mods: &[(&str, &[u8])] = &[
        ("Mods/Aye/Aye.pak", DATA),
        ("Mods/Aye/Aye.utoc", &a),
        ("Mods/Bee/Bee.pak", DATA),
        ("Mods/Bee/Bee.utoc", &b),
    ];
    let unavailable = scanned_with_base(DATA, mods);
    let (_dir, paks) = fixture();
    paks_tree(&paks, &[(BASE_TOC, &base_toc(GAME_FILES))]);
    paks_tree(&paks, mods);
    let not_built = mods_support::scan(&paks).content;
    for content in [unavailable, not_built] {
        for key in ["Standard:Aye", "Standard:Bee"] {
            assert!(linked(&content, key).is_empty(), "{key}");
            assert_eq!(row(&content, key).content.shared_other_versions, 0, "{key}");
        }
    }
}
