use remmods_core::settings::ColorScheme;
use tauri::{Theme, WebviewWindow};

/// Window theme for the app's colour scheme; `System` leaves it to the OS.
pub fn window_theme(scheme: ColorScheme) -> Option<Theme> {
    match scheme {
        ColorScheme::Dark => Some(Theme::Dark),
        ColorScheme::Light => Some(Theme::Light),
        ColorScheme::System => None,
    }
}

/// WebView2 takes `prefers-color-scheme` from the window theme, so without this "System" would
/// stay on whatever theme the window was created with.
#[cfg(windows)]
pub fn sync_window_theme(window: &WebviewWindow, scheme: ColorScheme) {
    if let Err(error) = window.set_theme(window_theme(scheme)) {
        log::warn!("updating the window theme failed: {error}");
    }
}

/// WebKitGTK follows the desktop's colour scheme through the portal by itself, and tao's
/// `set_theme(None)` would force GTK's light variant instead of following it.
#[cfg(not(windows))]
pub fn sync_window_theme(_window: &WebviewWindow, _scheme: ColorScheme) {}
