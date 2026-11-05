use remmods_core::CoreError;
use remmods_core::app_update::AppUpdateError;
use remmods_core::archive::{ArchiveError, UnsupportedReason};
use remmods_core::game::GameError;
use remmods_core::graphics::GraphicsError;
use remmods_core::install::InstallError;
use remmods_core::mods::{
    ManageAction as CoreManageAction, ManageBlock as CoreManageBlock, ModsError,
};
use remmods_core::platform;
use remmods_core::profiles::ProfileError;
use remmods_core::safe_fs::{BatchError, FsError};
use remmods_core::settings::SettingsError;
use remmods_core::skins::SkinsError;
use serde::{Deserialize, Serialize};

use crate::assets::PictureProblem;
use crate::game::{FolderProblem, RevealProblem};
use crate::install::{ArchiveProblem, InstallBlock, SourceReject};
use crate::manage::{ManageAction, ManageBlock, TrashUnavailableReason};
use crate::mods::bytes;
use crate::platform::PlatformFeature;
use crate::profiles::ImportProblem;
use crate::toggle::ToggleBlock;
use crate::update::{AppInstallKind, UpdateBlock, UpdateStage};

/// Error returned by every command. Carries no English: the UI translates by `kind`, and the raw
/// OS error only goes to the log.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
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
    /// A name core refused; `issue` is the name-rule code, e.g. `reserved_device_name`.
    InvalidName {
        issue: String,
    },
    ManageBlocked {
        action: ManageAction,
        reason: ManageBlock,
    },
    /// `size` (bytes) lets the UI offer a permanent delete instead; null when unknown.
    TrashUnavailable {
        reason: TrashUnavailableReason,
        #[specta(type = Option<specta_typescript::Number>)]
        size: Option<f64>,
    },
    DeleteTokenInvalid,
    ModinfoAmbiguous,
    ProfileNotFound,
    ProfileNameTaken,
    ImportFailed {
        problem: ImportProblem,
    },
    ArchiveFailed {
        reason: ArchiveProblem,
    },
    InstallBlocked {
        reason: InstallBlock,
    },
    InstallBusy,
    InstallCancelled,
    /// Bytes the install needs and the bytes free in the Paks file system.
    NoSpace {
        #[specta(type = specta_typescript::Number)]
        needed: f64,
        #[specta(type = specta_typescript::Number)]
        available: f64,
    },
    SourceRejected {
        reason: SourceReject,
    },
    PictureRefused {
        reason: PictureProblem,
    },
    /// The game's skin save files were not found.
    SkinsUnavailable,
    SkinSetNameTaken,
    /// The set in the game can't be deleted.
    SkinSetInUse,
    /// A stored set lost files.
    SkinSetBroken,
    /// An earlier switch has to be finished first.
    SkinSwitchInterrupted,
    /// `restored`: the game's old files are back in place.
    SkinSwitchFailed {
        restored: bool,
    },
    SkinSetsReadOnly,
    /// This build carries no updater config (local, dev, CI and `--smoke` builds).
    UpdateNotConfigured,
    /// This install kind updates through its package manager, never by itself.
    UpdateNotAllowed {
        install: AppInstallKind,
    },
    UpdateBlocked {
        reason: UpdateBlock,
    },
    UpdateFailed {
        stage: UpdateStage,
    },
    Unsupported {
        feature: PlatformFeature,
    },
    /// The user cancelled a wait for a folder in use; nothing was changed.
    BusyCancelled,
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
        quiet(error)
    }
}

/// The conversion without the per-error warning: bulk rows and prepare refusals log once per
/// call instead.
pub(crate) fn quiet(error: CoreError) -> AppError {
    match error {
        CoreError::Fs(error) => fs_error(error),
        CoreError::Settings(error) => settings_error(error),
        CoreError::MissingHomeDir => AppError::Internal,
        CoreError::Game(error) => game_error(error),
        CoreError::Mods(error) => mods_error(error),
        CoreError::Profiles(error) => profile_error(error),
        CoreError::Install(error) => install_error(error),
        CoreError::AppUpdate(error) => app_update_error(error),
        CoreError::Graphics(error) => graphics_error(error),
        CoreError::Skins(error) => skins_error(error),
    }
}

fn skins_error(error: SkinsError) -> AppError {
    match error {
        SkinsError::Fs(error) => fs_error(error),
        SkinsError::NoSaves => AppError::SkinsUnavailable,
        SkinsError::GameRunning => AppError::GameRunning,
        SkinsError::UnknownSet => AppError::Stale,
        SkinsError::NameTaken => AppError::SkinSetNameTaken,
        SkinsError::InvalidName => AppError::invalid_input("skinSetName"),
        SkinsError::NotesTooLong => AppError::invalid_input("skinSetNotes"),
        SkinsError::SetInUse => AppError::SkinSetInUse,
        SkinsError::BrokenSet => AppError::SkinSetBroken,
        SkinsError::Interrupted => AppError::SkinSwitchInterrupted,
        SkinsError::SwitchFailed { restored } => AppError::SkinSwitchFailed { restored },
        SkinsError::ReadOnly => AppError::SkinSetsReadOnly,
        SkinsError::Encode(_) => AppError::Internal,
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
        FsError::TrashUnavailable { reason } => AppError::TrashUnavailable {
            reason: reason.into(),
            size: None,
        },
        FsError::NotAnAside { .. } | FsError::AsideNotInert { .. } => AppError::Stale,
        FsError::AsideJournaled { .. } => AppError::ManageBlocked {
            action: CoreManageAction::Delete.into(),
            reason: CoreManageBlock::JournalPending.into(),
        },
        FsError::BusyCancelled { .. } => AppError::BusyCancelled,
        FsError::FolderBusy { .. } | FsError::InvalidBatch { .. } | FsError::Journal { .. } => {
            log::error!("unexpected file-system error: {error}");
            AppError::Internal
        }
    }
}

