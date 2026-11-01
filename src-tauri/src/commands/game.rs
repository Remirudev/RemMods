use std::path::{Path, PathBuf};

use remmods_core::game as core_game;
use remmods_core::{Core, platform};
use remmods_ipc::{AppError, GameStatus, LocationChoice, PickOutcome};
use tauri::{Manager, State, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, FilePath};

use super::{internal, reveal_in_file_manager, run_blocking};
use crate::os;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn game_status(state: State<'_, AppState>) -> Result<GameStatus, AppError> {
    run_blocking(state.core.clone(), |core| {
        Ok(GameStatus::from(&core.game_status()))
    })
    .await
}

/// Searches the Steam libraries again; never switches away from a chosen folder.
#[tauri::command]
#[specta::specta]
pub async fn game_detect(state: State<'_, AppState>) -> Result<GameStatus, AppError> {
    run_blocking(state.core.clone(), |core| {
        Ok(GameStatus::from(&core.game_detect()))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn game_set_location(
    state: State<'_, AppState>,
    choice: LocationChoice,
) -> Result<GameStatus, AppError> {
    let choice = choice.into();
    run_blocking(state.core.clone(), move |core| {
        core.game_set_location(choice)
            .map(|status| GameStatus::from(&status))
    })
    .await
}

/// Opens the system folder picker attached to the window and validates the pick. `cancelled`
/// also stands for "no picker available"; the UI then offers the typed path.
#[tauri::command]
#[specta::specta]
pub async fn game_pick_location(
    window: WebviewWindow,
    state: State<'_, AppState>,
    title: String,
) -> Result<PickOutcome, AppError> {
    let core = state.core.clone();
    // The blocking picker must never run on the main thread.
    let picked = tauri::async_runtime::spawn_blocking(move || pick_folder(&window, &core, title))
        .await
        .map_err(|error| internal("folder picker task", error))?;
    let Some(picked) = picked else {
        return Ok(PickOutcome::Cancelled);
    };
    let path = picked
        .into_path()
        .map_err(|error| internal("reading the picked folder", error))?;
    run_blocking(state.core.clone(), move |core| {
        core.game_choose_path(&path)
            .map(|result| PickOutcome::from(&result))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn game_ack_update(state: State<'_, AppState>) -> Result<GameStatus, AppError> {
    run_blocking(state.core.clone(), |core| {
        core.game_ack_update()
            .map(|status| GameStatus::from(&status))
    })
    .await
}

/// Starts the game through Steam; the running state then polls every second for a minute.
#[tauri::command]
#[specta::specta]
pub async fn game_launch(state: State<'_, AppState>) -> Result<(), AppError> {
    let plan = run_blocking(state.core.clone(), |core| Ok(core.game_launch_plan())).await?;
    let launched = tauri::async_runtime::spawn_blocking(move || os::launch::launch(&plan))
        .await
        .map_err(|error| internal("launch task", error))?;
    match launched {
        Ok(()) => {
            state.monitor.boost();
            Ok(())
        }
        Err(error) => {
            log::warn!("launching the game failed: {error}");
            Err(AppError::LaunchFailed)
        }
    }
}

/// Shows the Paks folder in the file manager.
#[tauri::command]
#[specta::specta]
pub async fn game_reveal(state: State<'_, AppState>) -> Result<(), AppError> {
    let target = run_blocking(state.core.clone(), Core::game_reveal_target).await?;
    reveal_in_file_manager(target).await
}

fn pick_folder(window: &WebviewWindow, core: &Core, title: String) -> Option<FilePath> {
    let mut dialog = window.dialog().file().set_title(title).set_parent(window);
    if let Some(start) = picker_start_dir(window, core) {
        dialog = dialog.set_directory(start);
    }
    dialog.blocking_pick_folder()
}

/// The current folder; for a missing one its nearest existing parent; else the home folder.
fn picker_start_dir(window: &WebviewWindow, core: &Core) -> Option<PathBuf> {
    let home = || window.path().home_dir().ok();
    match core.game_status().location {
        core_game::GameLocation::Ready(install) => Some(install.root),
        core_game::GameLocation::Missing { path } => existing_ancestor(&path).or_else(home),
        core_game::GameLocation::NotSet => home(),
    }
}

fn existing_ancestor(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|dir| platform::fs_path(dir).is_ok_and(|dir| dir.is_dir()))
        .map(Path::to_path_buf)
}
