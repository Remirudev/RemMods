//! The assets folder: pictures and names come from the game and the mods, read-only, and follow
//! both when they change.
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod mods_support;
mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use mods_support::toggle::ToggleFixture;
use mods_support::{DATA, MOUNT};
use remmods_core::CoreError;
use remmods_core::assets::{ItemNameSource, ModItem, ModItems, PICTURE_EDGE, PictureProblem};
use remmods_core::content::AssetOrigin;
use remmods_core::mods::{ModTarget, ModsError, RenameRequest};
use remmods_pak::fixture::TocBuilder;
use remmods_unpack::fixture::{bgra_package, item_list, text_table};

const BASE_MOUNT: &str = "../../../";
const PICTURE_DIR: &str = "Polaris/Content/UI/Rep_Texture/CUS_CH_Item";
const PREFAB_DIR: &str = "Character/Item/Item_Prefab";

fn hash(seed: u8) -> [u8; 20] {
    [seed; 20]
}

fn picture_path(item: &str) -> String {
    format!("{PICTURE_DIR}/x/T_UI_CUS_CH_item_{item}.uasset")
}

fn prefab_path(item: &str) -> String {
    let mut parts = item.split('_');
    let character = parts.next().expect("character");
    let slot = parts.next().expect("slot");
    format!("{PREFAB_DIR}/{character}/{slot}/IP_{item}.uasset")
}

/// The game's pictures and text tables, as `(file, bytes)` entries for the fixture.
struct Container {
    name: String,
    toc: TocBuilder,
}

impl Container {
    fn game(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            toc: TocBuilder::new(BASE_MOUNT),
        }
    }

    fn picture(mut self, item: &str, seed: u8) -> Self {
        let bytes = bgra_package([seed, seed, seed, 255]);
        self.toc = self
            .toc
            .file_with_bytes(&picture_path(item), hash(seed), &bytes);
        self
    }

    fn broken_picture(mut self, item: &str, seed: u8) -> Self {
        self.toc = self
            .toc
            .file_with_bytes(&picture_path(item), hash(seed), b"no texture");
        self
    }

    fn table(mut self, language: &str, table: &str, records: &[(&str, &str)], seed: u8) -> Self {
        let path = format!("Polaris/Content/Localize/{language}/{table}.uasset");
        self.toc = self
            .toc
            .file_with_bytes(&path, hash(seed), &text_table(records));
        self
    }

    fn prefab(mut self, item: &str, seed: u8) -> Self {
        let path = format!("Polaris/Content/{}", prefab_path(item));
        self.toc = self.toc.file_with_bytes(&path, hash(seed), b"prefab");
        self
    }

    fn entries(&self) -> Vec<(String, Vec<u8>)> {
        let (utoc, ucas) = self.toc.build_with_cas();
        vec![
            (format!("{}.utoc", self.name), utoc),
            (format!("{}.ucas", self.name), ucas),
        ]
    }
}

/// A mod folder `Mods/<name>` with a container and its `.pak`.
fn a_mod(name: &str) -> Container {
    Container {
        name: format!("Mods/{name}/{name}"),
        toc: TocBuilder::new(BASE_MOUNT),
    }
}

fn fixture(containers: &[&Container]) -> ToggleFixture {
    let mut files = Vec::new();
    for container in containers {
        files.extend(container.entries());
        if container.name.starts_with("Mods/") {
            files.push((format!("{}.pak", container.name), DATA.to_vec()));
        }
    }
    let entries: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
        .collect();
    ToggleFixture::new(&entries)
}

fn write_container(fixture: &ToggleFixture, container: &Container) {
    for (name, bytes) in container.entries() {
        let path = name
            .split('/')
            .fold(fixture.paks.clone(), |path, part| path.join(part));
        fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        fs::write(path, bytes).expect("write");
    }
}

fn refresh(fixture: &ToggleFixture) -> bool {
    fixture.core.mods_refresh().expect("mods_refresh");
    fixture.core.mods_base_refresh().expect("base refresh");
    fixture.core.assets_refresh().expect("assets refresh")
}

fn items(fixture: &ToggleFixture, key: &str, language: &str) -> ModItems {
    fixture
        .core
        .mods_items(fixture.row(key).mod_ref.as_str(), language)
        .expect("mods_items")
}

