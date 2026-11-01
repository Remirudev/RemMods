//! Types that cross the IPC boundary: serde + specta only, no Tauri. No `PathBuf` (display strings
//! only) and no `u64` (JavaScript numbers lose precision past 2^53).

mod error;
mod game;
mod mod_issues;
mod mods;
mod platform;
mod settings;
mod smoke;

pub use error::{AppError, OsErrorClass};
pub use game::{
    DetectionNote, FolderProblem, GameCandidate, GameLocation, GameStatus, InstallInfo,
    LocationChoice, LocationSource, NoteCode, PickOutcome, RevealProblem, RunningState, SteamKind,
    UpdateNotice, UpdateTracking,
};
pub use mod_issues::{ModIssue, NameProblem};
pub use mods::{
    ArchiveEntry, ConflictLink, LeftoverEntry, ModDetails, ModInfo, ModInfoDefaulted,
    ModInfoOrigin, ModKind, ModRow, ModState, ModsSnapshot, PayloadCounts, PayloadFile,
    PayloadKind, RootInfo, RootKind, ScanProblem, ScanStats, UnmanagedEntry, UnmanagedKind,
};
pub use platform::{AppPathsDto, OsKind, PlatformInfo};
pub use settings::{
    Appearance, AppearanceUpdate, ColorScheme, Settings, SettingsUpdate, TitleBarMode,
};
pub use smoke::{SmokeCheck, SmokeReport};
