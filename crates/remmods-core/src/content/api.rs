use super::listing::mod_contents;
use super::model::ModContents;
use crate::mods::ModsError;
use crate::{Core, CoreError};

impl Core {
    /// The asset list of one row of the current snapshot; reads only `.utoc` indexes.
    pub fn mods_contents(&self, mod_ref: &str) -> Result<ModContents, CoreError> {
        let snapshot = self.mods_list()?;
        let row = snapshot
            .content
            .rows
            .iter()
            .find(|row| row.mod_ref.as_str() == mod_ref)
            .ok_or(ModsError::Stale)?;
        let cache = self.mods.contents();
        let utocs = cache.row_containers(mod_ref).ok_or(ModsError::Stale)?;
        let base = self.mods.base_index(&snapshot.content.paks);
        Ok(mod_contents(row, &utocs, cache, &base))
    }
}
