//! Types that cross the IPC boundary: serde + specta only, no Tauri. No `PathBuf` (display strings
//! only) and no `u64` (JavaScript numbers lose precision past 2^53).

mod error;
mod platform;
mod settings;
mod smoke;

pub use error::{AppError, OsErrorClass};
pub use platform::{AppPathsDto, OsKind, PlatformInfo};
pub use settings::{
    Appearance, AppearanceUpdate, ColorScheme, Settings, SettingsUpdate, TitleBarMode,
};
pub use smoke::{SmokeCheck, SmokeReport};
