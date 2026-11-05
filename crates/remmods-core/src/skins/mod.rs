//! Skin sets: whole copies of the game's five skin save files, per Steam account, so the game's
//! slot limit stops mattering. Files only ever move as they are: RemMods never looks inside a
//! save, and it switches only with the game closed. `global1.sav` and the rest stay out of it.

mod api;
mod error;
mod locate;
mod model;
mod store;

pub use error::SkinsError;
pub use model::{
    LinkedSkins, SKIN_BACKUPS_KEPT, SKIN_SET_NAME_MAX_CHARS, SKIN_SET_NOTES_MAX_CHARS,
    SKIN_SETS_VERSION, SkinCopy, SkinSet, SkinSetView, SkinSetsFile, SkinsView,
};
pub(crate) use store::all_screenshots;
