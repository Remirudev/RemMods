pub mod settings;
pub mod shell;

use std::fmt::Display;
use std::sync::Arc;

use remmods_core::{Core, CoreError};
use remmods_ipc::AppError;

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

/// Logs a shell-side failure (the details stay in the log) and returns the untranslated kind.
fn internal(what: &str, error: impl Display) -> AppError {
    log::error!("{what} failed: {error}");
    AppError::Internal
}
