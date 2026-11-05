use remmods_core::install::{IMAGE_EXTENSIONS, SourceKind};
use remmods_core::mods as core_mods;
use remmods_ipc::{AppError, AssetsStatus, ModItems, ModTarget, PickedSource, SourceReject};
use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use super::install::{classify_picked, registry};
use super::{internal, run_blocking, run_mutation};
use crate::base_index;
use crate::install_sources::Classified;
use crate::state::AppState;

const PICTURE_FILTER: &str = "Images";

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

/// Opens the system file picker for one image; `None` when cancelled.
#[tauri::command]
#[specta::specta]
pub async fn pick_picture(
    window: WebviewWindow,
    state: State<'_, AppState>,
    title: String,
) -> Result<Option<PickedSource>, AppError> {
    // The blocking picker must never run on the main thread.
    let classified = tauri::async_runtime::spawn_blocking(move || {
        let picked = window
            .dialog()
            .file()
            .set_title(title)
            .set_parent(&window)
            .add_filter(PICTURE_FILTER, &IMAGE_EXTENSIONS)
            .blocking_pick_file();
        classify_picked(&window, picked.into_iter().collect())
    })
    .await
    .map_err(|error| internal("picture picker task", error))?;
    let Some(Classified { path, result }) = classified.into_iter().next() else {
        return Ok(None);
    };
    match result {
        Ok(SourceKind::Image) => Ok(Some(registry(&state)?.register(path, SourceKind::Image))),
        Ok(_) => Err(AppError::SourceRejected {
            reason: SourceReject::UnsupportedType,
        }),
        Err(reason) => Err(AppError::SourceRejected { reason }),
    }
}

/// Makes a picked or dropped image the mod's own picture. Its path stays in Rust.
#[tauri::command]
#[specta::specta]
pub async fn mods_set_picture(
    app: AppHandle,
    state: State<'_, AppState>,
    target: ModTarget,
    source: String,
) -> Result<(), AppError> {
    let path = registry(&state)?.resolve(&source).ok_or(AppError::Stale)?;
    let target = core_mods::ModTarget::from(&target);
    run_mutation(&app, &state.core, move |core| {
        core.mods_set_picture(&target, &path)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn mods_clear_picture(
    app: AppHandle,
    state: State<'_, AppState>,
    target: ModTarget,
) -> Result<(), AppError> {
    let target = core_mods::ModTarget::from(&target);
    run_mutation(&app, &state.core, move |core| {
        core.mods_clear_picture(&target)
    })
    .await
}
