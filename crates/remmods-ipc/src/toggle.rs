use remmods_core::mods as core;
use serde::{Deserialize, Serialize};

use crate::mods::ModsSnapshot;

/// Why a row cannot be toggled; the UI disables its switch and core re-checks it.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ToggleBlock {
    Ambiguous,
    Linked,
    UnknownState,
    NoPakFiles,
    InvalidLogicName,
}

/// A row as the UI saw it: core refuses the request as stale when `modRef` and `key` no longer
/// name the same row.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ToggleTarget {
    pub mod_ref: String,
    pub key: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ToggleRequest {
    pub targets: Vec<ToggleTarget>,
    pub enable: bool,
    pub apply_while_running: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SkippedRow {
    pub key: String,
    pub name: String,
    pub reason: ToggleBlock,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ToggleOutcome {
    pub snapshot: ModsSnapshot,
    pub skipped: Vec<SkippedRow>,
    /// Unmanaged entries with live payload files, which "disable all" never touches.
    pub unmanaged: u32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum WatchState {
    Active,
    LimitReached,
    Unavailable,
    Failed,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WatchStatus {
    pub state: WatchState,
}

impl From<core::ToggleBlock> for ToggleBlock {
    fn from(block: core::ToggleBlock) -> Self {
        match block {
            core::ToggleBlock::Ambiguous => Self::Ambiguous,
            core::ToggleBlock::Linked => Self::Linked,
            core::ToggleBlock::UnknownState => Self::UnknownState,
            core::ToggleBlock::NoPakFiles => Self::NoPakFiles,
            core::ToggleBlock::InvalidLogicName => Self::InvalidLogicName,
        }
    }
}

impl From<&ToggleRequest> for core::ToggleRequest {
    fn from(request: &ToggleRequest) -> Self {
        Self {
            targets: request
                .targets
                .iter()
                .map(|target| core::ToggleTarget {
                    mod_ref: core::ModRef::from_token(target.mod_ref.clone()),
                    key: target.key.clone(),
                })
                .collect(),
            enable: request.enable,
            apply_while_running: request.apply_while_running,
        }
    }
}

impl From<&core::SkippedRow> for SkippedRow {
    fn from(row: &core::SkippedRow) -> Self {
        Self {
            key: row.key.clone(),
            name: row.name.clone(),
            reason: row.reason.into(),
        }
    }
}

impl From<&core::ToggleOutcome> for ToggleOutcome {
    fn from(outcome: &core::ToggleOutcome) -> Self {
        Self {
            snapshot: ModsSnapshot::from(&*outcome.refreshed.snapshot),
            skipped: outcome.skipped.iter().map(SkippedRow::from).collect(),
            unmanaged: outcome.unmanaged,
        }
    }
}
