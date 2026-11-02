//! The only `cfg` place in the shell besides `build.rs`: OS-specific glue around core.

pub mod graphics;
pub mod launch;
pub mod navigation;
pub mod reveal;
pub mod theme;
pub mod webview;

/// The opener's error enum is `non_exhaustive`; only its I/O variant keeps a typed cause.
#[cfg(windows)]
fn opener_io_error(error: tauri_plugin_opener::Error) -> std::io::Error {
    match error {
        tauri_plugin_opener::Error::Io(source) => source,
        other => std::io::Error::other(other.to_string()),
    }
}
