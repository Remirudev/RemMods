use remmods_ipc::{AppError, AssetsStatus, ModItems};
use tauri::{AppHandle, State};

use super::run_blocking;
use crate::base_index;
use crate::state::AppState;

/// What the assets folder holds; `names` counts the texts of `language`.
#[tauri::command]
#[specta::specta]
pub async fn assets_status(
    state: State<'_, AppState>,
    language: String,
) -> Result<AssetsStatus, AppError> {
    run_blocking(state.core.clone(), move |core| {
        Ok(AssetsStatus::from(core.assets_status(&language)))
    })
    .await
}

/// The check the app runs after every scan, now; waits for a running one.
#[tauri::command]
#[specta::specta]
pub async fn assets_refresh(
    app: AppHandle,
    state: State<'_, AppState>,
    language: String,
) -> Result<AssetsStatus, AppError> {
    run_blocking(state.core.clone(), move |core| {
        base_index::check(&app, core);
        Ok(AssetsStatus::from(core.assets_status(&language)))
    })
    .await
}

/// One stored picture as a `data:` URL; `null` when it is gone.
#[tauri::command]
#[specta::specta]
pub async fn assets_picture(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<String>, AppError> {
    run_blocking(state.core.clone(), move |core| Ok(core.assets_picture(&id))).await
}

/// The items one row touches; pictures and names appear once the background check stored them.
#[tauri::command]
#[specta::specta]
pub async fn mods_items(
    state: State<'_, AppState>,
    mod_ref: String,
    language: String,
) -> Result<ModItems, AppError> {
    run_blocking(state.core.clone(), move |core| {
        core.mods_items(&mod_ref, &language)
            .map(|items| ModItems::from(&items))
    })
    .await
}