fn item<'a>(items: &'a ModItems, key: &str) -> &'a ModItem {
    items
        .items
        .iter()
        .find(|item| item.key == key)
        .unwrap_or_else(|| panic!("no item {key}"))
}

fn pictures_folder(fixture: &ToggleFixture) -> PathBuf {
    fixture.paths.data_dir.join("assets").join("pictures")
}

fn stored_files(folder: &Path) -> usize {
    fs::read_dir(folder).map_or(0, |entries| entries.count())
}

fn name_tables() -> Container {
    Container::game("pakchunk404-Windows")
        .table(
            "en",
            "GTB_Custom_Item",
            &[
                ("TEXT_UI_000_rat_bdu_f_blouse", "Frilled blouse"),
                ("TEXT_UI_000_cf0_sho_f_barefoot", "Barefoot"),
                ("LISTED_KEY", "Listed name"),
            ],
            40,
        )
        .table(
            "de",
            "GTB_Custom_Item",
            &[
                ("TEXT_UI_000_rat_bdu_f_blouse", "Rüschenbluse"),
                ("LISTED_KEY", "Name aus der Liste"),
            ],
            41,
        )
        .table("fr", "GTB_Custom_Item", &[("TEXT_UI_000_x_y_z", "Non")], 42)
}

fn game_pictures() -> Container {
    Container::game("pakchunk301-Windows")
        .picture("rat_bdu_f_blouse", 10)
        .picture("cf0_sho_f_barefoot", 11)
        .picture("rat_har_unused", 12)
}

fn blouse_mod() -> Container {
    a_mod("Blouse")
        .prefab("rat_bdu_f_blouse", 20)
        .picture("rat_bdu_f_blouse", 21)
        .prefab("rat_sho_f_barefoot", 22)
}

#[test]
fn pictures_and_names_of_touched_items_are_stored() {
    let (game, tables, blouse) = (game_pictures(), name_tables(), blouse_mod());
    let fixture = fixture(&[&game, &tables, &blouse]);

    assert!(refresh(&fixture));

    let found = items(&fixture, "Standard:Blouse", "en");
    assert_eq!(found.items.len(), 2);
    let top = item(&found, "rat_bdu_f_blouse");
    assert_eq!(top.character, "rat");
    assert_eq!(top.slot, "bdu");
    assert_eq!(top.name.as_deref(), Some("Frilled blouse"));
    assert_eq!(top.name_source, ItemNameSource::Matched);
    assert!(top.live);
    let (game_id, mod_id) = (
        top.game_picture.clone().expect("game picture"),
        top.mod_picture.clone().expect("mod picture"),
    );
    assert_ne!(game_id, mod_id);

    let shoes = item(&found, "rat_sho_f_barefoot");
    assert_eq!(shoes.name.as_deref(), Some("Barefoot"));
    assert!(shoes.game_picture.is_some(), "shared picture");
    assert_eq!(shoes.mod_picture, None);

    // Three of the game's pictures exist, the mods touch two; plus the mod's own.
    assert_eq!(stored_files(&pictures_folder(&fixture)), 3);
}

#[test]
fn german_names_are_used_and_english_is_the_fallback() {
    let (game, tables, blouse) = (game_pictures(), name_tables(), blouse_mod());
    let fixture = fixture(&[&game, &tables, &blouse]);
    refresh(&fixture);

    let german = items(&fixture, "Standard:Blouse", "de");
    let top = item(&german, "rat_bdu_f_blouse");
    assert_eq!(top.name.as_deref(), Some("Rüschenbluse"));

    let unknown_language = items(&fixture, "Standard:Blouse", "xx");
    let top = item(&unknown_language, "rat_bdu_f_blouse");
    assert_eq!(top.name.as_deref(), Some("Frilled blouse"));
}

#[test]
fn a_picture_reaches_the_ui_as_a_data_url() {
    let (game, tables, blouse) = (game_pictures(), name_tables(), blouse_mod());
    let fixture = fixture(&[&game, &tables, &blouse]);
    refresh(&fixture);

    let found = items(&fixture, "Standard:Blouse", "en");
    let id = item(&found, "rat_bdu_f_blouse")
        .game_picture
        .clone()
        .expect("picture");
    assert!(id.starts_with(&format!("{PICTURE_EDGE}-")));
    let url = fixture.core.assets_picture(&id).expect("url");
    let encoded = url.strip_prefix("data:image/png;base64,").expect("prefix");
    let bytes = STANDARD.decode(encoded).expect("base64");
    assert_eq!(&bytes[1..4], b"PNG");
}

