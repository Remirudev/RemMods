use remmods_core::assets as core;
use serde::{Deserialize, Serialize};

use crate::content::{AssetOrigin, CharacterHint, SlotHint};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum PictureProblem {
    NotAnImage,
    TooLarge,
    Unreadable,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ItemNameSource {
    ItemList,
    Matched,
    ModText,
    Internal,
}

/// One item a mod touches. Pictures are ids for `assets_picture`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModItem {
    /// The internal name.
    pub key: String,
    pub character: CharacterHint,
    pub slot: SlotHint,
    pub name: Option<String>,
    pub name_source: ItemNameSource,
    pub game_picture: Option<String>,
    pub mod_picture: Option<String>,
    pub live: bool,
    pub origin: AssetOrigin,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModItems {
    pub mod_ref: String,
    /// The picture the owner gave this mod, an id for `assets_picture`.
    pub own_picture: Option<String>,
    pub items: Vec<ModItem>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AssetsStatus {
    pub pictures: u32,
    #[specta(type = specta_typescript::Number)]
    pub bytes: f64,
    pub names: u32,
    pub failed: u32,
}

impl From<core::PictureProblem> for PictureProblem {
    fn from(problem: core::PictureProblem) -> Self {
        match problem {
            core::PictureProblem::NotAnImage => Self::NotAnImage,
            core::PictureProblem::TooLarge => Self::TooLarge,
            core::PictureProblem::Unreadable => Self::Unreadable,
        }
    }
}

impl From<core::ItemNameSource> for ItemNameSource {
    fn from(source: core::ItemNameSource) -> Self {
        match source {
            core::ItemNameSource::ItemList => Self::ItemList,
            core::ItemNameSource::Matched => Self::Matched,
            core::ItemNameSource::ModText => Self::ModText,
            core::ItemNameSource::Internal => Self::Internal,
        }
    }
}

impl From<&core::ModItem> for ModItem {
    fn from(item: &core::ModItem) -> Self {
        Self {
            key: item.key.clone(),
            character: CharacterHint::from_code(&item.character),
            slot: SlotHint::from_code(&item.slot),
            name: item.name.clone(),
            name_source: item.name_source.into(),
            game_picture: item.game_picture.clone(),
            mod_picture: item.mod_picture.clone(),
            live: item.live,
            origin: item.origin.into(),
        }
    }
}

impl From<&core::ModItems> for ModItems {
    fn from(items: &core::ModItems) -> Self {
        Self {
            mod_ref: items.mod_ref.as_str().to_owned(),
            own_picture: items.own_picture.clone(),
            items: items.items.iter().map(ModItem::from).collect(),
        }
    }
}

impl From<core::AssetsStatus> for AssetsStatus {
    // Byte counts stay far below 2^53.
    #[allow(clippy::cast_precision_loss)]
    fn from(status: core::AssetsStatus) -> Self {
        Self {
            pictures: status.pictures,
            bytes: status.bytes as f64,
            names: status.names,
            failed: status.failed,
        }
    }
}
