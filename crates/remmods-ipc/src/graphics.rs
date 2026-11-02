use remmods_core::graphics as core;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum GraphicsMode {
    Auto,
    ForceShm,
    Software,
}

/// An automatic step after a crashed start that the user has not acknowledged yet.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphicsStep {
    pub from: GraphicsMode,
    pub to: GraphicsMode,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphicsState {
    /// False on Windows; then the other fields are defaults.
    pub supported: bool,
    /// Stored in `graphics.json`; applies on the next start.
    pub mode: GraphicsMode,
    /// What this session runs with.
    pub effective: GraphicsMode,
    /// Started with `--safe-graphics`.
    pub safe_flag: bool,
    /// The user's environment sets WebKit renderer variables.
    pub user_env: bool,
    pub step: Option<GraphicsStep>,
    /// `graphics.json` is newer or unreadable.
    pub read_only: bool,
}

impl GraphicsState {
    /// What every OS without the fallback reports.
    pub fn unsupported() -> Self {
        Self {
            supported: false,
            mode: GraphicsMode::Auto,
            effective: GraphicsMode::Auto,
            safe_flag: false,
            user_env: false,
            step: None,
            read_only: false,
        }
    }
}

impl From<core::GraphicsMode> for GraphicsMode {
    fn from(mode: core::GraphicsMode) -> Self {
        match mode {
            core::GraphicsMode::Auto => Self::Auto,
            core::GraphicsMode::ForceShm => Self::ForceShm,
            core::GraphicsMode::Software => Self::Software,
        }
    }
}

impl From<GraphicsMode> for core::GraphicsMode {
    fn from(mode: GraphicsMode) -> Self {
        match mode {
            GraphicsMode::Auto => Self::Auto,
            GraphicsMode::ForceShm => Self::ForceShm,
            GraphicsMode::Software => Self::Software,
        }
    }
}

impl From<&core::GraphicsStep> for GraphicsStep {
    fn from(step: &core::GraphicsStep) -> Self {
        Self {
            from: step.from.into(),
            to: step.to.into(),
        }
    }
}
