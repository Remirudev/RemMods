use std::path::{Path, PathBuf};

use crate::error::CoreError;

pub const APP_IDENTIFIER: &str = "com.remirudev.remmods";

/// Where RemMods keeps its own files. Resolved like Tauri's `PathResolver` (both use `dirs`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub log_dir: PathBuf,
    pub settings_file: PathBuf,
    pub lock_file: PathBuf,
}

impl AppPaths {
    /// Linux: `~/.config/<id>` + `~/.local/share/<id>`; Windows: `%APPDATA%\<id>` + `%LOCALAPPDATA%\<id>`.
    pub fn for_user() -> Result<Self, CoreError> {
        let config_root = dirs::config_dir().ok_or(CoreError::MissingHomeDir)?;
        let data_root = dirs::data_local_dir().ok_or(CoreError::MissingHomeDir)?;
        Ok(Self::from_dirs(
            config_root.join(APP_IDENTIFIER),
            data_root.join(APP_IDENTIFIER),
        ))
    }

    /// Everything below one root (tests and `--smoke`): `root/config`, `root/data`, `root/data/logs`.
    pub fn under(root: &Path) -> Self {
        Self::from_dirs(root.join("config"), root.join("data"))
    }

    /// `data_dir/profiles.json`.
    pub fn profiles_file(&self) -> PathBuf {
        self.data_dir.join(crate::profiles::PROFILES_FILE)
    }

    fn from_dirs(config_dir: PathBuf, data_dir: PathBuf) -> Self {
        Self {
            log_dir: data_dir.join("logs"),
            settings_file: config_dir.join("settings.json"),
            lock_file: data_dir.join("remmods.lock"),
            config_dir,
            data_dir,
        }
    }
}

/// Path for display in the UI; strips Windows' `\\?\` prefix where that is lossless.
pub fn display_path(path: &Path) -> String {
    dunce::simplified(path).display().to_string()
}

/// Windows: `%LOCALAPPDATA%\TekkenModManager` if it is a folder; else None. Read-only.
pub fn tmm_profiles_dir() -> Option<PathBuf> {
    if crate::platform::os_kind() != crate::platform::OsKind::Windows {
        return None;
    }
    let dir = dirs::data_local_dir()?.join("TekkenModManager");
    dir.is_dir().then_some(dir)
}
