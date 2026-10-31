use remmods_ipc::{AppError, Settings, SettingsUpdate};
use tauri::{AppHandle, State};

use super::run_blocking;
use crate::state::AppState;
use crate::window;

#[tauri::command]
#[specta::specta]
pub async fn settings_get(state: State<'_, AppState>) -> Result<Settings, AppError> {
    run_blocking(state.core.clone(), |core| Ok(core.settings().into())).await
}

#[tauri::command]
#[specta::specta]
pub async fn settings_update(
    app: AppHandle,
    state: State<'_, AppState>,
    update: SettingsUpdate,
) -> Result<Settings, AppError> {
    let update = update.into();
    let snapshot = run_blocking(state.core.clone(), move |core| {
        core.update_settings(&update)
    })
    .await?;
    window::apply_color_scheme(&app, snapshot.settings.appearance.scheme);
    Ok(snapshot.into())
}
