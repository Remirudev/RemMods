use remmods_core::mods as core_mods;
use remmods_ipc::{
    AppError, DeleteOutcome, DeletePrepared, DeleteRequest, MetadataRequest, ModTarget,
    ModsSnapshot, RenameOutcome, RenameRequest, TrashOutcome, TrashRequest,
};
use tauri::{AppHandle, State};

use super::{run_blocking, run_mutation};
use crate::state::AppState;

/// Renames a mod folder (or loose group) and rekeys profiles and `mod-meta.json`.
#[tauri::command]
#[specta::specta]
pub async fn mods_rename(
    app: AppHandle,
    state: State<'_, AppState>,
    request: RenameRequest,
) -> Result<RenameOutcome, AppError> {
    let request = core_mods::RenameRequest::from(&request);
    run_mutation(&app, &state.core, move |core| {
        core.mods_rename(&request)
            .map(|outcome| RenameOutcome::from(&outcome))
    })
    .await
}

/// Moves a mod to the Trash; `TrashUnavailable` lets the UI offer the permanent delete instead.
#[tauri::command]
#[specta::specta]
pub async fn mods_trash(
    app: AppHandle,
    state: State<'_, AppState>,
    request: TrashRequest,
) -> Result<TrashOutcome, AppError> {
    let request = core_mods::TrashRequest::from(&request);
    run_mutation(&app, &state.core, move |core| {
        core.mods_trash(&request)
            .map(|outcome| TrashOutcome::from(&outcome))
    })
    .await
}

/// First step of a permanent delete: checks the target and issues a single-use token. Writes
/// nothing.
#[tauri::command]
#[specta::specta]
pub async fn mods_delete_prepare(
    state: State<'_, AppState>,
    target: ModTarget,
) -> Result<DeletePrepared, AppError> {
    let target = core_mods::ModTarget::from(&target);
    run_blocking(state.core.clone(), move |core| {
        core.mods_delete_prepare(&target)
            .map(|prepared| DeletePrepared::from(&prepared))
    })
    .await
}

/// Second step, after the second confirmation: consumes the token and deletes.
#[tauri::command]
#[specta::specta]
pub async fn mods_delete_permanently(
    app: AppHandle,
    state: State<'_, AppState>,
    request: DeleteRequest,
) -> Result<DeleteOutcome, AppError> {
    let request = core_mods::DeleteRequest::from(&request);
    run_mutation(&app, &state.core, move |core| {
        core.mods_delete_permanently(&request)
            .map(|outcome| DeleteOutcome::from(&outcome))
    })
    .await
}

/// Writes the row's `modinfo.json` (the previous one is backed up first).
#[tauri::command]
#[specta::specta]
pub async fn mods_set_metadata(
    app: AppHandle,
    state: State<'_, AppState>,
    request: MetadataRequest,
) -> Result<ModsSnapshot, AppError> {
    let request = core_mods::MetadataRequest::from(&request);
    run_mutation(&app, &state.core, move |core| {
        core.mods_set_metadata(&request)
            .map(|refreshed| ModsSnapshot::from(&*refreshed.snapshot))
    })
    .await
}
