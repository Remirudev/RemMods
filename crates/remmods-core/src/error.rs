use crate::app_update::AppUpdateError;
use crate::game::GameError;
use crate::graphics::GraphicsError;
use crate::install::InstallError;
use crate::mods::ModsError;
use crate::profiles::ProfileError;
use crate::safe_fs::FsError;
use crate::settings::SettingsError;
use crate::skins::SkinsError;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error(transparent)]
    Fs(#[from] FsError),
    #[error(transparent)]
    Settings(#[from] SettingsError),
    #[error("no home directory: cannot resolve the config and data folders")]
    MissingHomeDir,
    #[error(transparent)]
    Game(#[from] GameError),
    #[error(transparent)]
    Mods(#[from] ModsError),
    #[error(transparent)]
    Profiles(#[from] ProfileError),
    #[error(transparent)]
    Install(#[from] InstallError),
    #[error(transparent)]
    AppUpdate(#[from] AppUpdateError),
    #[error(transparent)]
    Graphics(#[from] GraphicsError),
    #[error(transparent)]
    Skins(#[from] SkinsError),
}
