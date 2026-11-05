use remmods_pak::hints::{
    AssetHint, ItemRef, Slot, character_name, hint_for, is_common, picture_item, prefab_item,
    prefab_key, slot,
};

fn hint(character: Option<&str>, slot: Option<&str>) -> AssetHint {
    AssetHint {
        character: character.map(str::to_owned),
        slot: slot.map(str::to_owned),
    }
}

#[test]
fn slot_table_is_exact() {
    let table = [
        ("bdu", Slot::Upper),
        ("bdl", Slot::Lower),
        ("btm", Slot::Bottom),
        ("bdf", Slot::FullBody),
        ("har", Slot::Hair),
        ("hed", Slot::Head),
        ("hef", Slot::FullHead),
        ("hat", Slot::Hat),
        ("fac", Slot::Face),
        ("fap", Slot::FacePaint),
        ("fah", Slot::FaceHair),
        ("eym", Slot::EyeMakeup),
        ("eye", Slot::Eyes),
        ("chk", Slot::Cheek),
        ("lip", Slot::Lips),
        ("gla", Slot::Glasses),
        ("acc", Slot::Accessory),
        ("sho", Slot::Shoes),
        ("exp", Slot::Extra),
        ("ex0", Slot::Extra),
    ];
    for (code, expected) in table {
        assert_eq!(slot(code), Some(expected), "{code}");
    }
    for code in ["ex3", "jaw", "far", "BDU", "", "bdux"] {
        assert_eq!(slot(code), None, "{code}");
    }
}

#[test]
fn character_names_are_only_confirmed() {
    assert_eq!(character_name("ker"), Some("Kunimitsu"));
    assert_eq!(character_name("mnt"), Some("Alisa"));
    assert_eq!(character_name("zbr"), Some("Reina"));
    assert_eq!(character_name("knk"), Some("Armor King"));
    for code in ["cf0", "cm0", "cmn", "KER", "ava", "xyz"] {
        assert_eq!(character_name(code), None, "{code}");
    }
    assert!(is_common("cmn"));
    assert!(!is_common("ker"));
}

#[test]
fn r1_category_folders() {
    assert_eq!(
        hint_for("Polaris/Content/Character/Item/Base_Item/ker/bdf/BI_x.uasset"),
        hint(Some("ker"), Some("bdf"))
    );
    assert_eq!(
        hint_for(
            "Polaris/Content/Character/Item/Base_Character_Item/mnt/btm/SBA_f_no_pantsuAlisa.uasset"
        ),
        hint(Some("mnt"), Some("btm"))
    );
    assert_eq!(
        hint_for("Polaris/Content/Character/Item/Customize_Item/zbr/Mesh.uasset"),
        hint(Some("zbr"), None)
    );
    assert_eq!(
        hint_for("Polaris/Content/Character/Item/Item_Prefab/aml/bdu.uasset"),
        hint(Some("aml"), None)
    );
}

#[test]
fn r1_mod_folder_names_do_not_match() {
    assert_eq!(
        hint_for("Polaris/Content/Character/Item/Base_Item/SPY/Mesh/SK_Spy.uasset"),
        AssetHint::default()
    );
    assert_eq!(
        hint_for("Polaris/Content/Character/Item/Base_Item/2026Dancer/T_Body.uasset"),
        AssetHint::default()
    );
    assert_eq!(
        hint_for("Polaris/Content/Character/Item/Other_Item/ker/bdf/T.uasset"),
        AssetHint::default()
    );
}

#[test]
fn r2_rep_texture() {
    assert_eq!(
        hint_for("Polaris/Content/UI/Rep_Texture/CUS_CH_Item/kal/T_Icon.uasset"),
        hint(Some("kal"), None)
    );
}

#[test]
fn r3_file_stem_prefixes() {
    assert_eq!(
        hint_for("Polaris/Content/Mod/BI_ker_bdf_sb_lily.uasset"),
        hint(Some("ker"), Some("bdf"))
    );
    assert_eq!(
        hint_for("x/BCI_cf0_btm_a.uexp"),
        hint(Some("cf0"), Some("btm"))
    );
    assert_eq!(
        hint_for("x/T_UI_CUS_CH_item_kgr_har_01.uasset"),
        hint(Some("kgr"), Some("har"))
    );
    assert_eq!(hint_for("x/BI_ker_bdf.uasset"), AssetHint::default());
    assert_eq!(hint_for("x/XX_ker_bdf_a.uasset"), AssetHint::default());
}

