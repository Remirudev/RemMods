use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use remmods_core::Core;
use remmods_core::app_update::InstallKind;

use crate::install_sources::SourceRegistry;
use crate::monitor::MonitorHandle;
use crate::smoke::SmokeFixture;
use crate::watcher::WatcherHandle;

pub struct AppState {
    pub core: Arc<Core>,
    pub smoke: Option<SmokeState>,
    /// Set once the main window has been shown (by `app_ready` or the 3 s fallback).
    pub shown: AtomicBool,
    pub monitor: MonitorHandle,
    pub watcher: WatcherHandle,
    pub last_focus_rescan: Mutex<Option<Instant>>,
    /// Picked and dropped install sources (paths stay in Rust).
    pub install_sources: Mutex<SourceRegistry>,
    /// How this copy was installed, computed once at startup.
    pub install_kind: InstallKind,
    /// The updater plugin is registered (release config, not `--smoke`).
    pub updater_configured: bool,
    /// The last update `update_check` found; `update_install` installs only this one.
    pub update: Mutex<Option<tauri_plugin_updater::Update>>,
    /// One `update_install` at a time.
    pub updating: AtomicBool,
    /// Started on `RunEvent::Exit` (after an AppImage update or a safe-graphics restart).
    pub relaunch: Mutex<Option<Relaunch>>,
}

/// A program the exit path starts through `platform::host_command`.
#[derive(Clone, Debug)]
pub struct Relaunch {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    /// This session runs from `--appimage-extract-and-run` (FUSE may be missing): the relaunched
    /// AppImage gets `APPIMAGE_EXTRACT_AND_RUN=1` so its runtime extracts instead of mounting.
    pub extract_and_run: bool,
}

/// What `on_setup` learned about the install and the updater.
#[derive(Clone, Copy, Debug)]
pub struct UpdateSetup {
    pub install_kind: InstallKind,
    pub updater_configured: bool,
}

pub struct SmokeState {
    /// Temp folder holding config, data, logs and webview data of this `--smoke` run.
    pub root: PathBuf,
    /// Expectations of the `--paks` fixture, checked after the frontend's report.
    pub fixture: Option<SmokeFixture>,
    outcome: AtomicU8,
    blocked_navigations: AtomicU32,
}

const RUNNING: u8 = 0;
const PASSED: u8 = 1;
const FAILED: u8 = 2;

impl AppState {
    pub fn new(
        core: Arc<Core>,
        monitor: MonitorHandle,
        watcher: WatcherHandle,
        smoke: Option<SmokeState>,
        update: UpdateSetup,
    ) -> Self {
        Self {
            core,
            smoke,
            shown: AtomicBool::new(false),
            monitor,
            watcher,
            last_focus_rescan: Mutex::new(None),
            install_sources: Mutex::new(SourceRegistry::default()),
            install_kind: update.install_kind,
            updater_configured: update.updater_configured,
            update: Mutex::new(None),
            updating: AtomicBool::new(false),
            relaunch: Mutex::new(None),
        }
    }

    /// The updater is rewriting the app (gate sealed, no relaunch stored yet). On Linux the
    /// plugin has moved `$APPIMAGE` aside and writes the new file; exiting now would leave it
    /// truncated, so a user close waits. A failed install unseals, a finished one stores the
    /// relaunch first, and either ends this.
    pub fn update_install_running(&self) -> bool {
        self.core.is_sealed_for_update()
            && self
                .relaunch
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .is_none()
    }
}

impl SmokeState {
    pub fn new(root: PathBuf, fixture: Option<SmokeFixture>) -> Self {
        Self {
            root,
            fixture,
            outcome: AtomicU8::new(RUNNING),
            blocked_navigations: AtomicU32::new(0),
        }
    }

    pub fn webview_data_dir(&self) -> PathBuf {
        self.root.join("webview")
    }

    /// Records the outcome; only the first caller (report, watchdog or exit) wins.
    pub fn finish(&self, passed: bool) -> bool {
        let outcome = if passed { PASSED } else { FAILED };
        self.outcome
            .compare_exchange(RUNNING, outcome, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    pub fn passed(&self) -> bool {
        self.outcome.load(Ordering::SeqCst) == PASSED
    }

    pub fn record_blocked_navigation(&self) {
        self.blocked_navigations.fetch_add(1, Ordering::SeqCst);
    }

    pub fn blocked_navigations(&self) -> u32 {
        self.blocked_navigations.load(Ordering::SeqCst)
    }
}
