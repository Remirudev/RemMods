use crate::safe_fs::FsError;
use crate::settings::SettingsError;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error(transparent)]
    Fs(#[from] FsError),
    #[error(transparent)]
    Settings(#[from] SettingsError),
    #[error("no home directory: cannot resolve the config and data folders")]
    MissingHomeDir,
}
