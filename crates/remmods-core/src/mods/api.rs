use std::path::PathBuf;
use std::sync::Arc;

use super::details::{read_details, shell_form};
use super::model::{ModDetails, ScanInput, ScanSnapshot};
use super::modref::{ModRef, paks_identity};
use super::service::Refreshed;
use crate::game::GameError;
use crate::safe_fs::names::utf16_len;
use crate::{Core, CoreError, platform};

impl Core {
    /// The cached snapshot of the current Paks folder, scanning on the first call.
    pub fn mods_list(&self) -> Result<Arc<ScanSnapshot>, CoreError> {
        let input = self.mods_scan_input()?;
        if let Some(cached) = self.mods.cached(&input) {
            return Ok(cached);
        }
        let _read = self.ops.read()?;
        Ok(self.mods.list(&input)?)
    }

    /// Scans hold the shared app lock, so they never see a batch half done.
    pub fn mods_refresh(&self) -> Result<Refreshed, CoreError> {
        let input = self.mods_scan_input()?;
        let _read = self.ops.read()?;
        Ok(self.mods.refresh(&input)?)
    }

    pub fn mods_details(&self, mod_ref: &str) -> Result<ModDetails, CoreError> {
        let input = self.mods_scan_input()?;
        Ok(read_details(&input, &ModRef::from_token(mod_ref))?)
    }

    /// The path to hand to the file manager (shell form, spec §4.8).
    pub fn mods_reveal_target(&self, mod_ref: &str) -> Result<PathBuf, CoreError> {
        let input = self.mods_scan_input()?;
        Ok(shell_form(
            &input,
            &ModRef::from_token(mod_ref),
            platform::os_kind(),
        )?)
    }

    pub fn mods_last(&self) -> Option<Arc<ScanSnapshot>> {
        self.mods.last()
    }

    pub(super) fn mods_scan_input(&self) -> Result<ScanInput, CoreError> {
        let location = self.game_paks().ok_or(GameError::NotSet)?;
        Ok(ScanInput {
            game_view_len: utf16_len(&location.game_view),
            paks_id: paks_identity(&location.paks, location.generation),
            paks: location.paks,
        })
    }
}
