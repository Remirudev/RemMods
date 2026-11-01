use remmods_core::platform;
use remmods_core::settings::normalize_ui_scale;
use remmods_ipc::{AppError, AppPathsDto, PlatformInfo, SmokeReport};
use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_opener::OpenerExt;

use super::internal;
use crate::state::AppState;
use crate::{smoke, window};

#[tauri::command]
#[specta::specta]
pub async fn platform_info(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<PlatformInfo, AppError> {
    let webview_version = tauri::async_runtime::spawn_blocking(|| tauri::webview_version().ok())
        .await
        .map_err(|error| internal("webview version lookup", error))?;
    Ok(PlatformInfo {
        os: platform::os_kind().into(),
        arch: std::env::consts::ARCH.to_owned(),
        app_version: app.package_info().version.to_string(),
        webview_version,
        smoke: state.smoke.is_some(),
        paths: AppPathsDto::from(&state.core.paths),
    })
}

/// Applies the zoom only; persisting the scale is `settings_update`'s job.
#[tauri::command]
#[specta::specta]
pub async fn set_ui_scale(webview: WebviewWindow, scale: f64) -> Result<(), AppError> {
    let scale = normalize_ui_scale(scale).ok_or_else(|| AppError::invalid_input("scale"))?;
    webview
        .set_zoom(scale)
        .map_err(|error| internal("set_zoom", error))
}

#[tauri::command]
#[specta::specta]
pub async fn app_ready(app: AppHandle, state: State<'_, AppState>) -> Result<(), AppError> {
    window::show_main_window_once(&app, &state, "app_ready");
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn open_logs(app: AppHandle, state: State<'_, AppState>) -> Result<(), AppError> {
    let log_dir = state.core.paths.log_dir.to_string_lossy().into_owned();
    let opened =
        tauri::async_runtime::spawn_blocking(move || app.opener().open_path(log_dir, None::<&str>))
            .await
            .map_err(|error| internal("open_logs task", error))?;
    opened.map_err(|error| match error {
        tauri_plugin_opener::Error::Io(source) => {
            log::warn!("opening the log folder failed: {source}");
            AppError::Io {
                op: "open_logs".to_owned(),
                class: platform::classify(&source).into(),
            }
        }
        other => internal("open_logs", other),
    })
}

/// Final step of `--smoke`: logs and prints the report, then exits 0 (passed) or 1.
#[tauri::command]
#[specta::specta]
pub async fn smoke_report(
    app: AppHandle,
    state: State<'_, AppState>,
    report: SmokeReport,
) -> Result<(), AppError> {
    let Some(smoke_state) = state.smoke.as_ref() else {
        return Err(AppError::invalid_input("smoke"));
    };
    smoke::finish(&app, &state.core, smoke_state, report);
    Ok(())
}
