use serde::{Deserialize, Serialize};

pub const SKIN_SETS_VERSION: u32 = 1;
pub const SKIN_SET_NAME_MAX_CHARS: usize = 100;
pub const SKIN_SET_NOTES_MAX_CHARS: usize = 4000;
/// Automatic copies kept of the game's files before a switch, and of each set's replaced copies.
pub const SKIN_BACKUPS_KEPT: usize = 3;

/// One saved set. Its files sit in a folder named after `id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinSet {
    /// 32 lowercase hex.
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub notes: String,
    /// RFC 3339 UTC.
    pub created: String,
    pub updated: String,
    /// Picture ids in the assets folder, in the order they were added.
    #[serde(default)]
    pub screenshots: Vec<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// `sets.json` of one Steam account.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinSetsFile {
    pub version: u32,
    /// The set the game's files belong to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<String>,
    #[serde(default)]
    pub sets: Vec<SkinSet>,
    /// Set while the game's files are being replaced: if it stays, they may be a mix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub switching: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Default for SkinSetsFile {
    fn default() -> Self {
        Self {
            version: SKIN_SETS_VERSION,
            active: None,
            sets: Vec::new(),
            switching: None,
            extra: serde_json::Map::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkinSetView {
    pub set: SkinSet,
    /// What the stored copy takes on disk.
    pub bytes: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SkinsView {
    /// The Steam account the game's newest skin files belong to.
    pub account: Option<String>,
    /// `false` when the game's five skin files were not found.
    pub available: bool,
    pub sets: Vec<SkinSetView>,
    pub active: Option<String>,
    /// The set a switch was going to when it stopped; the game's files may be a mix.
    pub interrupted: Option<String>,
    pub read_only: bool,
}

/// An automatic copy, offered to get skins back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkinCopy {
    pub id: String,
    /// RFC 3339.
    pub taken: String,
    /// The set it was replaced in; none for the game's files from before a switch.
    pub set_name: Option<String>,
}

/// What switching a group on did to its linked skin set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkedSkins {
    Switched { name: String },
    GameRunning { name: String },
    Failed { name: String },
}
