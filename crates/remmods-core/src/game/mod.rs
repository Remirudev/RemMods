//! TEKKEN 8 itself: where it is installed, which build it has, whether it runs. Read-only: P1
//! never writes under the game folder or a Steam root.

mod api;
mod detect;
mod error;
mod launch;
mod layout;
mod model;
mod process;
mod running;
mod service;
mod settle;
pub mod steam;
mod update;

pub use detect::{Detection, detect};
pub use error::GameError;
pub use layout::{
    GameLayout, MAX_PICK_LEVELS, ManifestLocation, game_view_of_paks, manifest_for_root,
    resolve_pick, validate_game_root, with_disk_casing,
};
pub use model::{
    DetectionNote, FolderProblem, GameCandidate, GameLocation, GameOptions, GameStatus,
    GameStatusChange, InstallInfo, LaunchPlan, LocationChoice, LocationSource, NoteCode,
    PaksLocation, PickResult, ProcessSource, RunningState, STEAM_APP_ID, STEAM_LAUNCH_URL,
    SteamKind, UpdateNotice, UpdateTracking,
};
pub use process::{
    COMM_MAX_BYTES, GAME_EXECUTABLES, GameMatch, GameSignal, ORIGINAL_APP_EXECUTABLE,
    ProcessVerdict, classify, game_matches,
};
pub use running::{NOT_RUNNING_AFTER, NOT_RUNNING_AFTER_SCANS, RunningTracker};
pub use service::GameService;
pub use steam::{
    AppManifest, LibraryFileError, ManifestError, VDF_MAX_BYTES, VDF_MAX_DEPTH, Vdf, VdfError,
    VdfErrorKind, flatpak_library_path, linux_root_candidates, parse_app_manifest,
    parse_library_folders, parse_vdf, parse_vdf_bytes, steam_kind_of, windows_root_candidates,
};
pub use update::{UpdateDecision, decide_update};

/// Steam's manifest file name for TEKKEN 8.
pub const MANIFEST_FILE: &str = "appmanifest_1778820.acf";
