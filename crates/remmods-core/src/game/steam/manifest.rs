//! `appmanifest_1778820.acf`: install folder name, build id and install state of TEKKEN 8.

use super::vdf::{VdfError, parse_vdf_bytes};
use crate::game::STEAM_APP_ID;

/// StateFlags bits (Steam's EAppState).
const STATE_UPDATE_REQUIRED: u32 = 2;
const STATE_FULLY_INSTALLED: u32 = 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppManifest {
    pub app_id: u32,
    pub install_dir: String,
    pub build_id: String,
    pub state_flags: u32,
    pub last_updated: Option<i64>,
    pub target_build_id: Option<String>,
    pub name: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManifestError {
    Vdf(VdfError),
    WrongRoot,
    MissingKey(&'static str),
    WrongAppId,
    UnsafeInstallDir,
    BadBuildId,
    BadNumber(&'static str),
}

impl ManifestError {
    /// camelCase code the UI translates; the key name of `MissingKey`/`BadNumber` is for logs.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Vdf(error) => error.kind.code(),
            Self::WrongRoot => "wrongRoot",
            Self::MissingKey(_) => "missingKey",
            Self::WrongAppId => "wrongAppId",
            Self::UnsafeInstallDir => "unsafeInstallDir",
            Self::BadBuildId => "badBuildId",
            Self::BadNumber(_) => "badNumber",
        }
    }

    pub fn line(&self) -> Option<u32> {
        match self {
            Self::Vdf(error) => Some(error.line),
            _ => None,
        }
    }
}

impl AppManifest {
    pub fn fully_installed(&self) -> bool {
        self.state_flags & STATE_FULLY_INSTALLED != 0
    }

    /// Steam flags an update, or a target build newer than the installed one is announced.
    pub fn update_pending(&self) -> bool {
        self.state_flags & STATE_UPDATE_REQUIRED != 0
            || self
                .target_build_id
                .as_deref()
                .is_some_and(|target| is_digits(target) && digits_greater(target, &self.build_id))
    }
}

pub fn parse_app_manifest(bytes: &[u8]) -> Result<AppManifest, ManifestError> {
    let (root, tree) = parse_vdf_bytes(bytes).map_err(ManifestError::Vdf)?;
    if !root.eq_ignore_ascii_case("AppState") {
        return Err(ManifestError::WrongRoot);
    }
    let app_id = tree
        .str("appid")
        .ok_or(ManifestError::MissingKey("appid"))?;
    if app_id != STEAM_APP_ID.to_string() {
        return Err(ManifestError::WrongAppId);
    }
    let install_dir = tree
        .str("installdir")
        .ok_or(ManifestError::MissingKey("installdir"))?;
    if !is_plain_component(install_dir) {
        return Err(ManifestError::UnsafeInstallDir);
    }
    let build_id = tree
        .str("buildid")
        .ok_or(ManifestError::MissingKey("buildid"))?;
    if !is_digits(build_id) {
        return Err(ManifestError::BadBuildId);
    }
    let state_flags = match tree.str("StateFlags") {
        Some(text) => text
            .parse::<u32>()
            .map_err(|_| ManifestError::BadNumber("StateFlags"))?,
        None => 0,
    };
    let last_updated = tree
        .str("LastUpdated")
        .map(|text| {
            text.parse::<i64>()
                .map_err(|_| ManifestError::BadNumber("LastUpdated"))
        })
        .transpose()?;
    Ok(AppManifest {
        app_id: STEAM_APP_ID,
        install_dir: install_dir.to_owned(),
        build_id: build_id.to_owned(),
        state_flags,
        last_updated,
        target_build_id: tree.str("TargetBuildID").map(str::to_owned),
        name: tree.str("name").map(str::to_owned),
    })
}

/// One plain folder name: the manifest must not point outside `steamapps/common`.
fn is_plain_component(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', ':', '\0'])
}

fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// Numeric comparison of two ASCII digit strings of any length.
fn digits_greater(left: &str, right: &str) -> bool {
    let left = left.trim_start_matches('0');
    let right = right.trim_start_matches('0');
    (left.len(), left) > (right.len(), right)
}
