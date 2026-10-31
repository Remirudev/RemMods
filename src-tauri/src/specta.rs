use tauri_specta::{Builder, ErrorHandlingMode, collect_commands};

use crate::commands::{settings, shell};

/// Single source of the IPC surface: the invoke handler and the TypeScript bindings both come
/// from here. Keep the list in sync with `COMMANDS` in `build.rs` (ACL permissions).
///
/// Serde phases are off: every DTO has the same shape in both directions, and phase mode would
/// split each one with an `Option` field into `_Serialize` / `_Deserialize` unions.
pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            shell::platform_info,
            settings::settings_get,
            settings::settings_update,
            shell::set_ui_scale,
            shell::app_ready,
            shell::open_logs,
            shell::smoke_report,
        ])
        .error_handling(ErrorHandlingMode::Result)
        .disable_serde_phases()
}
