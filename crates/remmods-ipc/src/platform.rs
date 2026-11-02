use remmods_core::AppPaths;
use remmods_core::paths::display_path;
use remmods_core::platform;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum OsKind {
    Linux,
    Windows,
    Other,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppPathsDto {
    pub config: String,
    pub data: String,
    pub logs: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlatformInfo {
    pub os: OsKind,
    pub arch: String,
    pub app_version: String,
    pub webview_version: Option<String>,
    pub smoke: bool,
    pub paths: AppPathsDto,
    pub features: PlatformFeatures,
}

/// Which Linux-first features this build supports; the rest report `unsupported`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlatformFeatures {
    pub graphics_fallback: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum PlatformFeature {
    GraphicsFallback,
}

impl From<platform::OsKind> for OsKind {
    fn from(kind: platform::OsKind) -> Self {
        match kind {
            platform::OsKind::Linux => Self::Linux,
            platform::OsKind::Windows => Self::Windows,
            platform::OsKind::Other => Self::Other,
        }
    }
}

impl From<&AppPaths> for AppPathsDto {
    fn from(paths: &AppPaths) -> Self {
        Self {
            config: display_path(&paths.config_dir),
            data: display_path(&paths.data_dir),
            logs: display_path(&paths.log_dir),
        }
    }
}
