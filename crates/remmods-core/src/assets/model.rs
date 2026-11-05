use crate::mods::ModRef;

/// Longest edge of a stored picture: sharp in the largest place the UI shows one.
pub const PICTURE_EDGE: u32 = 192;
/// The game's text folders that match the app's languages.
pub const NAME_LANGUAGES: [&str; 2] = ["en", "de"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemNameSource {
    /// The game's own item list names this text.
    ItemList,
    /// Found by matching the item's internal name against the text keys.
    Matched,
    /// From a text table the mod ships.
    ModText,
    /// No text found: only the internal name is known.
    Internal,
}

/// One item a mod touches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModItem {
    /// `<character>_<slot>_<name>`, the internal name.
    pub key: String,
    /// Raw code.
    pub character: String,
    /// Raw code.
    pub slot: String,
    pub name: Option<String>,
    pub name_source: ItemNameSource,
    /// Ids for `Core::assets_picture`; `None` until the picture is stored.
    pub game_picture: Option<String>,
    pub mod_picture: Option<String>,
    /// `false` when only switched-off containers carry it.
    pub live: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModItems {
    pub mod_ref: ModRef,
    /// Sorted by (character, slot, name).
    pub items: Vec<ModItem>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AssetsStatus {
    pub pictures: u32,
    pub bytes: u64,
    /// Item names known in the language asked for.
    pub names: u32,
    /// Pictures that could not be read this session.
    pub failed: u32,
}