#[test]
fn r4_model_unique_and_common() {
    assert_eq!(
        hint_for("Polaris/Content/Character/Item/model/unique/upper/wkz_bdu_01/SK_A.uasset"),
        hint(Some("wkz"), Some("bdu"))
    );
    assert_eq!(
        hint_for("Polaris/Content/Character/Item/model/common/female/hair/cmn_har_03/SK_A.uasset"),
        hint(Some("cmn"), Some("har"))
    );
    assert_eq!(
        hint_for("Polaris/Content/Character/Item/model/common/female/hair/abc_har_03/SK_A.uasset"),
        AssetHint::default()
    );
}

#[test]
fn r5_name_tokens() {
    assert_eq!(
        hint_for(
            "Polaris/Content/Character/Item/Base_Item/Samurai/Mesh/BI_SamuraiHair_Kunimitsu.uasset"
        ),
        hint(Some("ker"), None)
    );
    assert_eq!(
        hint_for("Polaris/Content/2025CND/BI/BI_CND_Alisa.uasset"),
        hint(Some("mnt"), None)
    );
    assert_eq!(hint_for("x/alisa-v2/Mesh.uasset"), hint(Some("mnt"), None));
    assert_eq!(hint_for("x/Kunimitsuish/A.uasset"), AssetHint::default());
}

#[test]
fn r5_leaves_names_that_are_ordinary_words_alone() {
    for path in [
        "Polaris/Content/Mods/Bob_Hair/Mesh.uasset",
        "x/King/Crown.uasset",
        "x/Jun-v2/Top.uasset",
    ] {
        assert_eq!(hint_for(path), AssetHint::default(), "{path}");
    }
}

#[test]
fn unknown_paths_have_no_hint() {
    for path in [
        "",
        "A.uasset",
        "Polaris/Content/Smoke/Shared.uasset",
        "Polaris/Content/Character/Item/Base_Item/ker",
        "Engine/Content/abc/def/g.uasset",
    ] {
        assert_eq!(hint_for(path), AssetHint::default(), "{path}");
    }
}

fn item(character: &str, slot: &str, name: &str) -> ItemRef {
    ItemRef {
        character: character.to_owned(),
        slot: slot.to_owned(),
        name: name.to_owned(),
    }
}

#[test]
fn prefabs_name_their_item() {
    let path = "Polaris/Content/Character/Item/Item_Prefab/rat/bdu/IP_rat_bdu_F_Blouse_v1.uasset";
    assert_eq!(prefab_item(path), Some(item("rat", "bdu", "f_blouse_v1")));
    assert_eq!(
        item("rat", "bdu", "f_blouse_v1").key(),
        "rat_bdu_f_blouse_v1"
    );
}

#[test]
fn only_prefab_assets_in_the_prefab_folder_count() {
    for path in [
        "Polaris/Content/Character/Item/Base_Item/rat/bdu/IP_rat_bdu_blouse.uasset",
        "Polaris/Content/Character/Item/Item_Prefab/rat/bdu/IP_rat_bdu_blouse.uexp",
        "Polaris/Content/Character/Item/Item_Prefab/rat/bdu/BI_rat_bdu_blouse.uasset",
        "Polaris/Content/Character/Item/Item_Prefab/rat/bdu/IP_rat_blouse.uasset",
        "Polaris/Content/Character/Item/Item_Prefab/rat/bdu/IP_rat_bdu_.uasset",
        "IP_rat_bdu_blouse.uasset",
    ] {
        assert_eq!(prefab_item(path), None, "{path}");
    }
}

#[test]
fn menu_pictures_name_their_item_in_any_case() {
    for stem in ["T_UI_CUS_CH_item", "T_UI_CUS_CH_Item"] {
        let path = format!(
            "Polaris/Content/UI/Rep_Texture/CUS_CH_Item/cf0/{stem}_cf0_sho_barefoot.uasset"
        );
        assert_eq!(picture_item(&path), Some(item("cf0", "sho", "barefoot")));
    }
    assert_eq!(
        picture_item("Polaris/Content/UI/T_UI_Other_rat_bdu_x.uasset"),
        None
    );
    assert_eq!(
        picture_item("Polaris/Content/UI/T_UI_CUS_CH_item_rat_bdu_x.ubulk"),
        None
    );
}

#[test]
fn an_item_is_looked_up_under_itself_then_the_shared_prefixes() {
    let keys: Vec<String> = item("rat", "sho", "f_barefoot").lookup_keys().collect();
    assert_eq!(
        keys,
        [
            "rat_sho_f_barefoot",
            "cmn_sho_f_barefoot",
            "cf0_sho_f_barefoot",
            "cm0_sho_f_barefoot",
        ]
    );
}

#[test]
fn the_item_list_spells_prefabs_like_the_file_names() {
    assert_eq!(
        prefab_key("IP_rat_bdu_F_Blouse").as_deref(),
        Some("rat_bdu_f_blouse")
    );
    assert_eq!(prefab_key("BI_rat_bdu_blouse"), None);
    assert_eq!(prefab_key("IP_nothing"), None);
}
