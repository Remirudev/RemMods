//! RemMods core: all logic, synchronous, no Tauri.
#![deny(unsafe_code)]

pub mod content;
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
    /// Every mutation under Paks and every scan passes this gate (spec §4.6).
    pub ops: safe_fs::OpGate,
    require_marker: Option<String>,
}

/// How `Core` finds the game. `--smoke` and tests turn Steam detection off.
#[derive(Clone, Debug)]
pub struct CoreOptions {
    pub game: game::GameOptions,
    /// `--smoke`: mutations need this regular file in Paks (A9).
    pub require_marker: Option<String>,
}

impl Default for CoreOptions {
    fn default() -> Self {
        Self {
            game: game::GameOptions {
                steam_detection: true,
                fixed_location: None,
                steam_hints: None,
                processes: None,
            },
            require_marker: None,
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
        let ops = safe_fs::OpGate::open(&paths)?;
        let settings = SettingsStore::load(&paths);
        let meta_path = paths.data_dir.join(mods::MOD_META_FILE);
        Ok(Self {
            settings,
            game: game::GameService::new(options.game),
            mods: mods::ModsService::new(meta_path),
            ops,
            require_marker: options.require_marker,
            paths,
        })
    }

    pub fn settings(&self) -> SettingsSnapshot {
        self.settings.snapshot()
    }

    pub fn update_settings(&self, update: &SettingsUpdate) -> Result<SettingsSnapshot, CoreError> {
        Ok(self.settings.update(update)?)
    }
}
