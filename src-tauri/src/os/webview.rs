/// Windows: without the WebView2 runtime the window would never appear, so say what is missing
/// and where to get it before building the window.
#[cfg(windows)]
pub fn ensure_runtime(smoke: bool) {
    if let Err(error) = tauri::webview_version() {
        log::error!("WebView2 runtime not found: {error}");
        crate::fail_fast(
            "RemMods needs the Microsoft Edge WebView2 Runtime (Evergreen)",
            &"Download it from https://developer.microsoft.com/microsoft-edge/webview2/ and start RemMods again.",
            smoke,
        );
    }
}

/// Linux ships WebKitGTK as a package dependency; nothing to check at runtime.
#[cfg(not(windows))]
pub fn ensure_runtime(_smoke: bool) {}
