use remmods_core::graphics::{self, GraphicsError, GraphicsPaths};
use remmods_core::{Core, CoreError};
use remmods_ipc::{AppError, GraphicsMode, GraphicsState, GraphicsStep};
use tauri::State;

use super::run_blocking;
use crate::os;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn graphics_get(state: State<'_, AppState>) -> Result<GraphicsState, AppError> {
    if !os::graphics::supported() {
        return Ok(GraphicsState::unsupported());
    }
    run_blocking(state.core.clone(), |core| Ok(current_state(core))).await
}

/// Stores the mode for the next start.
#[tauri::command]
#[specta::specta]
pub async fn graphics_set_mode(
    state: State<'_, AppState>,
    mode: GraphicsMode,
) -> Result<GraphicsState, AppError> {
    change(&state, move |paths| {
        graphics::set_mode(paths, mode.into()).map(drop)
    })
    .await
}

/// Keeps (`revert` false) or undoes (`revert` true) the automatic step after a crashed start.
#[tauri::command]
#[specta::specta]
pub async fn graphics_ack_step(
    state: State<'_, AppState>,
    revert: bool,
) -> Result<GraphicsState, AppError> {
    change(&state, move |paths| {
        graphics::ack_step(paths, revert).map(drop)
    })
    .await
}

async fn change<F>(state: &AppState, work: F) -> Result<GraphicsState, AppError>
where
    F: FnOnce(&GraphicsPaths) -> Result<(), GraphicsError> + Send + 'static,
{
    if !os::graphics::supported() {
        return Err(CoreError::Graphics(GraphicsError::Unsupported).into());
    }
    run_blocking(state.core.clone(), move |core| {
        work(&GraphicsPaths::from_app(&core.paths))?;
        Ok(current_state(core))
    })
    .await
}

fn current_state(core: &Core) -> GraphicsState {
    let loaded = graphics::load(&GraphicsPaths::from_app(&core.paths));
    let session = os::graphics::session();
    let stored = loaded.file.mode;
    GraphicsState {
        supported: true,
        mode: stored.into(),
        effective: session.map_or(stored, |session| session.effective).into(),
        safe_flag: session.is_some_and(|session| session.source == graphics::ModeSource::Flag),
        user_env: session.is_some_and(|session| session.user_controlled),
        step: loaded.file.stepped.as_ref().map(GraphicsStep::from),
        read_only: loaded.read_only,
    }
}
