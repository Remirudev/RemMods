//! Focus rescan: P1's only detection of changes made outside RemMods (no watcher until P2).

use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use remmods_core::game::GameError;
use remmods_core::{Core, CoreError};
use tauri::{AppHandle, Manager};

use crate::base_index;
use crate::events::{self, GameStatusChanged, ModsChanged};
use crate::state::AppState;

/// Alt-tabbing back and forth must not queue one scan per focus change.
const MIN_GAP: Duration = Duration::from_secs(2);

/// Runs on the main thread (window event); the scan itself goes to the blocking pool.
pub fn on_focus_gained(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    if !claim_rescan(&state.last_focus_rescan, Instant::now()) {
        return;
    }
    let core = state.core.clone();
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || rescan(&app, &core));
}

/// Records `now` as the last rescan unless one ran less than [`MIN_GAP`] ago.
fn claim_rescan(last: &Mutex<Option<Instant>>, now: Instant) -> bool {
    let mut last = last.lock().unwrap_or_else(PoisonError::into_inner);
    if last.is_some_and(|at| now.saturating_duration_since(at) < MIN_GAP) {
        return false;
    }
    *last = Some(now);
    true
}

fn rescan(app: &AppHandle, core: &Arc<Core>) {
    base_index::check_soon(app, core);
    match core.mods_refresh() {
        Ok(refreshed) if refreshed.changed => {
            let revision = refreshed.snapshot.revision;
            events::emit(app, &ModsChanged { revision });
        }
        Ok(_) | Err(CoreError::Game(GameError::NotSet)) => {}
        Err(error) => log::warn!("focus rescan failed: {error}"),
    }
    if let Some(change) = core.game_refresh_build() {
        events::emit(app, &GameStatusChanged::from(change));
    }
}
