use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};

use remmods_core::Core;

pub struct AppState {
    pub core: Arc<Core>,
    pub smoke: Option<SmokeState>,
    /// Set once the main window has been shown (by `app_ready` or the 3 s fallback).
    pub shown: AtomicBool,
}

pub struct SmokeState {
    /// Temp folder holding config, data, logs and webview data of this `--smoke` run.
    pub root: PathBuf,
    outcome: AtomicU8,
    blocked_navigations: AtomicU32,
}

const RUNNING: u8 = 0;
const PASSED: u8 = 1;
const FAILED: u8 = 2;

impl AppState {
    pub fn new(core: Core, smoke_root: Option<PathBuf>) -> Self {
        Self {
            core: Arc::new(core),
            smoke: smoke_root.map(SmokeState::new),
            shown: AtomicBool::new(false),
        }
    }
}

impl SmokeState {
    fn new(root: PathBuf) -> Self {
        Self {
            root,
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
