use std::ffi::OsStr;

use super::{GraphicsFacts, GraphicsMode, ModeSource, PendingMarker, USER_CONTROL_VARS};

/// The variables a mode needs. Never `WEBKIT_DISABLE_COMPOSITING_MODE`: in WebKitGTK 2.54
/// disabling the DMA-BUF renderer already turns compositing off.
pub fn env_for(mode: GraphicsMode, facts: GraphicsFacts) -> Vec<(&'static str, &'static str)> {
    let mut env = Vec::new();
    if facts.nvidia && facts.wayland {
        env.push(("__NV_DISABLE_EXPLICIT_SYNC", "1"));
    }
    match mode {
        GraphicsMode::Auto => {}
        GraphicsMode::ForceShm => env.push(("WEBKIT_DMABUF_RENDERER_FORCE_SHM", "1")),
        GraphicsMode::Software => env.push(("WEBKIT_DISABLE_DMABUF_RENDERER", "1")),
    }
    env
}

/// True when the user's environment already picks a WebKit renderer path.
pub fn is_user_controlled(present: &[String]) -> bool {
    present
        .iter()
        .any(|name| USER_CONTROL_VARS.contains(&name.as_str()))
}

/// `WAYLAND_DISPLAY` set (non-empty) or `XDG_SESSION_TYPE=wayland`, unless `GDK_BACKEND` forces
/// X11 first.
pub fn detect_wayland(
    wayland_display: Option<&OsStr>,
    session_type: Option<&OsStr>,
    gdk_backend: Option<&OsStr>,
) -> bool {
    let display = wayland_display.is_some_and(|value| !value.is_empty());
    let session = session_type.is_some_and(|value| value == "wayland");
    let forced_x11 = gdk_backend.is_some_and(|value| value.as_encoded_bytes().starts_with(b"x11"));
    (display || session) && !forced_x11
}

/// The mode to step to after a start that left its marker behind. Only a crash in the stored
/// mode counts: a `--safe-graphics` run or a marker of another mode says nothing about it.
pub fn step_after_crash(
    stored: GraphicsMode,
    marker: Option<&PendingMarker>,
    marker_corrupt: bool,
) -> Option<GraphicsMode> {
    let crashed = marker_corrupt
        || marker
            .is_some_and(|marker| marker.source == ModeSource::Stored && marker.mode == stored);
    if crashed { stored.next() } else { None }
}
