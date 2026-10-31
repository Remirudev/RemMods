use remmods_core::navigation::is_navigation_allowed;
use remmods_core::platform;
use tauri::{AppHandle, Url};

/// The dev server origin, only in dev builds (`tauri dev`, plain `cargo build`).
pub fn dev_origin(app: &AppHandle) -> Option<String> {
    if cfg!(dev) {
        app.config().build.dev_url.as_ref().map(Url::to_string)
    } else {
        None
    }
}

/// Uses the runtime origin of this OS; Tauri's `scheme == "tauri"` doc example would blank
/// every Windows release window, which loads from `http://tauri.localhost`.
pub fn is_allowed(url: &Url, dev_origin: Option<&str>) -> bool {
    let allowed = is_navigation_allowed(url.as_str(), platform::app_origin(), dev_origin);
    if !allowed {
        log::warn!("blocked navigation to {url}");
    }
    allowed
}
