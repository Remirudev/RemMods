//! Backend-initiated events. They are not buffered: the UI subscribes first, then fetches.

use remmods_core::game::GameStatusChange;
use remmods_ipc::{RunningState, UpdateNotice, WatchState};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_specta::Event;

/// `mods-changed`: a rescan published a new snapshot revision.
#[derive(Serialize, Deserialize, Clone, Debug, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct ModsChanged {
    pub revision: u32,
}

/// `game-status-changed`: the running state, session, original-app flag or update notice changed.
#[derive(Serialize, Deserialize, Clone, Debug, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct GameStatusChanged {
    pub running: RunningState,
    pub session: u32,
    pub original_app: bool,
    pub update: Option<UpdateNotice>,
}

impl From<GameStatusChange> for GameStatusChanged {
    fn from(change: GameStatusChange) -> Self {
        Self {
            running: change.running.into(),
            session: change.session,
            original_app: change.original_app,
            update: change.update.as_ref().map(UpdateNotice::from),
        }
    }
}

/// `watch-status-changed`: the Paks watcher became active, hit the watch limit, has no folder or
/// failed to start.
#[derive(Serialize, Deserialize, Clone, Debug, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct WatchStatusChanged {
    pub state: WatchState,
}

/// Emits to every window; a failure only means nobody listens, so it is logged, not returned.
pub fn emit<E: Event + Serialize + Clone>(app: &AppHandle, event: &E) {
    if let Err(error) = event.emit(app) {
        log::warn!("emitting {} failed: {error}", E::NAME);
    }
}
