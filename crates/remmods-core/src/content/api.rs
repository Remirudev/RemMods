use super::listing::mod_contents;
use super::model::ModContents;
use crate::mods::{ModsError, Refreshed};
use crate::{Core, CoreError};

impl Core {
    /// The asset list of one row of the current snapshot; reads only `.utoc` indexes. Never builds
    /// the base index: before it exists the result says `Loading`.
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
        let base = self.mods.stored_base_index(&snapshot.content.paks);
        Ok(mod_contents(row, &utocs, cache, base.as_deref()))
    }

    /// Builds the base-game index when it is missing or its containers changed (slow: meant for a
    /// background thread), then rescans with cached containers so content conflicts use it.
    /// `Some` when that published a new revision.
    pub fn mods_base_refresh(&self) -> Result<Option<Refreshed>, CoreError> {
        let input = self.mods_scan_input()?;
        let _gate = self.mods.base_gate();
        if !self.mods.update_base_index(&input.paks) {
            return Ok(None);
        }
        let _read = self.ops.read()?;
        let refreshed = self.mods.refresh(&input)?;
        Ok(refreshed.changed.then_some(refreshed))
    }
}
