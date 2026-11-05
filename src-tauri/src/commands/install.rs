use std::path::PathBuf;

use remmods_core::install as core_install;
use remmods_core::install::{InspectOptions, InstallSource, SourceKind};
use remmods_ipc::{
    AppError, InspectRequest, Inspection, InstallOutcome, InstallProgress, InstallSourceRef,
    PickedSource, StartRequest,
};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, FilePath};

use super::{internal, run_blocking, run_mutation};
use crate::install_sources::{Classified, SourceRegistry, classify_all, register_all};
use crate::state::AppState;

const ARCHIVE_FILTER: &str = "Archives";
const ARCHIVE_EXTENSIONS: &[&str] = &["zip", "7z", "rar"];

/// Opens the system file picker for archives and registers every usable pick; refused ones are
/// only logged. Empty when the dialog was cancelled.
#[tauri::command]
#[specta::specta]
pub async fn pick_archives(
    window: WebviewWindow,
    state: State<'_, AppState>,
    title: String,
) -> Result<Vec<PickedSource>, AppError> {
    // The blocking picker must never run on the main thread.
    let classified = tauri::async_runtime::spawn_blocking(move || {
        let picked = window
            .dialog()
            .file()
            .set_title(title)
            .set_parent(&window)
            .add_filter(ARCHIVE_FILTER, ARCHIVE_EXTENSIONS)
            .blocking_pick_files()
            .unwrap_or_default();
        classify_picked(&window, picked)
    })
    .await
    .map_err(|error| internal("archive picker task", error))?;
    let (sources, rejected) = register_all(&mut *registry(&state)?, classified);
    for refused in &rejected {
        log::info!("picked archive refused ({:?})", refused.reason);
    }
    Ok(sources)
}

/// Opens the system folder picker; `None` when cancelled. A folder core refuses (e.g. one inside
/// Paks) is an error, so the UI can say why.
#[tauri::command]
#[specta::specta]
pub async fn pick_folder(
    window: WebviewWindow,
    state: State<'_, AppState>,
    title: String,
) -> Result<Option<PickedSource>, AppError> {
    let classified = tauri::async_runtime::spawn_blocking(move || {
        let picked = window
            .dialog()
            .file()
            .set_title(title)
            .set_parent(&window)
            .blocking_pick_folder();
        classify_picked(&window, picked.into_iter().collect())
    })
    .await
    .map_err(|error| internal("folder picker task", error))?;
    let Some(Classified { path, result }) = classified.into_iter().next() else {
        return Ok(None);
    };
    match result {
        Ok(SourceKind::Folder) => Ok(Some(registry(&state)?.register(path, SourceKind::Folder))),
        Ok(_) => Err(AppError::SourceRejected {
            reason: remmods_ipc::SourceReject::UnsupportedType,
        }),
        Err(reason) => Err(AppError::SourceRejected { reason }),
    }
}

/// Lists the source and plans the install; writes nothing.
#[tauri::command]
#[specta::specta]
pub async fn install_inspect(
    state: State<'_, AppState>,
    request: InspectRequest,
) -> Result<Inspection, AppError> {
    let source = match &request.source {
        InstallSourceRef::Token { id } => {
            InstallSource::Path(registry(&state)?.resolve(id).ok_or(AppError::Stale)?)
        }
        InstallSourceRef::Archive { row } => InstallSource::ArchiveRow {
            mod_ref: row.clone(),
        },
    };
    let options = InspectOptions::from(&request);
    run_blocking(state.core.clone(), move |core| {
        core.install_inspect(&source, &options)
            .map(|inspection| Inspection::from(&inspection))
    })
    .await
}

/// Installs the picked candidates of an inspection. Progress goes over `progress`; a closed
/// window only drops the messages.
#[tauri::command]
#[specta::specta]
pub async fn install_start(
    app: AppHandle,
    state: State<'_, AppState>,
    request: StartRequest,
    progress: Channel<InstallProgress>,
) -> Result<InstallOutcome, AppError> {
    let request = core_install::StartRequest::from(&request);
    run_mutation(&app, &state.core, move |core| {
        let report = move |update: &core_install::InstallProgress| {
            let _ = progress.send(InstallProgress::from(update));
        };
        core.install_start(&request, &report)
            .map(|outcome| InstallOutcome::from(&outcome))
    })
    .await
}

/// Cancels the running install; false when none runs. Never waits for the write lock.
#[tauri::command]
#[specta::specta]
pub async fn install_cancel(state: State<'_, AppState>) -> Result<bool, AppError> {
    Ok(state.core.install_cancel())
}

pub(super) fn classify_picked(window: &WebviewWindow, picked: Vec<FilePath>) -> Vec<Classified> {
    let paths: Vec<PathBuf> = picked
        .into_iter()
        .filter_map(|file| {
            file.into_path()
                .inspect_err(|error| log::warn!("reading a picked path failed: {error}"))
                .ok()
        })
        .collect();
    if paths.is_empty() {
        return Vec::new();
    }
    let state = window.state::<AppState>();
    classify_all(&state.core, paths)
}

pub(super) fn registry<'a>(
    state: &'a State<'_, AppState>,
) -> Result<std::sync::MutexGuard<'a, SourceRegistry>, AppError> {
    state
        .install_sources
        .lock()
        .map_err(|error| internal("install source registry", error))
}
