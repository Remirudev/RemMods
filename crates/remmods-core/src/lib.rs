//! RemMods core: all logic, synchronous, no Tauri.
#![deny(unsafe_code)]

pub mod app_update;
pub mod archive;
pub mod content;
pub mod error;
pub mod game;
pub mod graphics;
pub mod install;
pub mod mods;
pub mod navigation;
pub mod paths;
#[allow(unsafe_code)]
pub mod platform;
pub mod profiles;
pub mod safe_fs;
pub mod settings;

pub use error::CoreError;
pub use paths::AppPaths;

use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use platform::{TopdirPolicy, TrashEnv};
use safe_fs::FaultPlan;
use settings::{SettingsSnapshot, SettingsStore, SettingsUpdate};

/// Facade the Tauri shell talks to. The game and mods methods live in `game/api.rs` and
/// `mods/api.rs`.
#[derive(Debug)]
pub struct Core {
    pub paths: AppPaths,
    pub settings: SettingsStore,
    pub game: game::GameService,
    pub mods: mods::ModsService,
    /// Every mutation under Paks and every scan passes this gate (spec §4.6).
    pub ops: safe_fs::OpGate,
    /// RemMods' own `data_dir/profiles.json`.
    pub profiles: profiles::ProfileStore,
    /// The second confirmation of permanent deletes (spec §5.4).
    pub delete_tokens: mods::DeleteTokens,
    /// Inspections and the one install job (P4 spec §5.5).
    pub install: install::InstallService,
    pub trash_env: TrashEnv,
    require_marker: Option<String>,
    /// The original app's data folder (Windows only); exports never go there.
    tmm_dir: Option<PathBuf>,
    /// Test hook: the next manage batch (rename, set-aside, profile apply, trash) runs with it.
    fault: Mutex<Option<FaultPlan>>,
}

/// How `Core` finds the game. `--smoke` and tests turn Steam detection off.
#[derive(Clone, Debug)]
pub struct CoreOptions {
    pub game: game::GameOptions,
    /// `--smoke`: mutations need this regular file in Paks (A9).
    pub require_marker: Option<String>,
    /// Where trashed mods go; the default refuses every trash.
    pub trash: TrashEnv,
    /// `paths::tmm_profiles_dir()` in the shell.
    pub tmm_dir: Option<PathBuf>,
}

impl Default for CoreOptions {
    fn default() -> Self {
        Self {
            game: game::GameOptions {
                steam_detection: true,
                fixed_location: None,
                steam_hints: None,
                processes: None,
            },
            require_marker: None,
            trash: TrashEnv {
                home_trash: None,
                uid: None,
                topdirs: TopdirPolicy::Deny,
            },
            tmm_dir: None,
        }
    }
}

impl Core {
    pub fn open(paths: AppPaths) -> Result<Self, CoreError> {
        Self::open_with(paths, CoreOptions::default())
    }

    /// Game detection and the first scan are lazy: this touches only the app's own folders.
    pub fn open_with(paths: AppPaths, options: CoreOptions) -> Result<Self, CoreError> {
        safe_fs::ensure_dir(&paths.data_dir)?;
        safe_fs::ensure_dir(&paths.log_dir)?;
        let ops = safe_fs::OpGate::open(&paths)?;
        let settings = SettingsStore::load(&paths);
        let meta_path = paths.data_dir.join(mods::MOD_META_FILE);
        Ok(Self {
            settings,
            game: game::GameService::new(options.game),
            mods: mods::ModsService::new(meta_path),
            ops,
            profiles: profiles::ProfileStore::new(paths.profiles_file()),
            delete_tokens: mods::DeleteTokens::new(),
            install: install::InstallService::default(),
            trash_env: options.trash,
            require_marker: options.require_marker,
            tmm_dir: options.tmm_dir,
            fault: Mutex::new(None),
            paths,
        })
    }

    /// Test hook: the next manage batch (rename, set-aside, profile apply, trash) fails as `plan`
    /// says.
    #[doc(hidden)]
    pub fn inject_batch_fault(&self, plan: FaultPlan) {
        *self.fault.lock().unwrap_or_else(PoisonError::into_inner) = Some(plan);
    }

    pub(crate) fn take_batch_fault(&self) -> Option<FaultPlan> {
        self.fault
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }

    pub(crate) fn tmm_dir(&self) -> Option<&std::path::Path> {
        self.tmm_dir.as_deref()
    }

    pub fn settings(&self) -> SettingsSnapshot {
        self.settings.snapshot()
    }

    pub fn update_settings(&self, update: &SettingsUpdate) -> Result<SettingsSnapshot, CoreError> {
        Ok(self.settings.update(update)?)
    }
}
