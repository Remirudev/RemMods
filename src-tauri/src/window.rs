use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use remmods_core::settings::{Appearance, ColorScheme, TitleBarMode};
use tauri::{AppHandle, Manager, WebviewWindow, WebviewWindowBuilder};

use crate::os;
use crate::smoke;
use crate::state::AppState;

pub const MAIN_WINDOW: &str = "main";

/// Shown anyway if the frontend never calls `app_ready` (e.g. a crashed bundle), so the user
/// sees something instead of a hidden process.
const SHOW_FALLBACK: Duration = Duration::from_secs(3);

/// Builds the window from `app.windows[0]` (`create: false`) with the saved title bar mode,
/// colour scheme and UI scale, the navigation filter and, for `--smoke`, a throwaway webview
/// data folder.
pub fn create_main_window(
    app: &AppHandle,
    appearance: &Appearance,
    webview_data_dir: Option<PathBuf>,
) -> tauri::Result<WebviewWindow> {
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == MAIN_WINDOW)
        .cloned()
        .ok_or(tauri::Error::WindowNotFound)?;
    let dev_origin = os::navigation::dev_origin(app);
    let handle = app.clone();
    let mut builder = WebviewWindowBuilder::from_config(app, &config)?
        .decorations(appearance.title_bar == TitleBarMode::System)
        .theme(os::theme::window_theme(appearance.scheme))
        .on_navigation(move |url| {
            let allowed = os::navigation::is_allowed(url, dev_origin.as_deref());
            if !allowed {
                smoke::record_blocked_navigation(&handle);
            }
            allowed
        });
    if let Some(dir) = webview_data_dir {
        builder = builder.data_directory(dir);
    }
    let window = builder.build()?;
    if let Err(error) = window.set_zoom(appearance.ui_scale) {
        log::warn!("applying the saved UI scale failed: {error}");
    }
    Ok(window)
}

/// Shows and focuses the main window the first time it is called; later calls do nothing.
pub fn show_main_window_once(app: &AppHandle, state: &AppState, reason: &str) {
    if state.shown.swap(true, Ordering::SeqCst) {
        return;
    }
    log::info!("showing the main window ({reason})");
    focus_main_window(app);
}

/// Keeps the window theme in step with a changed colour scheme (see `os::theme`).
pub fn apply_color_scheme(app: &AppHandle, scheme: ColorScheme) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        os::theme::sync_window_theme(&window, scheme);
    }
}

pub fn focus_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        log::warn!("main window not found");
        return;
    };
    let shown = window
        .unminimize()
        .and_then(|()| window.show())
        .and_then(|()| window.set_focus());
    if let Err(error) = shown {
        log::error!("showing the main window failed: {error}");
    }
}

pub fn start_show_fallback(app: AppHandle) -> std::io::Result<()> {
    thread::Builder::new()
        .name("show-fallback".into())
        .spawn(move || {
            thread::sleep(SHOW_FALLBACK);
            let state = app.state::<AppState>();
            if !state.shown.load(Ordering::SeqCst) {
                log::warn!("app_ready not received within {SHOW_FALLBACK:?}");
                show_main_window_once(&app, &state, "fallback");
            }
        })
        .map(drop)
}
