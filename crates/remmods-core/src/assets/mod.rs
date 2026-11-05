//! Pictures and names of game items, kept on this PC in `<data_dir>/assets`. Everything comes from
//! the user's own game and mods, read-only, in the background; no game file ships with RemMods.
//! A picture is stored under an id made of its content hashes, so a game update or a changed mod
//! simply asks for new ids and the old files go away.

mod api;
mod game_files;
mod items;
mod mod_names;
mod model;
mod names;
mod own_picture;
mod refresh;
mod store;

pub(crate) use game_files::GameFiles;
pub use model::{
    AssetsStatus, ItemNameSource, ModItem, ModItems, NAME_LANGUAGES, PICTURE_EDGE, PictureProblem,
};
pub(crate) use own_picture::read_own_picture;
pub use store::AssetStore;
