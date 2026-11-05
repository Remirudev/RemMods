//! RemMods core: all logic, synchronous, no Tauri.
#![deny(unsafe_code)]

pub mod app_update;
pub mod archive;
pub mod assets;
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
pub mod skins;

pub use error::CoreError;
pub use paths::AppPaths;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
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
    /// Item pictures and names in `data_dir/assets`.
    pub assets: assets::AssetStore,
    pub trash_env: TrashEnv,
    require_marker: Option<String>,
    /// The folder of the game's `SaveGames/<id>` folders when it is given, not found.
    save_root: Option<PathBuf>,
    /// Test hook: one entry per following skin file replacement, after how many files it fails.
    skin_fault: Mutex<VecDeque<Option<usize>>>,
    /// Test hook: one entry per following manage mutation (rename, set-aside, profile apply,
    /// trash; a bulk row takes one each), in order.
    fault: Mutex<VecDeque<Option<FaultPlan>>>,
    /// Test hook: the final rescan of the next bulk call fails.
    fail_final_refresh: AtomicBool,
}

/// How `Core` finds the game. `--smoke` and tests turn Steam detection off.
#[derive(Clone, Debug)]
pub struct CoreOptions {
    pub game: game::GameOptions,
    /// `--smoke`: mutations need this regular file in Paks (A9).
    pub require_marker: Option<String>,
    /// Where trashed mods go; the default refuses every trash.
    pub trash: TrashEnv,
    /// The folder holding `SaveGames/<id>`: tests pass their own, so no test sees real saves.
    /// `None` looks for it from the game folder, except under `--smoke`.
    pub save_root: Option<PathBuf>,
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
            save_root: None,
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
            assets: assets::AssetStore::new(&paths),
            trash_env: options.trash,
            require_marker: options.require_marker,
            save_root: options.save_root,
            skin_fault: Mutex::new(VecDeque::new()),
            fault: Mutex::new(VecDeque::new()),
            fail_final_refresh: AtomicBool::new(false),
            paths,
        })
    }

    /// Test hook: each entry stops one following replacement of the skin files after that many.
    #[doc(hidden)]
    pub fn inject_skin_faults(&self, faults: Vec<Option<usize>>) {
        self.skin_fault
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend(faults);
    }

    pub(crate) fn take_skin_fault(&self) -> Option<usize> {
        self.skin_fault
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front()
            .flatten()
    }

    /// Test hook: the next manage batch (rename, set-aside, profile apply, trash) fails as `plan`
    /// says.
    #[doc(hidden)]
    pub fn inject_batch_fault(&self, plan: FaultPlan) {
        self.inject_batch_faults(vec![Some(plan)]);
    }

    /// Test hook: one entry per following manage mutation (batch or trash call), in order;
    /// None = no fault.
    #[doc(hidden)]
    pub fn inject_batch_faults(&self, plans: Vec<Option<FaultPlan>>) {
        *self.fault.lock().unwrap_or_else(PoisonError::into_inner) = plans.into();
    }

    pub(crate) fn take_batch_fault(&self) -> Option<FaultPlan> {
        self.fault
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front()
            .flatten()
    }

    /// Test hook: the final rescan of the next mutation fails (manage, bulk, toggle or install;
    /// the rows of a bulk call still count).
    #[doc(hidden)]
    pub fn fail_next_final_refresh(&self) {
        self.fail_final_refresh.store(true, Ordering::SeqCst);
    }

    pub(crate) fn take_final_refresh_failure(&self) -> bool {
        self.fail_final_refresh.swap(false, Ordering::SeqCst)
    }

    pub fn settings(&self) -> SettingsSnapshot {
        self.settings.snapshot()
    }

    pub fn update_settings(&self, update: &SettingsUpdate) -> Result<SettingsSnapshot, CoreError> {
        Ok(self.settings.update(update)?)
    }
}
