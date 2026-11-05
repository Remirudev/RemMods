pub mod about;
pub mod assets;
pub mod game;
pub mod graphics;
pub mod install;
pub mod mods;
pub mod mods_bulk;
pub mod mods_manage;
pub mod profiles;
pub mod repair;
pub mod settings;
pub mod shell;
pub mod update;

use std::fmt::Display;
use std::path::PathBuf;
use std::sync::Arc;

use remmods_core::{Core, CoreError};
use remmods_ipc::{AppError, RevealProblem};
use tauri::AppHandle;

use crate::events::{self, ModsChanged};
use crate::os;

/// Runs core work on the blocking pool so the async runtime never waits on disk I/O.
async fn run_blocking<T, F>(core: Arc<Core>, work: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce(&Core) -> Result<T, CoreError> + Send + 'static,
{
    match tauri::async_runtime::spawn_blocking(move || work(&core)).await {
        Ok(result) => result.map_err(AppError::from),
        Err(error) => Err(internal("blocking task", error)),
    }
}

/// Core publishes the final snapshot also when a batch fails, so `mods-changed` goes out on
/// success and on error whenever the revision moved.
async fn run_mutation<T, F>(app: &AppHandle, core: &Arc<Core>, work: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce(&Core) -> Result<T, CoreError> + Send + 'static,
{
    let before = last_revision(core);
    let result = run_blocking(core.clone(), work).await;
    let after = last_revision(core);
    if let Some(revision) = after
        && after != before
    {
        events::emit(app, &ModsChanged { revision });
    }
    result
}

fn last_revision(core: &Core) -> Option<u32> {
    core.mods_last().map(|snapshot| snapshot.revision)
}

/// Logs a shell-side failure (the details stay in the log) and returns the untranslated kind.
fn internal(what: &str, error: impl Display) -> AppError {
    log::error!("{what} failed: {error}");
    AppError::Internal
}

/// Shows `target` (already in the shell form core hands out) selected in the file manager.
async fn reveal_in_file_manager(target: PathBuf) -> Result<(), AppError> {
    let revealed = tauri::async_runtime::spawn_blocking(move || os::reveal::reveal(&target))
        .await
        .map_err(|error| internal("reveal task", error))?;
    revealed.map_err(|error| {
        log::warn!("revealing an entry in the file manager failed: {error}");
        AppError::RevealFailed {
            reason: RevealProblem::Failed,
        }
    })
}
