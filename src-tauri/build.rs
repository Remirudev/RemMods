/// Every app command; tauri-build generates an `allow-<command>` permission for each.
const COMMANDS: &[&str] = &[
    "platform_info",
    "settings_get",
    "settings_update",
    "set_ui_scale",
    "app_ready",
    "open_logs",
    "smoke_report",
    "game_status",
    "game_detect",
    "game_set_location",
    "game_pick_location",
    "game_ack_update",
    "game_launch",
    "game_reveal",
    "mods_list",
    "mods_refresh",
    "mods_details",
    "mods_contents",
    "mods_items",
    "assets_status",
    "assets_refresh",
    "assets_picture",
    "pick_picture",
    "mods_set_picture",
    "mods_clear_picture",
    "mods_reveal",
    "mods_toggle",
    "mods_disable_all",
    "mods_watch_status",
    "mods_rename",
    "mods_trash",
    "mods_delete_prepare",
    "mods_delete_permanently",
    "mods_set_metadata",
    "mods_trash_many",
    "mods_delete_prepare_many",
    "mods_delete_many",
    "mods_busy_cancel",
    "repair_list",
    "profiles_list",
    "profiles_save",
    "profiles_delete",
    "profiles_diff",
    "profiles_apply",
    "profiles_transfer",
    "pick_archives",
    "pick_folder",
    "install_inspect",
    "install_start",
    "install_cancel",
    "update_channel",
    "update_check",
    "update_install",
    "graphics_get",
    "graphics_set_mode",
    "graphics_ack_step",
    "about_notices",
];

fn main() {
    if std::env::var_os("RUSTUP_TOOLCHAIN").is_none() {
        println!(
            "cargo::warning=RUSTUP_TOOLCHAIN is unset: build through rustup so rust-toolchain.toml applies"
        );
    }
    let manifest = tauri_build::AppManifest::new().commands(COMMANDS);
    let attributes = tauri_build::Attributes::new().app_manifest(manifest);
    if let Err(error) = tauri_build::try_build(attributes) {
        panic!("tauri-build failed: {error:#}");
    }
}
