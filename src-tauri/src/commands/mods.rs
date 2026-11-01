use remmods_core::mods as core_mods;
use remmods_core::{Core, CoreError};
use remmods_ipc::{
    AppError, ModContents, ModDetails, ModsSnapshot, ToggleOutcome, ToggleRequest, WatchStatus,
};
use tauri::{AppHandle, State};

use super::{reveal_in_file_manager, run_blocking};
use crate::events::{self, ModsChanged};
use crate::state::AppState;

/// The cached snapshot when it belongs to the current Paks folder, else a fresh scan.
#[tauri::command]
#[specta::specta]
pub async fn mods_list(state: State<'_, AppState>) -> Result<ModsSnapshot, AppError> {
    run_blocking(state.core.clone(), |core| {
        core.mods_list()
            .map(|snapshot| ModsSnapshot::from(&*snapshot))
    })
    .await
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

/// The asset list of one row's `.utoc` containers; the base index is built on the first call.
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
    run_mutation(&app, &state, move |core| core.mods_toggle(&request)).await
}

/// Disables every enabled, toggleable row; blocked rows come back as `skipped`.
#[tauri::command]
#[specta::specta]
pub async fn mods_disable_all(
    app: AppHandle,
    state: State<'_, AppState>,
    apply_while_running: bool,
) -> Result<ToggleOutcome, AppError> {
    run_mutation(&app, &state, move |core| {
        core.mods_disable_all(apply_while_running)
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

/// Core publishes the final snapshot also when a batch fails, so `mods-changed` goes out on
/// success and on error whenever the revision moved.
async fn run_mutation<F>(
    app: &AppHandle,
    state: &AppState,
    work: F,
) -> Result<ToggleOutcome, AppError>
where
    F: FnOnce(&Core) -> Result<core_mods::ToggleOutcome, CoreError> + Send + 'static,
{
    let before = last_revision(&state.core);
    let result = run_blocking(state.core.clone(), move |core| {
        work(core).map(|outcome| ToggleOutcome::from(&outcome))
    })
    .await;
    let after = last_revision(&state.core);
    if let Some(revision) = after
        && after != before
    {
        events::emit(app, &ModsChanged { revision });
    }
    result
}

fn last_revision(core: &Core) -> Option<u32> {
    core.mods_last().map(|snapshot| snapshot.revision)
}
