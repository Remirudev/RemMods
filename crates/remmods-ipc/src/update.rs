use remmods_core::app_update;
use serde::{Deserialize, Serialize};

/// How this copy of RemMods was installed; only `appImage` and `nsis` update themselves.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum AppInstallKind {
    AppImage,
    Nsis,
    Deb,
    Rpm,
    Aur,
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateChannel {
    pub install: AppInstallKind,
    /// `install` is `appImage` or `nsis`.
    pub self_update: bool,
    /// The updater plugin is registered (release config, not `--smoke`).
    pub configured: bool,
    /// The running app's version.
    pub current: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AppUpdateCheck {
    UpToDate,
    /// `date` is the raw `pub_date` of `latest.json`; `notes` the release body as plain text.
    Available {
        version: String,
        date: Option<String>,
        notes: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AppUpdateProgress {
    /// Bytes so far and the announced size (null when the server sends none).
    Downloading {
        #[specta(type = specta_typescript::Number)]
        downloaded: f64,
        #[specta(type = Option<specta_typescript::Number>)]
        total: Option<f64>,
    },
    Installing,
    Restarting,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum UpdateBlock {
    InstallRunning,
    JournalPending,
    Busy,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum UpdateStage {
    Check,
    Download,
    Verify,
    Install,
}

impl From<app_update::InstallKind> for AppInstallKind {
    fn from(kind: app_update::InstallKind) -> Self {
        match kind {
            app_update::InstallKind::AppImage => Self::AppImage,
            app_update::InstallKind::Nsis => Self::Nsis,
            app_update::InstallKind::Deb => Self::Deb,
            app_update::InstallKind::Rpm => Self::Rpm,
            app_update::InstallKind::Aur => Self::Aur,
            app_update::InstallKind::Unknown => Self::Unknown,
        }
    }
}

impl From<app_update::UpdateBlock> for UpdateBlock {
    fn from(block: app_update::UpdateBlock) -> Self {
        match block {
            app_update::UpdateBlock::InstallRunning => Self::InstallRunning,
            app_update::UpdateBlock::JournalPending => Self::JournalPending,
            app_update::UpdateBlock::Busy => Self::Busy,
        }
    }
}
