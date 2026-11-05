//! Which content conflicts count toward a row's badge: only shared upper-body (`bdu`) and
//! full-body (`bdf`) clothing. Links over shoes, lower-body pieces or assets without a slot hint
//! stay in the row for the details, with `outfit == 0`.

mod mods_support;
mod support;

use mods_support::{BASE_TOC, DATA, MOUNT, base_toc, fixture, paks_tree, row, scan_based};
use remmods_core::mods::ScanContent;
use remmods_pak::fixture::TocBuilder;

const TOP: &str = "Character/Item/Base_Item/ker/bdu/BI_ker_bdu_top.uasset";
const SUIT: &str = "Character/Item/Base_Item/ker/bdf/BI_ker_bdf_suit.uasset";
const PANTS: &str = "Character/Item/Base_Item/ker/bdl/BI_ker_bdl_pants.uasset";
const SHOES: &str = "Character/Item/Base_Item/ker/sho/BI_ker_sho_shoes.uasset";
const MATERIAL: &str = "Common_umin/Materials/M_Skin.uasset";
/// Upper-body clothing found through the model-folder rule instead of the item folders.
const MODEL_TOP: &str = "Character/Item/model/unique/ker/ker_bdu_01/SK_ker_bdu_01.uasset";

/// `files` (below `MOUNT`) as the base game's display paths.
fn game(files: &[&str]) -> Vec<String> {
    files
        .iter()
        .map(|file| format!("Polaris/Content/{file}"))
        .collect()
}

/// A `.utoc` whose files all carry the content hash `[byte; 20]`.
fn hashed(files: &[&str], byte: u8) -> Vec<u8> {
    files
        .iter()
        .fold(TocBuilder::new(MOUNT), |builder, file| {
            builder.file_with_hash(file, [byte; 20])
        })
        .build()
}

/// One enabled mod per `(name, utoc)` and a base game listing `game_files`.
fn scanned(game_files: &[&str], mods: &[(&str, Vec<u8>)]) -> ScanContent {
    let (_dir, paks) = fixture();
    let base_paths = game(game_files);
    let base: Vec<&str> = base_paths.iter().map(String::as_str).collect();
    paks_tree(&paks, &[(BASE_TOC, &base_toc(&base))]);
    for (name, utoc) in mods {
        let pak = format!("Mods/{name}/{name}.pak");
        let toc = format!("Mods/{name}/{name}.utoc");
        paks_tree(&paks, &[(pak.as_str(), DATA), (toc.as_str(), utoc)]);
    }
    scan_based(&paks).content
}

/// (other key, shared, of them upper-body or full-body clothing) per link.
fn links(content: &ScanContent, name: &str) -> Vec<(String, u32, u32)> {
    row(content, &format!("Standard:{name}"))
        .content_conflicts
        .iter()
        .map(|link| (link.key.clone(), link.shared, link.outfit))
        .collect()
}

fn link(other: &str, shared: u32, outfit: u32) -> (String, u32, u32) {
    (format!("Standard:{other}"), shared, outfit)
}

#[test]
fn only_upper_and_full_body_clothing_counts() {
    let all = [TOP, SUIT, PANTS, SHOES, MATERIAL];
    let content = scanned(
        &all,
        &[
            ("Aye", hashed(&[SHOES, PANTS, MATERIAL, TOP], 1)),
            ("Bee", hashed(&[SHOES, PANTS, MATERIAL], 2)),
            ("Cee", hashed(&[TOP], 3)),
            ("Dee", hashed(&[SUIT], 4)),
            ("Eee", hashed(&[SUIT], 5)),
        ],
    );
    assert_eq!(
        links(&content, "Aye"),
        [link("Bee", 3, 0), link("Cee", 1, 1)]
    );
    assert_eq!(links(&content, "Bee"), [link("Aye", 3, 0)]);
    assert_eq!(links(&content, "Cee"), [link("Aye", 1, 1)]);
    assert_eq!(links(&content, "Dee"), [link("Eee", 1, 1)]);
    assert_eq!(links(&content, "Eee"), [link("Dee", 1, 1)]);
}

#[test]
fn model_folder_clothing_counts() {
    let content = scanned(
        &[MODEL_TOP],
        &[
            ("Aye", hashed(&[MODEL_TOP], 1)),
            ("Bee", hashed(&[MODEL_TOP], 2)),
        ],
    );
    assert_eq!(links(&content, "Aye"), [link("Bee", 1, 1)]);
}

#[test]
fn identical_clothing_never_conflicts() {
    let content = scanned(
        &[TOP, SHOES],
        &[
            ("Aye", hashed(&[TOP, SHOES], 1)),
            ("Bee", hashed(&[TOP, SHOES], 1)),
        ],
    );
    for name in ["Aye", "Bee"] {
        assert!(links(&content, name).is_empty(), "{name}");
        let key = format!("Standard:{name}");
        assert_eq!(row(&content, &key).content.shared_identical, 2, "{name}");
    }
}

#[test]
fn clothing_absent_from_the_game_is_only_counted() {
    let content = scanned(
        &[SHOES],
        &[("Aye", hashed(&[TOP], 1)), ("Bee", hashed(&[TOP], 2))],
    );
    for name in ["Aye", "Bee"] {
        assert!(links(&content, name).is_empty(), "{name}");
        let key = format!("Standard:{name}");
        assert_eq!(
            row(&content, &key).content.shared_other_versions,
            1,
            "{name}"
        );
    }
}

#[test]
fn the_sample_lists_clothing_first() {
    let pants: Vec<String> = (1..=6)
        .map(|i| format!("Character/Item/Base_Item/ker/bdl/BI_ker_bdl_p{i}.uasset"))
        .collect();
    let mut files: Vec<&str> = pants.iter().map(String::as_str).collect();
    files.push(TOP);
    let content = scanned(
        &files,
        &[("Aye", hashed(&files, 1)), ("Bee", hashed(&files, 2))],
    );
    let to_bee = &row(&content, "Standard:Aye").content_conflicts[0];
    assert_eq!((to_bee.shared, to_bee.outfit), (7, 1));
    let game_pants = game(&files[..4]);
    let mut expected = game(&[TOP]);
    expected.extend(game_pants);
    assert_eq!(to_bee.sample, expected);
}
