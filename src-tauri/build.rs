/// Every app command; tauri-build generates an `allow-<command>` permission for each.
const COMMANDS: &[&str] = &[
    "platform_info",
    "settings_get",
    "settings_update",
    "set_ui_scale",
    "app_ready",
    "open_logs",
    "smoke_report",
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
