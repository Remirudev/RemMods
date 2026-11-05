use std::path::Path;

use remmods_core::profiles as core_profiles;
use remmods_core::{Core, CoreError};
use remmods_ipc::{
    AppError, ApplyDirection, ApplyOutcome, ApplyRequest, ExportReport, ImportReport, ProfileDiff,
    ProfileSave, ProfilesView, TransferOutcome, TransferRequest,
};
use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, FilePath};

use super::{internal, run_blocking, run_mutation};
use crate::state::AppState;

const JSON_FILTER: &str = "JSON";

#[tauri::command]
#[specta::specta]
pub async fn profiles_list(state: State<'_, AppState>) -> Result<ProfilesView, AppError> {
    run_blocking(state.core.clone(), |core| {
        core.profiles_list().map(|view| ProfilesView::from(&view))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn profiles_save(
    state: State<'_, AppState>,
    save: ProfileSave,
) -> Result<ProfilesView, AppError> {
    let save = core_profiles::ProfileSave::from(&save);
    run_blocking(state.core.clone(), move |core| {
        core.profiles_save(&save)
            .map(|view| ProfilesView::from(&view))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn profiles_delete(
    state: State<'_, AppState>,
    id: String,
) -> Result<ProfilesView, AppError> {
    run_blocking(state.core.clone(), move |core| {
        core.profiles_delete(&id)
            .map(|view| ProfilesView::from(&view))
    })
    .await
}

/// What switching the profile on or off would change; its `fingerprint` goes back in
/// `profiles_apply` with the same direction.
#[tauri::command]
#[specta::specta]
pub async fn profiles_diff(
    state: State<'_, AppState>,
    id: String,
    direction: ApplyDirection,
) -> Result<ProfileDiff, AppError> {
    let direction = direction.into();
    run_blocking(state.core.clone(), move |core| {
        core.profiles_diff(&id, direction)
            .map(|diff| ProfileDiff::from(&diff))
    })
    .await
}

/// Applies the shown diff as one rename batch after an automatic snapshot profile.
#[tauri::command]
#[specta::specta]
pub async fn profiles_apply(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ApplyRequest,
) -> Result<ApplyOutcome, AppError> {
    let request = core_profiles::ApplyRequest::from(&request);
    run_mutation(&app, &state.core, move |core| {
        core.profiles_apply(&request)
            .map(|outcome| ApplyOutcome::from(&outcome))
    })
    .await
}

/// Imports or exports through a native file dialog; `None` when the dialog was cancelled. The
/// picked path stays in Rust (only the export's file name comes back, for display).
#[tauri::command]
#[specta::specta]
pub async fn profiles_transfer(
    window: WebviewWindow,
    state: State<'_, AppState>,
    request: TransferRequest,
) -> Result<Option<TransferOutcome>, AppError> {
    let dialog_request = request.clone();
    // The blocking dialog must never run on the main thread.
    let picked = tauri::async_runtime::spawn_blocking(move || pick(&window, dialog_request))
        .await
        .map_err(|error| internal("profile file dialog task", error))?;
    let Some(picked) = picked else {
        return Ok(None);
    };
    let file = picked
        .into_path()
        .map_err(|error| internal("reading the picked profile file", error))?;
    run_blocking(state.core.clone(), move |core| {
        transfer(core, &request, &file).map(Some)
    })
    .await
}

fn pick(window: &WebviewWindow, request: TransferRequest) -> Option<FilePath> {
    let dialog = window.dialog().file().set_parent(window);
    match request {
        TransferRequest::ImportOtherManager { title } | TransferRequest::Import { title } => dialog
            .set_title(title)
            .add_filter(JSON_FILTER, &["json"])
            .blocking_pick_file(),
        TransferRequest::Export {
            title,
            default_file_name,
        } => dialog
            .set_title(title)
            .add_filter(JSON_FILTER, &["json"])
            .set_file_name(default_file_name)
            .blocking_save_file(),
    }
}

fn transfer(
    core: &Core,
    request: &TransferRequest,
    file: &Path,
) -> Result<TransferOutcome, CoreError> {
    match request {
        TransferRequest::ImportOtherManager { .. } => {
            core.profiles_import_other_manager(file).map(imported)
        }
        TransferRequest::Import { .. } => core.profiles_import(file).map(imported),
        TransferRequest::Export { .. } => {
            core.profiles_export(file)
                .map(|report| TransferOutcome::Exported {
                    report: ExportReport::from(&report),
                })
        }
    }
}

fn imported(report: core_profiles::ImportReport) -> TransferOutcome {
    TransferOutcome::Imported {
        report: ImportReport::from(&report),
    }
}
