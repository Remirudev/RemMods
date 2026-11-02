use crate::safe_fs::FsError;

/// Why an update may not install now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateBlock {
    InstallRunning,
    JournalPending,
    Busy,
}

#[derive(Debug, thiserror::Error)]
pub enum AppUpdateError {
    #[error("update blocked: {0:?}")]
    Blocked(UpdateBlock),
    #[error(transparent)]
    Fs(#[from] FsError),
}
