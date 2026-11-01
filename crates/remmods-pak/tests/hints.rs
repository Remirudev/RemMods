use remmods_pak::hints::{AssetHint, Slot, character_name, hint_for, is_common, slot};

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
    ];
    for (code, expected) in table {
        assert_eq!(slot(code), Some(expected), "{code}");
    }
    for code in ["ex0", "ex3", "jaw", "far", "BDU", "", "bdux"] {
        assert_eq!(slot(code), None, "{code}");
    }
}

#[test]
fn character_names_are_only_confirmed() {
    assert_eq!(character_name("ker"), Some("Kunimitsu"));
    assert_eq!(character_name("mnt"), Some("Alisa"));
    for code in ["zbr", "aml", "cf0", "cmn", "KER"] {
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
