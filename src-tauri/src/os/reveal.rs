use std::io;
use std::path::Path;

/// Selects `target` in Explorer. `target` is the shell form from core (no `\\?\` prefix).
#[cfg(windows)]
pub fn reveal(target: &Path) -> io::Result<()> {
    tauri_plugin_opener::reveal_item_in_dir(target).map_err(super::opener_io_error)
}

/// Selects `target` in the file manager. The opener's D-Bus portal fallback fails on desktops
/// without the FileManager1 service, so xdg-open on the parent folder is the last resort.
#[cfg(not(windows))]
pub fn reveal(target: &Path) -> io::Result<()> {
    use remmods_core::platform;

    match tauri_plugin_opener::reveal_item_in_dir(target) {
        Ok(()) => Ok(()),
        Err(error) => {
            log::warn!(
                "revealing {} failed ({error}); opening its folder with xdg-open",
                target.display()
            );
            let parent = target.parent().unwrap_or(target);
            let mut command = platform::host_command("xdg-open");
            command.arg(parent);
            platform::spawn_detached(command)
        }
    }
}

/// Opens `path` (a folder) in Explorer.
#[cfg(windows)]
pub fn open_folder(path: &Path) -> io::Result<()> {
    tauri_plugin_opener::open_path(path, None::<&str>).map_err(super::opener_io_error)
}

/// Opens `path` (a folder) in the file manager through xdg-open with the host environment: the
/// opener plugin would hand the AppImage's environment to it.
#[cfg(not(windows))]
pub fn open_folder(path: &Path) -> io::Result<()> {
    use remmods_core::platform;

    let mut command = platform::host_command("xdg-open");
    command.arg(path);
    platform::spawn_detached(command)
}