#[test]
fn only_real_picture_ids_are_read() {
    let (game, tables, blouse) = (game_pictures(), name_tables(), blouse_mod());
    let fixture = fixture(&[&game, &tables, &blouse]);
    refresh(&fixture);
    let names = fixture.paths.data_dir.join("assets").join("names.en.json");
    assert!(names.is_file());

    for id in [
        "",
        "names.en",
        "../names.en",
        "..",
        "192",
        "192-",
        "192-zz",
        "-0123456789abcdef0123456789abcdef",
        "192-0123456789abcdef0123456789abcdeg",
        "192-0123456789ABCDEF0123456789ABCDEF",
    ] {
        assert_eq!(fixture.core.assets_picture(id), None, "{id}");
    }
}

#[test]
fn nothing_is_read_twice() {
    let (game, tables, blouse) = (game_pictures(), name_tables(), blouse_mod());
    let fixture = fixture(&[&game, &tables, &blouse]);
    assert!(refresh(&fixture));
    assert!(!refresh(&fixture));

    let second = fixture.second_core();
    second.mods_list().expect("list");
    second.mods_base_refresh().expect("base");
    assert!(!second.assets_refresh().expect("restart reads nothing new"));
    let found = second
        .mods_items(fixture.row("Standard:Blouse").mod_ref.as_str(), "en")
        .expect("items");
    let top = item(&found, "rat_bdu_f_blouse");
    assert_eq!(top.name.as_deref(), Some("Frilled blouse"));
    assert!(top.game_picture.is_some());
}

#[test]
fn a_game_update_refreshes_names_and_swaps_pictures() {
    let (game, tables, blouse) = (game_pictures(), name_tables(), blouse_mod());
    let fixture = fixture(&[&game, &tables, &blouse]);
    refresh(&fixture);
    let before = items(&fixture, "Standard:Blouse", "en");
    let old_id = item(&before, "rat_bdu_f_blouse").game_picture.clone();

    let updated = Container::game("pakchunk301-Windows")
        .picture("rat_bdu_f_blouse", 30)
        .picture("cf0_sho_f_barefoot", 11);
    write_container(&fixture, &updated);
    let new_tables = Container::game("pakchunk404-Windows").table(
        "en",
        "GTB_Custom_Item",
        &[("TEXT_UI_000_rat_bdu_f_blouse", "Renamed blouse")],
        43,
    );
    write_container(&fixture, &new_tables);

    assert!(refresh(&fixture));

    let after = items(&fixture, "Standard:Blouse", "en");
    let top = item(&after, "rat_bdu_f_blouse");
    assert_eq!(top.name.as_deref(), Some("Renamed blouse"));
    assert!(top.game_picture.is_some());
    assert_ne!(top.game_picture, old_id);
    assert_eq!(
        fixture.core.assets_picture(&old_id.expect("id")),
        None,
        "the old picture is gone"
    );
    assert_eq!(stored_files(&pictures_folder(&fixture)), 3);
}

#[test]
fn a_patch_container_overrides_its_base() {
    let base = Container::game("pakchunk301-Windows").picture("rat_bdu_f_blouse", 10);
    let patch = Container::game("pakchunk301-Windows_0_P").picture("rat_bdu_f_blouse", 31);
    let only_base = Container::game("pakchunk301-Windows").picture("rat_bdu_f_blouse", 31);
    let blouse = a_mod("Blouse").prefab("rat_bdu_f_blouse", 20);

    let patched = fixture(&[&base, &patch, &blouse]);
    refresh(&patched);
    let reference = fixture(&[&only_base, &blouse]);
    refresh(&reference);

    let id = |fixture: &ToggleFixture| {
        let found = items(fixture, "Standard:Blouse", "en");
        item(&found, "rat_bdu_f_blouse").game_picture.clone()
    };
    assert!(id(&patched).is_some());
    assert_eq!(id(&patched), id(&reference));
}

