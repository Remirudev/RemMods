use remmods_core::mods as core_mods;
use remmods_ipc::{
    AppError, ModContents, ModDetails, ModsSnapshot, ToggleOutcome, ToggleRequest, WatchStatus,
};
use tauri::{AppHandle, State};

use super::{reveal_in_file_manager, run_blocking, run_mutation};
use crate::base_index;
use crate::events::{self, ModsChanged};
use crate::state::AppState;

/// The cached snapshot when it belongs to the current Paks folder, else a fresh scan.
#[tauri::command]
#[specta::specta]
pub async fn mods_list(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ModsSnapshot, AppError> {
    let snapshot = run_blocking(state.core.clone(), |core| {
        core.mods_list()
            .map(|snapshot| ModsSnapshot::from(&*snapshot))
    })
    .await?;
    base_index::check_soon(&app, &state.core);
    Ok(snapshot)
}

/// Rescans Paks; emits `mods-changed` when the content differs from the last snapshot.
#[tauri::command]
#[specta::specta]
pub async fn mods_refresh(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ModsSnapshot, AppError> {
    let (snapshot, changed) = run_blocking(state.core.clone(), |core| {
        core.mods_refresh()
            .map(|refreshed| (ModsSnapshot::from(&*refreshed.snapshot), refreshed.changed))
    })
    .await?;
    base_index::check_soon(&app, &state.core);
    if changed {
        events::emit(
            &app,
            &ModsChanged {
                revision: snapshot.revision,
            },
        );
    }
    Ok(snapshot)
}

#[tauri::command]
#[specta::specta]
pub async fn mods_details(
    state: State<'_, AppState>,
    mod_ref: String,
) -> Result<ModDetails, AppError> {
    run_blocking(state.core.clone(), move |core| {
        core.mods_details(&mod_ref)
            .map(|details| ModDetails::from(&details))
    })
    .await
}

/// The asset list of one row's `.utoc` containers; `base` is `loading` until the index is built.
#[tauri::command]
#[specta::specta]
pub async fn mods_contents(
    state: State<'_, AppState>,
    mod_ref: String,
) -> Result<ModContents, AppError> {
    run_blocking(state.core.clone(), move |core| {
        core.mods_contents(&mod_ref)
            .map(|contents| ModContents::from(&contents))
    })
    .await
}

/// Shows a mod, archive or unmanaged entry selected in the file manager.
#[tauri::command]
#[specta::specta]
pub async fn mods_reveal(state: State<'_, AppState>, mod_ref: String) -> Result<(), AppError> {
    let target = run_blocking(state.core.clone(), move |core| {
        core.mods_reveal_target(&mod_ref)
    })
    .await?;
    reveal_in_file_manager(target).await
}

/// Enables or disables the given rows in one all-or-none rename batch.
#[tauri::command]
#[specta::specta]
pub async fn mods_toggle(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ToggleRequest,
) -> Result<ToggleOutcome, AppError> {
    let request = core_mods::ToggleRequest::from(&request);
    run_mutation(&app, &state.core, move |core| {
        core.mods_toggle(&request)
            .map(|outcome| ToggleOutcome::from(&outcome))
    })
    .await
}

/// Disables every enabled, toggleable row; blocked rows come back as `skipped`.
#[tauri::command]
#[specta::specta]
pub async fn mods_disable_all(
    app: AppHandle,
    state: State<'_, AppState>,
    apply_while_running: bool,
) -> Result<ToggleOutcome, AppError> {
    run_mutation(&app, &state.core, move |core| {
        core.mods_disable_all(apply_while_running)
            .map(|outcome| ToggleOutcome::from(&outcome))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn mods_watch_status(state: State<'_, AppState>) -> Result<WatchStatus, AppError> {
    Ok(WatchStatus {
        state: state.watcher.status(),
    })
}
