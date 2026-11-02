use crate::safe_fs::FsError;

#[derive(Debug, thiserror::Error)]
pub enum GraphicsError {
    #[error("graphics settings are read-only (newer format or unreadable file)")]
    ReadOnly,
    #[error("there is no automatic graphics step to acknowledge")]
    NoStep,
    #[error("the graphics fallback is not supported on this OS")]
    Unsupported,
    #[error("cannot encode graphics settings: {0}")]
    Encode(#[from] serde_json::Error),
    #[error(transparent)]
    Fs(#[from] FsError),
}