#[test]
fn a_removed_mod_takes_its_pictures_along() {
    let (game, tables, blouse) = (game_pictures(), name_tables(), blouse_mod());
    let fixture = fixture(&[&game, &tables, &blouse]);
    refresh(&fixture);
    assert_eq!(stored_files(&pictures_folder(&fixture)), 3);

    fs::remove_dir_all(fixture.paks.join("Mods").join("Blouse")).expect("remove mod");

    assert!(refresh(&fixture));
    assert_eq!(stored_files(&pictures_folder(&fixture)), 0);
}

#[test]
fn a_switched_off_mod_keeps_its_pictures() {
    let (game, tables, blouse) = (game_pictures(), name_tables(), blouse_mod());
    let fixture = fixture(&[&game, &tables, &blouse]);
    refresh(&fixture);

    fixture.toggle(&["Standard:Blouse"], false).expect("toggle");
    refresh(&fixture);

    let found = items(&fixture, "Standard:Blouse", "en");
    let top = item(&found, "rat_bdu_f_blouse");
    assert!(!top.live);
    assert!(top.mod_picture.is_some());
    assert_eq!(stored_files(&pictures_folder(&fixture)), 3);
}

#[test]
fn a_bad_picture_costs_only_itself_and_is_not_tried_again() {
    let (game, tables) = (game_pictures(), name_tables());
    let bad = a_mod("Bad")
        .prefab("rat_bdu_f_blouse", 20)
        .broken_picture("rat_bdu_f_blouse", 25);
    let fixture = fixture(&[&game, &tables, &bad]);

    assert!(refresh(&fixture));
    assert!(!refresh(&fixture), "the failure is remembered");

    let found = items(&fixture, "Standard:Bad", "en");
    let top = item(&found, "rat_bdu_f_blouse");
    assert_eq!(top.mod_picture, None);
    assert!(top.game_picture.is_some());
    assert_eq!(fixture.core.assets_status("en").failed, 1);
}

#[test]
fn nothing_new_is_read_while_the_game_runs() {
    let (game, tables, blouse) = (game_pictures(), name_tables(), blouse_mod());
    let fixture = fixture(&[&game, &tables, &blouse]);
    fixture.running.store(true, Ordering::SeqCst);

    assert!(!refresh(&fixture));
    assert_eq!(stored_files(&pictures_folder(&fixture)), 0);

    fixture.running.store(false, Ordering::SeqCst);
    assert!(refresh(&fixture));
    assert_eq!(stored_files(&pictures_folder(&fixture)), 2 + 1);
}

fn item_list_path(fixture: &ToggleFixture) -> PathBuf {
    fixture
        .game_root
        .join("Polaris")
        .join("Content")
        .join("Binary")
        .join("Mods")
        .join("fbsdata_extract")
        .join("customize_item_common_list.bin")
}

#[test]
fn the_item_list_beats_name_matching_and_a_new_list_is_noticed() {
    let (game, tables, blouse) = (game_pictures(), name_tables(), blouse_mod());
    let fixture = fixture(&[&game, &tables, &blouse]);
    refresh(&fixture);

    let list = item_list_path(&fixture);
    fs::create_dir_all(list.parent().expect("parent")).expect("folder");
    fs::write(
        &list,
        item_list(&[("IP_rat_bdu_f_blouse", "LISTED_KEY"), ("IP_gone", "NO_KEY")]),
    )
    .expect("list");

    assert!(refresh(&fixture));

    let found = items(&fixture, "Standard:Blouse", "de");
    let top = item(&found, "rat_bdu_f_blouse");
    assert_eq!(top.name.as_deref(), Some("Name aus der Liste"));
    assert_eq!(top.name_source, ItemNameSource::ItemList);
    let shoes = item(&found, "rat_sho_f_barefoot");
    assert_eq!(shoes.name_source, ItemNameSource::Internal);
    assert_eq!(shoes.name, None, "no German text for these");
}

#[test]
fn a_colour_variant_is_named_like_its_item() {
    let tables = name_tables();
    let variant = a_mod("Variant").prefab("rat_bdu_f_blouse_v2", 20);
    let fixture = fixture(&[&tables, &variant]);
    refresh(&fixture);

    let found = items(&fixture, "Standard:Variant", "en");
    let top = item(&found, "rat_bdu_f_blouse_v2");
    assert_eq!(top.name.as_deref(), Some("Frilled blouse"));
    assert_eq!(top.name_source, ItemNameSource::Matched);
}

