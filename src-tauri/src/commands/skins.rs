use remmods_core::install::{IMAGE_EXTENSIONS, SourceKind};
use remmods_ipc::{AppError, PickedSource, SkinCopy, SkinsView};
use tauri::{State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use super::install::{classify_picked, registry};
use super::{internal, run_blocking};
use crate::install_sources::{Classified, register_all};
use crate::state::AppState;

const PICTURE_FILTER: &str = "Images";

/// The Steam account's skin sets; `available` is false when the game's files can't be found.
#[tauri::command]
#[specta::specta]
pub async fn skins_view(state: State<'_, AppState>) -> Result<SkinsView, AppError> {
    run_blocking(state.core.clone(), |core| {
        Ok(SkinsView::from(&core.skins_view()))
    })
    .await
}

/// Copies the game's five skin files into a new set, which then counts as the one in the game.
#[tauri::command]
#[specta::specta]
pub async fn skins_save_new(
    state: State<'_, AppState>,
    name: String,
) -> Result<SkinsView, AppError> {
    run_blocking(state.core.clone(), move |core| {
        core.skins_save_new(&name)
            .map(|view| SkinsView::from(&view))
    })
    .await
}

/// With the game closed: keeps the current skins in their set and puts this set in.
#[tauri::command]
#[specta::specta]
pub async fn skins_switch(state: State<'_, AppState>, id: String) -> Result<SkinsView, AppError> {
    run_blocking(state.core.clone(), move |core| {
        core.skins_switch(&id).map(|view| SkinsView::from(&view))
    })
    .await
}

/// Puts the stored copy of the set in the game back, for when the game's files went wrong.
#[tauri::command]
#[specta::specta]
pub async fn skins_put_back(state: State<'_, AppState>, id: String) -> Result<SkinsView, AppError> {
    run_blocking(state.core.clone(), move |core| {
        core.skins_put_back(&id).map(|view| SkinsView::from(&view))
    })
    .await
}

/// The automatic copies, newest first.
#[tauri::command]
#[specta::specta]
pub async fn skins_copies(state: State<'_, AppState>) -> Result<Vec<SkinCopy>, AppError> {
    run_blocking(state.core.clone(), |core| {
        Ok(core.skins_copies().iter().map(SkinCopy::from).collect())
    })
    .await
}

/// Makes a new set from an automatic copy.
#[tauri::command]
#[specta::specta]
pub async fn skins_keep_copy(
    state: State<'_, AppState>,
    copy: String,
    name: String,
) -> Result<SkinsView, AppError> {
    run_blocking(state.core.clone(), move |core| {
        core.skins_keep_copy(&copy, &name)
            .map(|view| SkinsView::from(&view))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn skins_edit(
    state: State<'_, AppState>,
    id: String,
    name: String,
    notes: String,
) -> Result<SkinsView, AppError> {
    run_blocking(state.core.clone(), move |core| {
        core.skins_edit(&id, &name, &notes)
            .map(|view| SkinsView::from(&view))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn skins_delete(state: State<'_, AppState>, id: String) -> Result<SkinsView, AppError> {
    run_blocking(state.core.clone(), move |core| {
        core.skins_delete(&id).map(|view| SkinsView::from(&view))
    })
    .await
}

/// Opens the file picker for screenshots, at Steam's folder for this game when there is one, and
/// registers every image picked. Empty when cancelled.
#[tauri::command]
#[specta::specta]
pub async fn pick_screenshots(
    window: WebviewWindow,
    state: State<'_, AppState>,
    title: String,
) -> Result<Vec<PickedSource>, AppError> {
    let core = state.core.clone();
    // The blocking picker must never run on the main thread.
    let classified = tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = window
            .dialog()
            .file()
            .set_title(title)
            .set_parent(&window)
            .add_filter(PICTURE_FILTER, &IMAGE_EXTENSIONS);
        if let Some(folder) = core.skins_screenshot_folder() {
            dialog = dialog.set_directory(folder);
        }
        let picked = dialog.blocking_pick_files().unwrap_or_default();
        classify_picked(&window, picked)
    })
    .await
    .map_err(|error| internal("screenshot picker task", error))?;
    let images: Vec<Classified> = classified
        .into_iter()
        .filter(|item| matches!(item.result, Ok(SourceKind::Image)))
        .collect();
    let (sources, _) = register_all(&mut *registry(&state)?, images);
    Ok(sources)
}

/// Adds a picked or dropped image to a set. Its path stays in Rust.
#[tauri::command]
#[specta::specta]
pub async fn skins_add_screenshot(
    state: State<'_, AppState>,
    id: String,
    source: String,
) -> Result<SkinsView, AppError> {
    let path = registry(&state)?.resolve(&source).ok_or(AppError::Stale)?;
    run_blocking(state.core.clone(), move |core| {
        core.skins_add_screenshot(&id, &path)
            .map(|view| SkinsView::from(&view))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn skins_remove_screenshot(
    state: State<'_, AppState>,
    id: String,
    shot: String,
) -> Result<SkinsView, AppError> {
    run_blocking(state.core.clone(), move |core| {
        core.skins_remove_screenshot(&id, &shot)
            .map(|view| SkinsView::from(&view))
    })
    .await
}
