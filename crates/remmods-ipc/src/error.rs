use remmods_core::CoreError;
use remmods_core::game::GameError;
use remmods_core::mods::ModsError;
use remmods_core::platform;
use remmods_core::safe_fs::FsError;
use remmods_core::settings::SettingsError;
use serde::{Deserialize, Serialize};

use crate::game::{FolderProblem, RevealProblem};

/// Error returned by every command. Carries no English: the UI translates by `kind`, and the raw
/// OS error only goes to the log.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AppError {
    Busy,
    Io { op: String, class: OsErrorClass },
    InvalidInput { field: String },
    ReadOnly,
    Internal,
    Stale,
    GameNotSet,
    InvalidGameFolder { reason: FolderProblem },
    LaunchFailed,
    RevealFailed { reason: RevealProblem },
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum OsErrorClass {
    InUse,
    AccessDenied,
    NoSpace,
    ReadOnlyFs,
    CrossDevice,
    NameTooLong,
    NotFound,
    Unsupported,
    Other,
}

impl AppError {
    pub fn invalid_input(field: &str) -> Self {
        Self::InvalidInput {
            field: field.to_owned(),
        }
    }
}

impl From<CoreError> for AppError {
    fn from(error: CoreError) -> Self {
        log::warn!("command failed: {error}");
        match error {
            CoreError::Fs(error) => fs_error(error),
            CoreError::Settings(error) => settings_error(error),
            CoreError::MissingHomeDir => Self::Internal,
            CoreError::Game(error) => game_error(error),
            CoreError::Mods(error) => mods_error(error),
        }
    }
}

fn fs_error(error: FsError) -> AppError {
    match error {
        FsError::Busy => AppError::Busy,
        FsError::Io { op, class, .. } => AppError::Io {
            op: op.to_owned(),
            class: class.into(),
        },
    }
}

fn settings_error(error: SettingsError) -> AppError {
    match error {
        SettingsError::ReadOnly => AppError::ReadOnly,
        SettingsError::InvalidValue { field } => AppError::invalid_input(field),
        SettingsError::Encode(_) => AppError::Internal,
        SettingsError::Fs(error) => fs_error(error),
    }
}

fn game_error(error: GameError) -> AppError {
    match error {
        GameError::NotSet => AppError::GameNotSet,
        GameError::InvalidFolder(problem) => AppError::InvalidGameFolder {
            reason: problem.into(),
        },
        GameError::StaleCandidate => AppError::Stale,
        GameError::InvalidPath => AppError::invalid_input("path"),
    }
}

fn mods_error(error: ModsError) -> AppError {
    match error {
        ModsError::Fs(error) => fs_error(error),
        ModsError::Stale => AppError::Stale,
        ModsError::BadRef => AppError::invalid_input("modRef"),
        ModsError::NotShellSafe => AppError::RevealFailed {
            reason: RevealProblem::UnsupportedName,
        },
    }
}

impl From<platform::OsErrorClass> for OsErrorClass {
    fn from(class: platform::OsErrorClass) -> Self {
        match class {
            platform::OsErrorClass::InUse => Self::InUse,
            platform::OsErrorClass::AccessDenied => Self::AccessDenied,
            platform::OsErrorClass::NoSpace => Self::NoSpace,
            platform::OsErrorClass::ReadOnlyFs => Self::ReadOnlyFs,
            platform::OsErrorClass::CrossDevice => Self::CrossDevice,
            platform::OsErrorClass::NameTooLong => Self::NameTooLong,
            platform::OsErrorClass::NotFound => Self::NotFound,
            platform::OsErrorClass::Unsupported => Self::Unsupported,
            platform::OsErrorClass::Other => Self::Other,
        }
    }
}
