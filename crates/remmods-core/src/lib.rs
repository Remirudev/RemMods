//! RemMods core: all logic, synchronous, no Tauri.
#![deny(unsafe_code)]

pub mod error;
pub mod game;
pub mod mods;
pub mod navigation;
pub mod paths;
#[allow(unsafe_code)]
pub mod platform;
pub mod safe_fs;
pub mod settings;

pub use error::CoreError;
pub use paths::AppPaths;

use settings::{SettingsSnapshot, SettingsStore, SettingsUpdate};

/// Facade the Tauri shell talks to. The game and mods methods live in `game/api.rs` and
/// `mods/api.rs`.
#[derive(Debug)]
pub struct Core {
    pub paths: AppPaths,
    pub settings: SettingsStore,
    pub game: game::GameService,
    pub mods: mods::ModsService,
}

/// How `Core` finds the game. `--smoke` and tests turn Steam detection off.
#[derive(Clone, Debug)]
pub struct CoreOptions {
    pub game: game::GameOptions,
}

impl Default for CoreOptions {
    fn default() -> Self {
        Self {
            game: game::GameOptions {
                steam_detection: true,
                fixed_location: None,
                steam_hints: None,
            },
        }
    }
}

impl Core {
    pub fn open(paths: AppPaths) -> Result<Self, CoreError> {
        Self::open_with(paths, CoreOptions::default())
    }

    /// Game detection and the first scan are lazy: this touches only the app's own folders.
    pub fn open_with(paths: AppPaths, options: CoreOptions) -> Result<Self, CoreError> {
        safe_fs::ensure_dir(&paths.data_dir)?;
        safe_fs::ensure_dir(&paths.log_dir)?;
        let settings = SettingsStore::load(&paths);
        Ok(Self {
            paths,
            settings,
            game: game::GameService::new(options.game),
            mods: mods::ModsService::new(),
        })
    }

    pub fn settings(&self) -> SettingsSnapshot {
        self.settings.snapshot()
    }

    pub fn update_settings(&self, update: &SettingsUpdate) -> Result<SettingsSnapshot, CoreError> {
        Ok(self.settings.update(update)?)
    }
}
