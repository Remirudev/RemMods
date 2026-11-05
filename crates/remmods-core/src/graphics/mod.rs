//! Linux graphics fallback (PLAN §5): which WebKitGTK renderer variables a session sets, the
//! stored choice in `graphics.json` and the crash marker that steps the next start down. Pure
//! decisions live in `decide`, file handling in `store`; the shell sets the variables.

mod decide;
mod error;
mod model;
mod store;

pub use decide::{detect_wayland, env_for, is_user_controlled, step_after_crash};
pub use error::GraphicsError;
pub use model::{
    GraphicsFacts, GraphicsFile, GraphicsLoad, GraphicsMode, GraphicsPaths, GraphicsSession,
    GraphicsStep, ModeSource, PendingMarker, StartInput,
};
pub use store::{ack_step, clear_marker, load, set_mode, start};

/// In `config_dir` (PLAN §4 table).
pub const GRAPHICS_FILE: &str = "graphics.json";
/// In `config_dir`; written at start, cleared once the frontend ran.
pub const MARKER_FILE: &str = "graphics-pending";
pub const GRAPHICS_VERSION: u32 = 1;
/// Every variable RemMods itself may set.
pub const MANAGED_VARS: [&str; 3] = [
    "__NV_DISABLE_EXPLICIT_SYNC",
    "WEBKIT_DMABUF_RENDERER_FORCE_SHM",
    "WEBKIT_DISABLE_DMABUF_RENDERER",
];
/// A user who sets any of these controls the renderer; RemMods then sets no `WEBKIT_*` var.
pub const USER_CONTROL_VARS: [&str; 3] = [
    "WEBKIT_DMABUF_RENDERER_FORCE_SHM",
    "WEBKIT_DISABLE_DMABUF_RENDERER",
    "WEBKIT_DISABLE_COMPOSITING_MODE",
];

/// Local time in RFC 3339, for `GraphicsStep::at`.
pub fn now_rfc3339() -> String {
    chrono::Local::now().to_rfc3339()
}
