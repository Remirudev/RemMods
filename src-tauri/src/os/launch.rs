use std::io;

use remmods_core::game::LaunchPlan;

/// Hands the `steam://` URL to the Steam client; RemMods never starts the game exe itself.
#[cfg(windows)]
pub fn launch(plan: &LaunchPlan) -> io::Result<()> {
    log::info!("launching through {}", plan.url);
    tauri_plugin_opener::open_url(&plan.url, None::<&str>).map_err(super::opener_io_error)
}

/// Runs the host command for the install's Steam (xdg-open, `flatpak run` or `snap run`): with a
/// Flatpak or Snap install, xdg-open would usually start the native Steam instead.
#[cfg(not(windows))]
pub fn launch(plan: &LaunchPlan) -> io::Result<()> {
    use remmods_core::platform;

    let argv = plan.host_argv();
    let Some((program, args)) = argv.split_first() else {
        return Err(io::Error::other("empty launch command"));
    };
    log::info!("launching with {argv:?}");
    let mut command = platform::host_command(program);
    command.args(args);
    platform::spawn_detached(command)
}
