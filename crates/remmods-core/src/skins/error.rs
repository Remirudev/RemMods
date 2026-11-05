use crate::safe_fs::FsError;

#[derive(Debug, thiserror::Error)]
pub enum SkinsError {
    #[error(transparent)]
    Fs(#[from] FsError),
    #[error("the game's skin save files were not found")]
    NoSaves,
    #[error("the game is running")]
    GameRunning,
    #[error("no such skin set")]
    UnknownSet,
    #[error("a skin set with this name exists")]
    NameTaken,
    #[error("the name is empty or too long")]
    InvalidName,
    #[error("the notes are too long")]
    NotesTooLong,
    #[error("this set is the one in the game")]
    SetInUse,
    #[error("the stored copy of the set is incomplete")]
    BrokenSet,
    #[error("an interrupted switch has to be finished first")]
    Interrupted,
    #[error("the switch failed (old files back: {restored})")]
    SwitchFailed { restored: bool },
    #[error("the skin set list comes from a newer RemMods")]
    ReadOnly,
    #[error("the skin set list cannot be encoded: {0}")]
    Encode(#[from] serde_json::Error),
}
