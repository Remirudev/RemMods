use super::model::ToggleBlock;
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
        }
    }
}
