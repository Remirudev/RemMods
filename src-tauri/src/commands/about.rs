use remmods_ipc::{AppError, Notices};

/// Embedded at compile time, so the About dialog needs no file access in any package format.
const RUST_NOTICES: &str = include_str!("../../../THIRD-PARTY-NOTICES");
const NPM_NOTICES: &str = include_str!("../../../THIRD-PARTY-NOTICES-NPM");

#[tauri::command]
#[specta::specta]
pub async fn about_notices() -> Result<Notices, AppError> {
    Ok(Notices {
        rust: RUST_NOTICES.to_owned(),
        npm: NPM_NOTICES.to_owned(),
    })
}
