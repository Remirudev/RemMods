use remmods_ipc::{AppError, ModDetails, ModsSnapshot};
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
