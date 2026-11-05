#![allow(clippy::expect_used)]

use remmods_core::assets as core;
use remmods_core::mods::ModRef;
use remmods_ipc::{AssetsStatus, ItemNameSource, ModItems, Slot};
use serde_json::json;

fn items() -> core::ModItems {
    core::ModItems {
        mod_ref: ModRef::from_token("row"),
        items: vec![core::ModItem {
            key: "ker_bdu_f_blouse".to_owned(),
            character: "ker".to_owned(),
            slot: "bdu".to_owned(),
            name: Some("Bluse".to_owned()),
            name_source: core::ItemNameSource::ModText,
            game_picture: None,
            mod_picture: Some("192-abc".to_owned()),
            live: true,
        }],
    }
}

#[test]
fn items_carry_hints_and_picture_ids() {
    let dto = ModItems::from(&items());
    let item = &dto.items[0];
    assert_eq!(item.name_source, ItemNameSource::ModText);
    assert_eq!(item.slot.slot, Some(Slot::Upper));
    assert_eq!(item.character.name.as_deref(), Some("Kunimitsu"));
    let value = serde_json::to_value(&dto).expect("json");
    assert_eq!(value["items"][0]["nameSource"], json!("modText"));
    assert_eq!(value["items"][0]["gamePicture"], json!(null));
    assert_eq!(value["items"][0]["modPicture"], json!("192-abc"));
}

#[test]
fn the_status_counts_bytes_as_a_number() {
    let dto = AssetsStatus::from(core::AssetsStatus {
        pictures: 3,
        bytes: 5_000_000,
        names: 7,
        failed: 1,
    });
    assert_eq!(
        serde_json::to_value(dto).expect("json"),
        json!({ "pictures": 3, "bytes": 5_000_000.0, "names": 7, "failed": 1 })
    );
}
