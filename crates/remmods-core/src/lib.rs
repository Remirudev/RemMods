//! RemMods core: all logic, synchronous, no Tauri.
#![deny(unsafe_code)]

pub mod error;
pub mod navigation;
pub mod paths;
#[allow(unsafe_code)]
pub mod platform;
pub mod safe_fs;
pub mod settings;

pub use error::CoreError;
pub use paths::AppPaths;

use settings::{SettingsSnapshot, SettingsStore, SettingsUpdate};

/// Facade the Tauri shell talks to.
#[derive(Debug)]
pub struct Core {
    pub paths: AppPaths,
    pub settings: SettingsStore,
}

impl Core {
    pub fn open(paths: AppPaths) -> Result<Self, CoreError> {
        safe_fs::ensure_dir(&paths.data_dir)?;
        safe_fs::ensure_dir(&paths.log_dir)?;
        let settings = SettingsStore::load(&paths);
        Ok(Self { paths, settings })
    }

    pub fn settings(&self) -> SettingsSnapshot {
        self.settings.snapshot()
    }

    pub fn update_settings(&self, update: &SettingsUpdate) -> Result<SettingsSnapshot, CoreError> {
        Ok(self.settings.update(update)?)
    }
}
