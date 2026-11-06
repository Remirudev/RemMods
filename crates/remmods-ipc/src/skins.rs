use remmods_core::skins as core;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SkinSet {
    pub id: String,
    pub name: String,
    pub notes: String,
    pub created: String,
    pub updated: String,
    /// Ids for `assets_picture`.
    pub screenshots: Vec<String>,
    #[specta(type = specta_typescript::Number)]
    pub bytes: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SkinsView {
    /// The Steam account in use.
    pub account: Option<String>,
    /// `false` when the game's skin files were not found.
    pub available: bool,
    pub sets: Vec<SkinSet>,
    pub active: Option<String>,
    /// The set a switch was going to when it stopped.
    pub interrupted: Option<String>,
    pub read_only: bool,
}

/// An automatic copy, offered to get skins back.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SkinCopy {
    pub id: String,
    /// RFC 3339.
    pub taken: String,
    /// The set it was replaced in; `None` for the game's files from before a switch.
    pub set_name: Option<String>,
}

/// What switching a group on did to its linked skin set.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LinkedSkins {
    Switched { name: String },
    GameRunning { name: String },
    Failed { name: String },
    Several,
}

impl From<&core::SkinSetView> for SkinSet {
    // Byte counts stay far below 2^53.
    #[allow(clippy::cast_precision_loss)]
    fn from(view: &core::SkinSetView) -> Self {
        Self {
            id: view.set.id.clone(),
            name: view.set.name.clone(),
            notes: view.set.notes.clone(),
            created: view.set.created.clone(),
            updated: view.set.updated.clone(),
            screenshots: view.set.screenshots.clone(),
            bytes: view.bytes as f64,
        }
    }
}

impl From<&core::SkinsView> for SkinsView {
    fn from(view: &core::SkinsView) -> Self {
        Self {
            account: view.account.clone(),
            available: view.available,
            sets: view.sets.iter().map(SkinSet::from).collect(),
            active: view.active.clone(),
            interrupted: view.interrupted.clone(),
            read_only: view.read_only,
        }
    }
}

impl From<&core::SkinCopy> for SkinCopy {
    fn from(copy: &core::SkinCopy) -> Self {
        Self {
            id: copy.id.clone(),
            taken: copy.taken.clone(),
            set_name: copy.set_name.clone(),
        }
    }
}

impl From<&core::LinkedSkins> for LinkedSkins {
    fn from(linked: &core::LinkedSkins) -> Self {
        match linked {
            core::LinkedSkins::Switched { name } => Self::Switched { name: name.clone() },
            core::LinkedSkins::GameRunning { name } => Self::GameRunning { name: name.clone() },
            core::LinkedSkins::Failed { name } => Self::Failed { name: name.clone() },
            core::LinkedSkins::Several => Self::Several,
        }
    }
}
