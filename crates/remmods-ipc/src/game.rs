use std::path::PathBuf;

use remmods_core::game as core;
use remmods_core::paths::display_path;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum RunningState {
    Running,
    NotRunning,
    Unknown,
}

/// Steam updated the game since the build RemMods last saw (build ids, not versions).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNotice {
    pub from: String,
    pub to: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum LocationSource {
    Steam,
    Manual,
    Fixture,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum SteamKind {
    Native,
    Flatpak,
    Snap,
    Windows,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum UpdateTracking {
    Available,
    NoManifest,
    Unreadable,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum FolderProblem {
    NotFound,
    Ambiguous,
    NotUnicode,
    Unreadable,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstallInfo {
    pub root: String,
    pub paks: String,
    pub source: LocationSource,
    pub steam_kind: Option<SteamKind>,
    pub build_id: Option<String>,
    pub update_tracking: UpdateTracking,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum GameLocation {
    NotSet,
    Ready { install: InstallInfo },
    Missing { path: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameCandidate {
    pub id: String,
    pub root: String,
    pub library: String,
    pub steam_kind: SteamKind,
    pub build_id: Option<String>,
    pub fully_installed: bool,
    pub problem: Option<FolderProblem>,
    /// Same folder as the current location; the UI never compares paths itself.
    pub in_use: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DetectionNote {
    pub code: NoteCode,
    pub path: String,
    pub line: Option<u32>,
    /// A code the UI translates (Steam kind, VDF or manifest error), never English.
    pub detail: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameStatus {
    pub location: GameLocation,
    pub running: RunningState,
    pub session: u32,
    pub other_manager: bool,
    pub update: Option<UpdateNotice>,
    pub candidates: Vec<GameCandidate>,
    pub notes: Vec<DetectionNote>,
}

/// Input of `game_set_location`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LocationChoice {
    Candidate { id: String },
    Path { path: String },
}

/// Result of `game_pick_location`. `cancelled` also covers "no folder picker available".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PickOutcome {
    Cancelled,
    Selected { status: GameStatus },
    Invalid { reason: FolderProblem, path: String },
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum RevealProblem {
    /// A name the Windows shell would normalise or map to a device.
    UnsupportedName,
    Failed,
}

impl From<core::RunningState> for RunningState {
    fn from(state: core::RunningState) -> Self {
        match state {
            core::RunningState::Running => Self::Running,
            core::RunningState::NotRunning => Self::NotRunning,
            core::RunningState::Unknown => Self::Unknown,
        }
    }
}

impl From<&core::UpdateNotice> for UpdateNotice {
    fn from(notice: &core::UpdateNotice) -> Self {
        Self {
            from: notice.from.clone(),
            to: notice.to.clone(),
        }
    }
}

impl From<core::LocationSource> for LocationSource {
    fn from(source: core::LocationSource) -> Self {
        match source {
            core::LocationSource::Steam => Self::Steam,
            core::LocationSource::Manual => Self::Manual,
            core::LocationSource::Fixture => Self::Fixture,
        }
    }
}

impl From<core::SteamKind> for SteamKind {
    fn from(kind: core::SteamKind) -> Self {
        match kind {
            core::SteamKind::Native => Self::Native,
            core::SteamKind::Flatpak => Self::Flatpak,
            core::SteamKind::Snap => Self::Snap,
            core::SteamKind::Windows => Self::Windows,
        }
    }
}

impl From<core::UpdateTracking> for UpdateTracking {
    fn from(tracking: core::UpdateTracking) -> Self {
        match tracking {
            core::UpdateTracking::Available => Self::Available,
            core::UpdateTracking::NoManifest => Self::NoManifest,
            core::UpdateTracking::Unreadable => Self::Unreadable,
        }
    }
}

impl From<core::FolderProblem> for FolderProblem {
    fn from(problem: core::FolderProblem) -> Self {
        match problem {
            core::FolderProblem::NotFound => Self::NotFound,
            core::FolderProblem::Ambiguous => Self::Ambiguous,
            core::FolderProblem::NotUnicode => Self::NotUnicode,
            core::FolderProblem::Unreadable => Self::Unreadable,
        }
    }
}

impl From<&core::InstallInfo> for InstallInfo {
    fn from(install: &core::InstallInfo) -> Self {
        Self {
            root: display_path(&install.root),
            paks: display_path(&install.paks),
            source: install.source.into(),
            steam_kind: install.steam_kind.map(SteamKind::from),
            build_id: install.build_id.clone(),
            update_tracking: install.update_tracking.into(),
        }
    }
}

impl From<&core::GameLocation> for GameLocation {
    fn from(location: &core::GameLocation) -> Self {
        match location {
            core::GameLocation::NotSet => Self::NotSet,
            core::GameLocation::Ready(install) => Self::Ready {
                install: install.into(),
            },
            core::GameLocation::Missing { path } => Self::Missing {
                path: display_path(path),
            },
        }
    }
}

impl From<&core::GameCandidate> for GameCandidate {
    fn from(candidate: &core::GameCandidate) -> Self {
        Self {
            id: candidate.id.clone(),
            root: display_path(&candidate.root),
            library: display_path(&candidate.library),
            steam_kind: candidate.steam_kind.into(),
            build_id: candidate.build_id.clone(),
            fully_installed: candidate.fully_installed,
            problem: candidate.problem.map(FolderProblem::from),
            in_use: candidate.in_use,
        }
    }
}

impl From<core::NoteCode> for NoteCode {
    fn from(code: core::NoteCode) -> Self {
        match code {
            core::NoteCode::SteamRoot => Self::SteamRoot,
            core::NoteCode::NoSteam => Self::NoSteam,
            core::NoteCode::VdfError => Self::VdfError,
            core::NoteCode::LibraryMissing => Self::LibraryMissing,
            core::NoteCode::LibraryNotAbsolute => Self::LibraryNotAbsolute,
            core::NoteCode::ManifestError => Self::ManifestError,
            core::NoteCode::InstallIncomplete => Self::InstallIncomplete,
            core::NoteCode::GameFolderInvalid => Self::GameFolderInvalid,
            core::NoteCode::PathNotUnicode => Self::PathNotUnicode,
            core::NoteCode::RegistryUnreadable => Self::RegistryUnreadable,
        }
    }
}

impl From<&core::DetectionNote> for DetectionNote {
    fn from(note: &core::DetectionNote) -> Self {
        Self {
            code: note.code.into(),
            path: display_path(&note.path),
            line: note.line,
            detail: note.detail.clone(),
        }
    }
}

impl From<&core::GameStatus> for GameStatus {
    fn from(status: &core::GameStatus) -> Self {
        Self {
            location: (&status.location).into(),
            running: status.running.into(),
            session: status.session,
            other_manager: status.other_manager,
            update: status.update.as_ref().map(UpdateNotice::from),
            candidates: status.candidates.iter().map(GameCandidate::from).collect(),
            notes: status.notes.iter().map(DetectionNote::from).collect(),
        }
    }
}

impl From<LocationChoice> for core::LocationChoice {
    fn from(choice: LocationChoice) -> Self {
        match choice {
            LocationChoice::Candidate { id } => Self::Candidate { id },
            LocationChoice::Path { path } => Self::Path {
                path: PathBuf::from(path),
            },
        }
    }
}

impl From<&core::PickResult> for PickOutcome {
    fn from(result: &core::PickResult) -> Self {
        match result {
            core::PickResult::Selected(status) => Self::Selected {
                status: status.into(),
            },
            core::PickResult::Invalid { problem, path } => Self::Invalid {
                reason: (*problem).into(),
                path: display_path(path),
            },
        }
    }
}
