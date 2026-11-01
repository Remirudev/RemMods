use super::FolderProblem;

#[derive(Debug, thiserror::Error)]
pub enum GameError {
    #[error("no game folder is set")]
    NotSet,
    #[error("not a TEKKEN 8 folder: {0:?}")]
    InvalidFolder(FolderProblem),
    #[error("the chosen installation is out of date")]
    StaleCandidate,
    #[error("the path is empty or not absolute")]
    InvalidPath,
}

impl GameError {
    /// Stable snake_case code for logs and tests.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotSet => "game_not_set",
            Self::InvalidFolder(_) => "invalid_game_folder",
            Self::StaleCandidate => "stale_candidate",
            Self::InvalidPath => "invalid_path",
        }
    }
}