#[test]
fn items_a_mod_adds_take_the_names_it_ships() {
    let (game, tables) = (game_pictures(), name_tables());
    let added = a_mod("Added")
        .prefab("ker_bdf_sfyor", 20)
        .prefab("aml_bdu_f_custom_islandvacation", 21)
        .prefab("ker_har_nameless", 22)
        .picture("ker_bdf_sfyor", 23)
        .table(
            "en",
            "GTB_UMIN_ker",
            &[
                ("TEXT_UMIN_IP_ker_bdf_sfyor", "Yor Style"),
                ("TEXT_ORB_IP_cf0_bdu_custom_Island_Vacation", "Island Top"),
            ],
            26,
        )
        .table(
            "de",
            "GTB_UMIN_ker",
            &[("TEXT_UMIN_IP_ker_bdf_sfyor", "Yor-Stil")],
            27,
        );
    let fixture = fixture(&[&game, &tables, &added]);
    assert!(refresh(&fixture));

    let german = items(&fixture, "Standard:Added", "de");
    let outfit = item(&german, "ker_bdf_sfyor");
    assert_eq!(outfit.name.as_deref(), Some("Yor-Stil"));
    assert_eq!(outfit.name_source, ItemNameSource::ModText);
    assert_eq!(outfit.game_picture, None);
    assert!(outfit.mod_picture.is_some());

    let top = item(&german, "aml_bdu_f_custom_islandvacation");
    assert_eq!(
        top.name.as_deref(),
        Some("Island Top"),
        "English table, shared key"
    );

    let plain = item(&german, "ker_har_nameless");
    assert_eq!(plain.name, None);
    assert_eq!(plain.name_source, ItemNameSource::Internal);
}

#[test]
fn a_mod_with_only_pictures_lists_them_as_items() {
    let game = game_pictures();
    let hidden = a_mod("Hidden").picture("cf0_sho_f_hiddenshoes", 24);
    let fixture = fixture(&[&game, &hidden]);
    refresh(&fixture);

    let found = items(&fixture, "Standard:Hidden", "en");
    assert_eq!(found.items.len(), 1);
    assert_eq!(found.items[0].key, "cf0_sho_f_hiddenshoes");
    assert!(found.items[0].mod_picture.is_some());
}

#[test]
fn a_mod_picture_under_a_shared_name_belongs_to_the_prefab() {
    let game = game_pictures();
    let shoes = a_mod("Shoes")
        .prefab("rat_sho_f_barefoot", 20)
        .picture("cf0_sho_f_barefoot", 24);
    let fixture = fixture(&[&game, &shoes]);
    refresh(&fixture);

    let found = items(&fixture, "Standard:Shoes", "en");
    assert_eq!(found.items.len(), 1);
    assert!(found.items[0].mod_picture.is_some());
}

#[test]
fn without_a_base_index_nothing_is_stored_and_nothing_fails() {
    let (game, tables, blouse) = (game_pictures(), name_tables(), blouse_mod());
    let fixture = fixture(&[&game, &tables, &blouse]);
    fixture.core.mods_refresh().expect("scan");

    assert!(!fixture.core.assets_refresh().expect("refresh"));

    let found = items(&fixture, "Standard:Blouse", "en");
    assert_eq!(found.items.len(), 2);
    assert!(found.items.iter().all(|item| item.name.is_none()));
    assert!(found.items.iter().all(|item| item.game_picture.is_none()));
    let status = fixture.core.assets_status("en");
    assert_eq!((status.pictures, status.names), (0, 0));
}

#[test]
fn the_status_counts_what_is_stored() {
    let (game, tables, blouse) = (game_pictures(), name_tables(), blouse_mod());
    let fixture = fixture(&[&game, &tables, &blouse]);
    refresh(&fixture);

    let status = fixture.core.assets_status("en");
    assert_eq!(status.pictures, 3);
    assert!(status.bytes > 0);
    assert_eq!(status.names, 2);
    assert_eq!(status.failed, 0);
}

