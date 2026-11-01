//! Builds the base-game index off the scan's hot path (P2b): after a scan, one background task
//! checks it and, when it changed the content conflicts, announces the new revision.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use remmods_core::game::GameError;
use remmods_core::{Core, CoreError};
use tauri::AppHandle;

use crate::events::{self, ModsChanged};

static RUNNING: AtomicBool = AtomicBool::new(false);
static AGAIN: AtomicBool = AtomicBool::new(false);

/// Starts the check unless one is running; a request during a run makes it check once more.
pub fn check_soon(app: &AppHandle, core: &Arc<Core>) {
    AGAIN.store(true, Ordering::SeqCst);
    if RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    let (app, core) = (app.clone(), Arc::clone(core));
    tauri::async_runtime::spawn_blocking(move || {
        loop {
            while AGAIN.swap(false, Ordering::SeqCst) {
                check(&app, &core);
            }
            RUNNING.store(false, Ordering::SeqCst);
            // A request between the last swap and the store found RUNNING set and left.
            if !AGAIN.load(Ordering::SeqCst) || RUNNING.swap(true, Ordering::SeqCst) {
                break;
            }
        }
    });
}

fn check(app: &AppHandle, core: &Core) {
    match core.mods_base_refresh() {
        Ok(Some(refreshed)) => {
            let revision = refreshed.snapshot.revision;
            events::emit(app, &ModsChanged { revision });
        }
        Ok(None) | Err(CoreError::Game(GameError::NotSet)) => {}
        Err(error) => log::warn!("base index check failed: {error}"),
    }
}
