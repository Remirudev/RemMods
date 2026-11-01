use remmods_core::CoreError;
use remmods_core::game::GameError;
use remmods_core::mods::ModsError;
use remmods_core::platform;
use remmods_core::safe_fs::{BatchError, FsError};
use remmods_core::settings::SettingsError;
use serde::{Deserialize, Serialize};

use crate::game::{FolderProblem, RevealProblem};
use crate::toggle::ToggleBlock;

/// Error returned by every command. Carries no English: the UI translates by `kind`, and the raw
/// OS error only goes to the log.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AppError {
    Busy,
    Io {
        op: String,
        class: OsErrorClass,
    },
    InvalidInput {
        field: String,
    },
    ReadOnly,
    Internal,
    Stale,
    GameNotSet,
    InvalidGameFolder {
        reason: FolderProblem,
    },
    LaunchFailed,
    RevealFailed {
        reason: RevealProblem,
    },
    GameRunning,
    TargetExists,
    OutOfScope,
    ToggleBlocked {
        reason: ToggleBlock,
    },
    /// A rename batch failed and was rolled back; `stuck` ops could not be undone (recovered on
    /// the next start), `folder` says a folder rename was refused because something held it.
    BatchFailed {
        class: OsErrorClass,
        stuck: u32,
        folder: bool,
    },
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
    AlreadyExists,
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
        FsError::OutOfScope { .. } => AppError::OutOfScope,
        FsError::TargetExists { .. } => AppError::TargetExists,
        FsError::SourceMissing { .. } => AppError::Stale,
        FsError::Batch(batch) => batch_failed(&batch),
        FsError::FolderBusy { .. } | FsError::InvalidBatch { .. } | FsError::Journal { .. } => {
            log::error!("unexpected file-system error: {error}");
            AppError::Internal
        }
    }
}

fn batch_failed(batch: &BatchError) -> AppError {
    AppError::BatchFailed {
        class: batch
            .cause
            .class()
            .map_or(OsErrorClass::Other, OsErrorClass::from),
        stuck: u32::try_from(batch.stuck.len()).unwrap_or(u32::MAX),
        folder: matches!(batch.cause, FsError::FolderBusy { .. }),
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
        ModsError::Blocked { reason, .. } => AppError::ToggleBlocked {
            reason: reason.into(),
        },
        ModsError::GameRunning => AppError::GameRunning,
        ModsError::NotAFixture => AppError::invalid_input("fixture"),
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
            platform::OsErrorClass::AlreadyExists => Self::AlreadyExists,
            platform::OsErrorClass::NotFound => Self::NotFound,
            platform::OsErrorClass::Unsupported => Self::Unsupported,
            platform::OsErrorClass::Other => Self::Other,
        }
    }
}
