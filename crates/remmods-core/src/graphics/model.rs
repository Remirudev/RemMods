use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{GRAPHICS_FILE, GRAPHICS_VERSION, MARKER_FILE};
use crate::paths::AppPaths;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GraphicsMode {
    Auto,
    ForceShm,
    Software,
}

impl GraphicsMode {
    /// The fallback after a crash: Auto → ForceShm → Software → none.
    pub fn next(self) -> Option<GraphicsMode> {
        match self {
            Self::Auto => Some(Self::ForceShm),
            Self::ForceShm => Some(Self::Software),
            Self::Software => None,
        }
    }
}

/// Where the session's mode came from: `graphics.json` or `--safe-graphics`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModeSource {
    Stored,
    Flag,
}

/// An automatic step after a crashed start that the user has not acknowledged yet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphicsStep {
    pub from: GraphicsMode,
    pub to: GraphicsMode,
    /// RFC 3339.
    pub at: String,
}

/// On-disk `graphics.json`. Missing fields take defaults and unknown fields are ignored.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GraphicsFile {
    pub version: u32,
    pub mode: GraphicsMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stepped: Option<GraphicsStep>,
}

impl Default for GraphicsFile {
    fn default() -> Self {
        Self {
            version: GRAPHICS_VERSION,
            mode: GraphicsMode::Auto,
            stepped: None,
        }
    }
}

/// `graphics-pending`: written at start, removed once the frontend ran. A marker that survives
/// means the previous start did not get that far.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingMarker {
    pub version: u32,
    pub mode: GraphicsMode,
    pub source: ModeSource,
    pub pid: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GraphicsFacts {
    pub nvidia: bool,
    pub wayland: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphicsPaths {
    pub file: PathBuf,
    pub marker: PathBuf,
    pub lock_file: PathBuf,
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
}

impl GraphicsPaths {
    pub fn from_app(paths: &AppPaths) -> GraphicsPaths {
        GraphicsPaths {
            file: paths.config_dir.join(GRAPHICS_FILE),
            marker: paths.config_dir.join(MARKER_FILE),
            lock_file: paths.lock_file.clone(),
            config_dir: paths.config_dir.clone(),
            data_dir: paths.data_dir.clone(),
        }
    }
}

/// What `start` needs from the process; tests inject every field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartInput {
    pub safe_flag: bool,
    pub facts: GraphicsFacts,
    /// Names of the managed and user-control variables already set in the environment.
    pub present: Vec<String>,
    pub other_instance: bool,
    pub pid: u32,
    /// RFC 3339; the time a step is recorded with.
    pub now: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphicsSession {
    pub stored: GraphicsMode,
    pub effective: GraphicsMode,
    pub source: ModeSource,
    /// The variables to set before GTK starts; never one the user already set.
    pub env: Vec<(&'static str, &'static str)>,
    pub stepped: Option<GraphicsStep>,
    /// This start stepped the stored mode down.
    pub stepped_now: bool,
    pub user_controlled: bool,
    pub marker_written: bool,
    pub read_only: bool,
    /// Log lines for the shell (the logger is not up while `start` runs).
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphicsLoad {
    pub file: GraphicsFile,
    /// Newer format or unreadable file: never written back.
    pub read_only: bool,
    /// The file was corrupt, backed up and replaced by defaults.
    pub recovered: bool,
}