#[test]
fn a_damaged_names_file_is_built_again() {
    let (game, tables, blouse) = (game_pictures(), name_tables(), blouse_mod());
    let fixture = fixture(&[&game, &tables, &blouse]);
    refresh(&fixture);
    let names = fixture.paths.data_dir.join("assets").join("names.en.json");
    fs::write(&names, b"{broken").expect("damage");

    let second = fixture.second_core();
    second.mods_list().expect("list");
    second.mods_base_refresh().expect("base");
    assert!(second.assets_refresh().expect("refresh"));
    let found = second
        .mods_items(fixture.row("Standard:Blouse").mod_ref.as_str(), "en")
        .expect("items");
    let top = item(&found, "rat_bdu_f_blouse");
    assert_eq!(top.name.as_deref(), Some("Frilled blouse"));
}

#[test]
fn mount_point_of_a_mod_is_taken_into_account() {
    let game = game_pictures();
    let toc = TocBuilder::new(MOUNT).file_with_bytes(
        &prefab_path("rat_bdu_f_blouse"),
        hash(20),
        b"prefab",
    );
    let shortened = Container {
        name: "Mods/Short/Short".to_owned(),
        toc,
    };
    let fixture = fixture(&[&game, &shortened]);
    refresh(&fixture);

    let found = items(&fixture, "Standard:Short", "en");
    assert_eq!(found.items.len(), 1);
    assert!(found.items[0].game_picture.is_some());
}

#[test]
fn items_say_whether_the_game_has_them() {
    let game = Container::game("pakchunk301-Windows")
        .picture("rat_bdu_f_blouse", 10)
        .prefab("rat_bdu_f_blouse", 13);
    let tables = name_tables();
    let both = a_mod("Both")
        .prefab("rat_bdu_f_blouse", 20)
        .prefab("rat_bdf_brandnew", 21);
    let fixture = fixture(&[&game, &tables, &both]);
    refresh(&fixture);

    let found = items(&fixture, "Standard:Both", "en");
    assert_eq!(
        item(&found, "rat_bdu_f_blouse").origin,
        AssetOrigin::Replaces
    );
    assert_eq!(item(&found, "rat_bdf_brandnew").origin, AssetOrigin::Adds);
}

const PNG_FILE: &[u8] = b"\x89PNG\r\n\x1a\nrest of a picture";
const JPEG_FILE: &[u8] = b"\xff\xd8\xff\xe0rest of a picture";
const WEBP_FILE: &[u8] = b"RIFF\x10\0\0\0WEBPVP8 rest";

struct OwnPicture {
    fixture: ToggleFixture,
    target: ModTarget,
}

fn own_picture_fixture() -> OwnPicture {
    let (game, tables, blouse) = (game_pictures(), name_tables(), blouse_mod());
    let fixture = fixture(&[&game, &tables, &blouse]);
    refresh(&fixture);
    let target = fixture.target("Standard:Blouse");
    OwnPicture { fixture, target }
}

impl OwnPicture {
    fn source(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.fixture.root.join(name);
        fs::write(&path, bytes).expect("source file");
        path
    }

    fn set(&self, name: &str, bytes: &[u8]) -> Result<(), CoreError> {
        let source = self.source(name, bytes);
        self.fixture.core.mods_set_picture(&self.target, &source)
    }

    fn current(&self) -> Option<String> {
        items(&self.fixture, "Standard:Blouse", "en").own_picture
    }

    fn folder(&self) -> PathBuf {
        self.fixture.paths.data_dir.join("assets").join("own")
    }
}

#[test]
fn a_mod_keeps_its_own_picture() {
    let own = own_picture_fixture();
    assert_eq!(own.current(), None);

    own.set("look.png", PNG_FILE).expect("set");

    let id = own.current().expect("own picture");
    let url = own.fixture.core.assets_picture(&id).expect("url");
    let encoded = url.strip_prefix("data:image/png;base64,").expect("mime");
    assert_eq!(STANDARD.decode(encoded).expect("base64"), PNG_FILE);
}

#[test]
fn the_picture_stays_when_its_source_file_goes() {
    let own = own_picture_fixture();
    own.set("look.png", PNG_FILE).expect("set");
    fs::remove_file(own.fixture.root.join("look.png")).expect("remove source");

    let id = own.current().expect("own picture");
    assert!(own.fixture.core.assets_picture(&id).is_some());
}

