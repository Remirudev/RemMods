use super::manage_target::{ManageAction, ManageBlock};
use super::model::ToggleBlock;
use crate::platform::TrashUnavailableReason;
use crate::safe_fs::FsError;

#[derive(Debug, thiserror::Error)]
pub enum ModsError {
    /// Paks could not be listed (op "scan") or a read for details failed.
    #[error(transparent)]
    Fs(#[from] FsError),
    #[error("the entry no longer matches the disk")]
    Stale,
    #[error("malformed mod reference")]
    BadRef,
    #[error("the name cannot be handed to the Windows shell")]
    NotShellSafe,
    #[error("{key} cannot be toggled ({reason:?})")]
    Blocked { key: String, reason: ToggleBlock },
    #[error("the game is running")]
    GameRunning,
    #[error("the Paks folder has no smoke-fixture marker")]
    NotAFixture,
    #[error("the new name equals the current one")]
    RenameUnchanged,
    #[error("the name is not allowed ({code})")]
    InvalidName { code: &'static str },
    #[error("an entry with that name exists")]
    TargetExists,
    #[error("{key} cannot be managed ({action:?}: {reason:?})")]
    ManageBlocked {
        key: String,
        action: ManageAction,
        reason: ManageBlock,
    },
    #[error("no usable trash ({reason:?})")]
    TrashUnavailable {
        reason: TrashUnavailableReason,
        size: Option<u64>,
    },
    #[error("the delete confirmation is missing, expired or for another mod")]
    DeleteToken,
    #[error("the mod folder has several modinfo.json case variants")]
    ModinfoAmbiguous,
    #[error("the {field} field is too long")]
    FieldTooLong { field: &'static str },
    #[error("the bulk target list is empty, too long or names a row twice")]
    BulkTargets,
}

impl ModsError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Fs(_) => "fs",
            Self::Stale => "stale",
            Self::BadRef => "bad_ref",
            Self::NotShellSafe => "not_shell_safe",
            Self::Blocked { .. } => "blocked",
            Self::GameRunning => "game_running",
            Self::NotAFixture => "not_a_fixture",
            Self::RenameUnchanged => "rename_unchanged",
            Self::InvalidName { .. } => "invalid_name",
            Self::TargetExists => "target_exists",
            Self::ManageBlocked { .. } => "manage_blocked",
            Self::TrashUnavailable { .. } => "trash_unavailable",
            Self::DeleteToken => "delete_token",
            Self::ModinfoAmbiguous => "modinfo_ambiguous",
            Self::FieldTooLong { .. } => "field_too_long",
            Self::BulkTargets => "bulk_targets",
        }
    }
}
