use tauri_specta::{Builder, ErrorHandlingMode, collect_commands, collect_events};

use crate::commands::{game, mods, settings, shell};
use crate::events::{GameStatusChanged, ModsChanged, WatchStatusChanged};

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
            game::game_status,
            game::game_detect,
            game::game_set_location,
            game::game_pick_location,
            game::game_ack_update,
            game::game_launch,
            game::game_reveal,
            mods::mods_list,
            mods::mods_refresh,
            mods::mods_details,
            mods::mods_reveal,
            mods::mods_toggle,
            mods::mods_disable_all,
            mods::mods_watch_status,
        ])
        .events(collect_events![
            ModsChanged,
            GameStatusChanged,
            WatchStatusChanged
        ])
        .error_handling(ErrorHandlingMode::Result)
        .disable_serde_phases()
}