#[test]
fn each_format_reaches_the_ui_with_its_own_type() {
    let own = own_picture_fixture();
    for (bytes, mime) in [
        (JPEG_FILE, "data:image/jpeg;base64,"),
        (WEBP_FILE, "data:image/webp;base64,"),
    ] {
        own.set("look.bin", bytes).expect("set");
        let id = own.current().expect("own picture");
        let url = own.fixture.core.assets_picture(&id).expect("url");
        assert!(url.starts_with(mime), "{id}");
    }
}

#[test]
fn only_pictures_of_a_sane_size_are_taken() {
    let own = own_picture_fixture();
    let refused = |result: Result<(), CoreError>, wanted: PictureProblem| match result {
        Err(CoreError::Mods(ModsError::Picture { problem })) => assert_eq!(problem, wanted),
        other => panic!("{other:?}"),
    };

    refused(
        own.set("fake.png", b"just text"),
        PictureProblem::NotAnImage,
    );
    refused(own.set("empty.png", b""), PictureProblem::NotAnImage);
    let mut large = PNG_FILE.to_vec();
    large.resize(16 * 1024 * 1024 + 1, 0);
    refused(own.set("large.png", &large), PictureProblem::TooLarge);
    let gone = own.fixture.root.join("gone.png");
    refused(
        own.fixture.core.mods_set_picture(&own.target, &gone),
        PictureProblem::Unreadable,
    );
    assert_eq!(own.current(), None);
}

#[test]
fn a_picture_of_exactly_the_limit_is_taken() {
    let own = own_picture_fixture();
    let mut largest = PNG_FILE.to_vec();
    largest.resize(16 * 1024 * 1024, 0);
    own.set("largest.png", &largest).expect("set");
    assert!(own.current().is_some());
}

#[test]
fn a_new_picture_replaces_the_old_one_on_the_next_refresh() {
    let own = own_picture_fixture();
    own.set("first.png", PNG_FILE).expect("first");
    let first = own.current().expect("first id");
    own.set("second.png", JPEG_FILE).expect("second");
    let second = own.current().expect("second id");
    assert_ne!(first, second);

    refresh(&own.fixture);

    assert_eq!(own.fixture.core.assets_picture(&first), None);
    assert!(own.fixture.core.assets_picture(&second).is_some());
    assert_eq!(stored_files(&own.folder()), 1);
}

#[test]
fn removing_the_picture_frees_its_file() {
    let own = own_picture_fixture();
    own.set("look.png", PNG_FILE).expect("set");
    assert_eq!(stored_files(&own.folder()), 1);

    own.fixture
        .core
        .mods_clear_picture(&own.target)
        .expect("clear");
    assert_eq!(own.current(), None);

    refresh(&own.fixture);
    assert_eq!(stored_files(&own.folder()), 0);
}

#[test]
fn the_picture_follows_a_renamed_mod() {
    let own = own_picture_fixture();
    own.set("look.png", PNG_FILE).expect("set");
    let id = own.current();

    let request = RenameRequest {
        target: own.target.clone(),
        new_name: "Top".to_owned(),
        apply_while_running: false,
    };
    own.fixture.core.mods_rename(&request).expect("rename");

    assert_eq!(items(&own.fixture, "Standard:Top", "en").own_picture, id);
}

#[test]
fn a_stale_target_stores_nothing() {
    let own = own_picture_fixture();
    let stale = ModTarget {
        mod_ref: own.target.mod_ref.clone(),
        key: "Standard:Other".to_owned(),
    };
    let source = own.source("look.png", PNG_FILE);

    assert!(own.fixture.core.mods_set_picture(&stale, &source).is_err());
    assert_eq!(stored_files(&own.folder()), 0);
}

#[test]
fn own_picture_ids_stay_inside_their_folder() {
    let own = own_picture_fixture();
    own.set("look.png", PNG_FILE).expect("set");
    for id in [
        "own-../../names.en.json",
        "own-0123456789abcdef0123456789abcdef.exe",
        "own-0123456789abcdef0123456789abcdef",
        "own-0123456789abcdef0123456789abcde.png",
        "own-0123456789ABCDEF0123456789ABCDEF.png",
        "own-0123456789abcdef0123456789abcdef.png.png",
    ] {
        assert_eq!(own.fixture.core.assets_picture(id), None, "{id}");
    }
}
