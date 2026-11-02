use remmods_core::mods as core_mods;
use remmods_ipc::{
    AppError, BulkDeletePrepared, BulkDeleteRequest, BulkOutcome, BulkTrashRequest, ModTarget,
};
use tauri::{AppHandle, State};

use super::{run_blocking, run_mutation};
use crate::state::AppState;

/// Moves several rows to the Trash in one call (one write guard, one rescan); per-row results.
#[tauri::command]
#[specta::specta]
pub async fn mods_trash_many(
    app: AppHandle,
    state: State<'_, AppState>,
    request: BulkTrashRequest,
) -> Result<BulkOutcome, AppError> {
    let request = core_mods::BulkTrashRequest::from(&request);
    run_mutation(&app, &state.core, move |core| {
        core.mods_trash_many(&request).map(BulkOutcome::from)
    })
    .await
}

/// First step of a bulk permanent delete: checks every row and issues one token for the eligible
/// ones. Writes nothing.
#[tauri::command]
#[specta::specta]
pub async fn mods_delete_prepare_many(
    state: State<'_, AppState>,
    targets: Vec<ModTarget>,
) -> Result<BulkDeletePrepared, AppError> {
    let targets: Vec<core_mods::ModTarget> =
        targets.iter().map(core_mods::ModTarget::from).collect();
    run_blocking(state.core.clone(), move |core| {
        core.mods_delete_prepare_many(&targets)
            .map(BulkDeletePrepared::from)
    })
    .await
}

/// Second step, after the second confirmation: consumes the token and deletes the rows it covers.
#[tauri::command]
#[specta::specta]
pub async fn mods_delete_many(
    app: AppHandle,
    state: State<'_, AppState>,
    request: BulkDeleteRequest,
) -> Result<BulkOutcome, AppError> {
    let request = core_mods::BulkDeleteRequest::from(&request);
    run_mutation(&app, &state.core, move |core| {
        core.mods_delete_many(&request).map(BulkOutcome::from)
    })
    .await
}

/// Cancels folder-busy wait `wait_id` (WG-2); false when it ended or cannot be cancelled. Never
/// waits for the write lock or the blocking pool, so it answers while a mutation is running.
#[tauri::command]
#[specta::specta]
pub async fn mods_busy_cancel(state: State<'_, AppState>, wait_id: u32) -> Result<bool, AppError> {
    let cancelled = state.core.ops.busy().cancel(wait_id);
    log::info!("folder-busy wait {wait_id}: cancel accepted {cancelled}");
    Ok(cancelled)
}
