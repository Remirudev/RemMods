use remmods_ipc::{AppError, RepairList};
use tauri::State;

use super::run_blocking;
use crate::state::AppState;

/// Leftovers and rows that need attention, each with the actions the UI offers. Read-only.
#[tauri::command]
#[specta::specta]
pub async fn repair_list(state: State<'_, AppState>) -> Result<RepairList, AppError> {
    run_blocking(state.core.clone(), |core| {
        core.repair_list().map(|list| RepairList::from(&list))
    })
    .await
}
