use remmods_pak::{asset_key, display_path, file_key};

#[test]
fn display_path_strips_dotdot_and_slash() {
    assert_eq!(
        display_path("../../../Polaris/Content/Smoke/A.uasset"),
        "Polaris/Content/Smoke/A.uasset"
    );
    assert_eq!(display_path("/Engine/A.uasset"), "Engine/A.uasset");
    assert_eq!(display_path("../..//x/A.uasset"), "x/A.uasset");
    assert_eq!(display_path("Polaris/../A.uasset"), "Polaris/../A.uasset");
    assert_eq!(display_path("A.uasset"), "A.uasset");
}

#[test]
fn file_key_lowercases_ascii_only() {
    assert_eq!(
        file_key("Polaris/Content/ÄB/Cé.UASSET"),
        "polaris/content/Äb/cé.uasset"
    );
}

#[test]
fn asset_key_drops_last_extension() {
    assert_eq!(asset_key("A/B/C.uasset"), "a/b/c");
    assert_eq!(asset_key("A/B/C.tar.uexp"), "a/b/c.tar");
    assert_eq!(asset_key("A/B.dir/C"), "a/b.dir/c");
    assert_eq!(asset_key("A/.hidden"), "a/.hidden");
    assert_eq!(asset_key("Map.umap"), "map");
}

#[test]
fn uasset_and_ubulk_share_asset_key() {
    let a = asset_key("Polaris/Content/Smoke/Shared.uasset");
    assert_eq!(a, asset_key("Polaris/Content/Smoke/Shared.ubulk"));
    assert_eq!(a, asset_key("Polaris/Content/Smoke/Shared.uptnl"));
    assert_ne!(
        file_key("Polaris/Content/Smoke/Shared.uasset"),
        file_key("Polaris/Content/Smoke/Shared.ubulk")
    );
}

#[test]
fn case_variants_share_keys() {
    let lower = "Polaris/Content/Common_umin/materials/M_Skin.uasset";
    let upper = "Polaris/Content/Common_umin/Materials/M_Skin.uasset";
    assert_eq!(file_key(lower), file_key(upper));
    assert_eq!(asset_key(lower), asset_key(upper));
}
