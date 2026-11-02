//! Public game types: where the game is, whether it runs, what detection found.

use std::fmt;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use crate::platform::{ProcessRecord, SteamHints};

pub const STEAM_APP_ID: u32 = 1_778_820;
pub const STEAM_LAUNCH_URL: &str = "steam://rungameid/1778820";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameStatus {
    pub location: GameLocation,
    pub running: RunningState,
    pub session: u32,
    pub other_manager: bool,
    pub update: Option<UpdateNotice>,
    pub candidates: Vec<GameCandidate>,
    pub notes: Vec<DetectionNote>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameLocation {
    NotSet,
    Ready(InstallInfo),
    Missing { path: PathBuf },
}

/// A validated game folder. All paths are plain absolute paths with on-disk names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallInfo {
    pub root: PathBuf,
    pub paks: PathBuf,
    pub exe: PathBuf,
    pub source: LocationSource,
    pub steam_kind: Option<SteamKind>,
    pub build_id: Option<String>,
    pub update_tracking: UpdateTracking,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocationSource {
    Steam,
    Manual,
    Fixture,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SteamKind {
    Native,
    Flatpak,
    Snap,
    Windows,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateTracking {
    Available,
    NoManifest,
    Unreadable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameCandidate {
    pub id: String,
    pub root: PathBuf,
    pub library: PathBuf,
    pub steam_kind: SteamKind,
    pub build_id: Option<String>,
    pub fully_installed: bool,
    pub problem: Option<FolderProblem>,
    /// Same folder as the Ready location (file identity, never a path comparison).
    pub in_use: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderProblem {
    NotFound,
    Ambiguous,
    NotUnicode,
    Unreadable,
}

/// Something detection noticed. `detail` is a code the UI translates, never English.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetectionNote {
    pub code: NoteCode,
    pub path: PathBuf,
    pub line: Option<u32>,
    pub detail: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteCode {
    SteamRoot,
    NoSteam,
    VdfError,
    LibraryMissing,
    LibraryNotAbsolute,
    ManifestError,
    InstallIncomplete,
    GameFolderInvalid,
    PathNotUnicode,
    RegistryUnreadable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunningState {
    Running,
    NotRunning,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateNotice {
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocationChoice {
    Candidate { id: String },
    Path { path: PathBuf },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PickResult {
    Selected(GameStatus),
    Invalid {
        problem: FolderProblem,
        path: PathBuf,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameStatusChange {
    pub running: RunningState,
    pub session: u32,
    pub other_manager: bool,
    pub update: Option<UpdateNotice>,
}

/// How to start the game: always through Steam (core never starts the exe itself).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchPlan {
    pub url: String,
    pub steam_kind: Option<SteamKind>,
}

/// The Paks folder of the Ready location.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaksLocation {
    pub paks: PathBuf,
    /// Paks as the game sees it (`S:\…` under Proton, the real path on Windows).
    pub game_view: String,
    /// Grows by one on every location change.
    pub generation: u32,
}

/// `Default` turns Steam detection off; `CoreOptions::default()` turns it on.
#[derive(Clone, Debug, Default)]
pub struct GameOptions {
    pub steam_detection: bool,
    /// `--smoke --paks`: this location only, never Steam, nothing stored for it.
    pub fixed_location: Option<PathBuf>,
    /// Injected by tests; `platform::steam_hints()` otherwise.
    pub steam_hints: Option<SteamHints>,
    /// Injected by tests; `platform::processes()` otherwise.
    pub processes: Option<ProcessSource>,
}

/// Where the running check gets its process list from.
#[derive(Clone)]
pub struct ProcessSource(pub Arc<dyn Fn() -> io::Result<Vec<ProcessRecord>> + Send + Sync>);

impl ProcessSource {
    pub fn scan(&self) -> io::Result<Vec<ProcessRecord>> {
        (self.0)()
    }
}

impl fmt::Debug for ProcessSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ProcessSource(..)")
    }
}