/// A cancelled wait that rolled back completely changed nothing; one that left stuck ops is
/// reported like a folder held by another program.
fn batch_failed(batch: &BatchError) -> AppError {
    let stuck = u32::try_from(batch.stuck.len()).unwrap_or(u32::MAX);
    if matches!(batch.cause, FsError::BusyCancelled { .. }) {
        if stuck == 0 {
            return AppError::BusyCancelled;
        }
        return AppError::BatchFailed {
            class: OsErrorClass::InUse,
            stuck,
            folder: true,
        };
    }
    AppError::BatchFailed {
        class: batch
            .cause
            .class()
            .map_or(OsErrorClass::Other, OsErrorClass::from),
        stuck,
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
        ModsError::RenameUnchanged => AppError::invalid_input("name"),
        ModsError::InvalidName { code } => AppError::InvalidName {
            issue: code.to_owned(),
        },
        ModsError::TargetExists => AppError::TargetExists,
        ModsError::ManageBlocked { action, reason, .. } => AppError::ManageBlocked {
            action: action.into(),
            reason: reason.into(),
        },
        ModsError::TrashUnavailable { reason, size } => AppError::TrashUnavailable {
            reason: reason.into(),
            size: size.map(bytes),
        },
        ModsError::DeleteToken => AppError::DeleteTokenInvalid,
        ModsError::ModinfoAmbiguous => AppError::ModinfoAmbiguous,
        ModsError::FieldTooLong { field } => AppError::invalid_input(field),
        ModsError::BulkTargets => AppError::invalid_input("targets"),
        ModsError::Picture { problem } => AppError::PictureRefused {
            reason: problem.into(),
        },
    }
}

fn profile_error(error: ProfileError) -> AppError {
    match error {
        ProfileError::Fs(error) => fs_error(error),
        ProfileError::NotFound => AppError::ProfileNotFound,
        ProfileError::InvalidName => AppError::invalid_input("profileName"),
        ProfileError::NameTaken => AppError::ProfileNameTaken,
        ProfileError::InvalidEntries => AppError::invalid_input("profileMods"),
        ProfileError::ReadOnly => AppError::ReadOnly,
        ProfileError::Unparsable(problem) => AppError::ImportFailed {
            problem: problem.into(),
        },
        ProfileError::DiffChanged => AppError::Stale,
    }
}

fn install_error(error: InstallError) -> AppError {
    match error {
        InstallError::Archive(error) => archive_error(error),
        InstallError::Busy => AppError::InstallBusy,
        InstallError::UnknownInspection | InstallError::SourceChanged => AppError::Stale,
        InstallError::Blocked { reason } => AppError::InstallBlocked {
            reason: reason.into(),
        },
        InstallError::NoSpace { needed, available } => AppError::NoSpace {
            needed: bytes(needed),
            available: bytes(available),
        },
        InstallError::Cancelled => AppError::InstallCancelled,
        InstallError::Source { reason } => AppError::SourceRejected {
            reason: reason.into(),
        },
    }
}

fn app_update_error(error: AppUpdateError) -> AppError {
    match error {
        AppUpdateError::Blocked(block) => AppError::UpdateBlocked {
            reason: block.into(),
        },
        AppUpdateError::Fs(error) => fs_error(error),
    }
}

fn graphics_error(error: GraphicsError) -> AppError {
    match error {
        GraphicsError::ReadOnly => AppError::ReadOnly,
        GraphicsError::NoStep => AppError::Stale,
        GraphicsError::Unsupported => AppError::Unsupported {
            feature: PlatformFeature::GraphicsFallback,
        },
        GraphicsError::Encode(_) => AppError::Internal,
        GraphicsError::Fs(error) => fs_error(error),
    }
}

fn archive_error(error: ArchiveError) -> AppError {
    let failed = |reason| AppError::ArchiveFailed { reason };
    match error {
        ArchiveError::NotAnArchive => failed(ArchiveProblem::NotAnArchive),
        ArchiveError::Unsupported { reason } => failed(match reason {
            UnsupportedReason::MultiVolume => ArchiveProblem::MultiVolume,
            UnsupportedReason::SelfExtracting => ArchiveProblem::SelfExtracting,
            UnsupportedReason::Method => ArchiveProblem::UnsupportedMethod,
            UnsupportedReason::RarDisabled => ArchiveProblem::RarDisabled,
        }),
        ArchiveError::Encrypted => failed(ArchiveProblem::Encrypted),
        ArchiveError::Corrupt { .. } | ArchiveError::SizeMismatch { .. } => {
            failed(ArchiveProblem::Corrupt)
        }
        ArchiveError::MemoryLimit { .. } => failed(ArchiveProblem::MemoryLimit),
        ArchiveError::TooManyEntries { .. } => failed(ArchiveProblem::TooManyEntries),
        ArchiveError::TooLarge { .. } => failed(ArchiveProblem::TooLarge),
        ArchiveError::Cancelled => AppError::InstallCancelled,
        ArchiveError::Io { class, .. } => AppError::Io {
            op: "archive_read".to_owned(),
            class: class.into(),
        },
        ArchiveError::Target { class, .. } => AppError::Io {
            op: "archive_write".to_owned(),
            class: class.into(),
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
            platform::OsErrorClass::AlreadyExists => Self::AlreadyExists,
            platform::OsErrorClass::NotFound => Self::NotFound,
            platform::OsErrorClass::Unsupported => Self::Unsupported,
            platform::OsErrorClass::Other => Self::Other,
        }
    }
}
