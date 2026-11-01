//! The game half of the `Core` facade.

use std::path::{Path, PathBuf};

use super::{GameStatus, GameStatusChange, LaunchPlan, LocationChoice, PaksLocation, PickResult};
use crate::{Core, CoreError};

impl Core {
    /// Resolves the location on the first call (concurrent callers wait).
    pub fn game_status(&self) -> GameStatus {
        self.game.ensure_initialised(&self.settings);
        self.game.status()
    }

    pub fn game_detect(&self) -> GameStatus {
        self.game.detect_again(&self.settings);
        self.game.status()
    }

    pub fn game_set_location(&self, choice: LocationChoice) -> Result<GameStatus, CoreError> {
        self.game.set_location(&self.settings, choice)?;
        Ok(self.game.status())
    }

    pub fn game_choose_path(&self, picked: &Path) -> Result<PickResult, CoreError> {
        Ok(self.game.choose_path(&self.settings, picked)?)
    }

    pub fn game_ack_update(&self) -> Result<GameStatus, CoreError> {
        self.game.ack_update(&self.settings);
        Ok(self.game.status())
    }

    pub fn game_launch_plan(&self) -> LaunchPlan {
        self.game.ensure_initialised(&self.settings);
        self.game.launch_plan()
    }

    /// `Some` only while the location is Ready.
    pub fn game_paks(&self) -> Option<PaksLocation> {
        self.game.ensure_initialised(&self.settings);
        self.game.paks()
    }

    /// The Paks folder in the form handed to the shell.
    pub fn game_reveal_target(&self) -> Result<PathBuf, CoreError> {
        self.game.ensure_initialised(&self.settings);
        Ok(self.game.reveal_target()?)
    }

    /// One process scan; `Some` when the running state, session or original app changed.
    pub fn game_poll_running(&self) -> Option<GameStatusChange> {
        self.game.poll_running()
    }

    /// Re-reads the build id; `Some` when the update notice changed.
    pub fn game_refresh_build(&self) -> Option<GameStatusChange> {
        self.game.refresh_build(&self.settings)
    }
}
