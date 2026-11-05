//! Backend-initiated events. They are not buffered: the UI subscribes first, then fetches.

use std::sync::Arc;

use remmods_core::game::GameStatusChange;
use remmods_core::safe_fs::{BusyEvent, BusySink};
use remmods_ipc::{
    BusyPhase, DropPhase, FolderBusyInfo, PickedSource, RejectedSource, RunningState, UpdateNotice,
    WatchState,
};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_specta::Event;

/// `mods-changed`: a rescan published a new snapshot revision.
#[derive(Serialize, Deserialize, Clone, Debug, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct ModsChanged {
    pub revision: u32,
}

/// `assets-changed`: stored item pictures or names changed.
#[derive(Serialize, Deserialize, Clone, Debug, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct AssetsChanged {}

/// `game-status-changed`: the running state, session, other-manager flag or update notice changed.
#[derive(Serialize, Deserialize, Clone, Debug, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct GameStatusChanged {
    pub running: RunningState,
    pub session: u32,
    pub other_manager: bool,
    pub update: Option<UpdateNotice>,
}

impl From<GameStatusChange> for GameStatusChanged {
    fn from(change: GameStatusChange) -> Self {
        Self {
            running: change.running.into(),
            session: change.session,
            other_manager: change.other_manager,
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

/// `install-drop`: files or folders are dragged over the window (`enter`/`leave`, `count` items)
/// or were dropped (`dropped`: the registered sources and the refused ones).
#[derive(Serialize, Deserialize, Clone, Debug, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct InstallDrop {
    pub phase: DropPhase,
    pub count: u32,
    pub sources: Vec<PickedSource>,
    pub rejected: Vec<RejectedSource>,
}

/// `folder-busy`: a folder rename waits for another program (WG-2): `waiting` about once a second,
/// then one of `done`, `timedOut`, `cancelled`, `failed`.
#[derive(Serialize, Deserialize, Clone, Debug, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct FolderBusy {
    pub wait_id: u32,
    pub phase: BusyPhase,
    pub folder: String,
    pub holders: Vec<String>,
    #[specta(type = specta_typescript::Number)]
    pub elapsed_ms: f64,
    #[specta(type = specta_typescript::Number)]
    pub limit_ms: f64,
    pub cancellable: bool,
}

impl From<FolderBusyInfo> for FolderBusy {
    fn from(info: FolderBusyInfo) -> Self {
        Self {
            wait_id: info.wait_id,
            phase: info.phase,
            folder: info.folder,
            holders: info.holders,
            elapsed_ms: info.elapsed_ms,
            limit_ms: info.limit_ms,
            cancellable: info.cancellable,
        }
    }
}

/// Core's folder-busy notices as `folder-busy` events. The closure holds only the app handle, so
/// the OpGate that owns the sink is never kept alive by it.
pub fn folder_busy_sink(app: AppHandle) -> BusySink {
    Arc::new(move |event: &BusyEvent| {
        emit(&app, &FolderBusy::from(FolderBusyInfo::from(event)));
    })
}

/// Emits to every window; a failure only means nobody listens, so it is logged, not returned.
pub fn emit<E: Event + Serialize + Clone>(app: &AppHandle, event: &E) {
    if let Err(error) = event.emit(app) {
        log::warn!("emitting {} failed: {error}", E::NAME);
    }
}
