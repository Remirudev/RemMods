use remmods_core::CoreError;
use remmods_core::mods as core;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, quiet};
use crate::manage::{ModTarget, TrashKind, TrashUnavailableReason};
use crate::mods::{ModsSnapshot, bytes};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BulkTrashRequest {
    pub targets: Vec<ModTarget>,
    pub apply_while_running: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BulkDeleteRequest {
    pub targets: Vec<ModTarget>,
    pub token: String,
    pub apply_while_running: bool,
}

/// One row of a bulk delete prepare; `refusal` says why it cannot be deleted (null: the token
/// covers it).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PreparedRow {
    pub target: ModTarget,
    pub name: String,
    #[specta(type = Option<specta_typescript::Number>)]
    pub size: Option<f64>,
    pub refusal: Option<AppError>,
}

/// `token` is null when no row is eligible; it covers exactly the rows without a refusal, in
/// order.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BulkDeletePrepared {
    pub token: Option<String>,
    pub rows: Vec<PreparedRow>,
    pub expires_in_secs: u32,
}

/// What a bulk call did with one row. `unavailable` (trash only) changed nothing and lets the UI
/// offer the permanent delete; `notRun` rows were skipped after a cancelled folder-busy wait.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BulkRowResult {
    Trashed {
        items: u32,
        trash: TrashKind,
    },
    Deleted {
        leftover: Option<String>,
    },
    Unavailable {
        reason: TrashUnavailableReason,
        #[specta(type = Option<specta_typescript::Number>)]
        size: Option<f64>,
    },
    Failed {
        error: AppError,
    },
    NotRun,
}

/// `name` is the row's display name in the fresh scan (its key when it was not found).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BulkRow {
    pub target: ModTarget,
    pub name: String,
    pub result: BulkRowResult,
}

/// `snapshot` is null when the final rescan failed after rows ran; the rows still count.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BulkOutcome {
    pub snapshot: Option<ModsSnapshot>,
    pub rows: Vec<BulkRow>,
}

fn core_targets(targets: &[ModTarget]) -> Vec<core::ModTarget> {
    targets.iter().map(core::ModTarget::from).collect()
}

impl From<&BulkTrashRequest> for core::BulkTrashRequest {
    fn from(request: &BulkTrashRequest) -> Self {
        Self {
            targets: core_targets(&request.targets),
            apply_while_running: request.apply_while_running,
        }
    }
}

impl From<&BulkDeleteRequest> for core::BulkDeleteRequest {
    fn from(request: &BulkDeleteRequest) -> Self {
        Self {
            targets: core_targets(&request.targets),
            token: request.token.clone(),
            apply_while_running: request.apply_while_running,
        }
    }
}

impl From<core::PreparedRow> for PreparedRow {
    fn from(row: core::PreparedRow) -> Self {
        Self {
            target: ModTarget::from(&row.target),
            name: row.name,
            size: row.size.map(bytes),
            refusal: row.refusal.map(|error| quiet(CoreError::Mods(error))),
        }
    }
}

impl From<core::BulkDeletePrepared> for BulkDeletePrepared {
    fn from(prepared: core::BulkDeletePrepared) -> Self {
        Self {
            token: prepared.token,
            rows: prepared.rows.into_iter().map(PreparedRow::from).collect(),
            expires_in_secs: prepared.expires_in_secs,
        }
    }
}

impl From<core::BulkRowResult> for BulkRowResult {
    fn from(result: core::BulkRowResult) -> Self {
        match result {
            core::BulkRowResult::Trashed { items, kind } => Self::Trashed {
                items,
                trash: kind.into(),
            },
            core::BulkRowResult::Deleted { leftover } => Self::Deleted { leftover },
            core::BulkRowResult::TrashUnavailable { reason, size } => Self::Unavailable {
                reason: reason.into(),
                size: size.map(bytes),
            },
            core::BulkRowResult::Failed(error) => {
                log::info!("bulk row failed: {error}");
                Self::Failed {
                    error: quiet(error),
                }
            }
            core::BulkRowResult::NotRun => Self::NotRun,
        }
    }
}

impl From<core::BulkRow> for BulkRow {
    fn from(row: core::BulkRow) -> Self {
        Self {
            target: ModTarget::from(&row.target),
            name: row.name,
            result: row.result.into(),
        }
    }
}

/// The row errors move into `AppError` without a warning each; one summary warning per call.
impl From<core::BulkOutcome> for BulkOutcome {
    fn from(outcome: core::BulkOutcome) -> Self {
        let total = outcome.rows.len();
        let failed = outcome
            .rows
            .iter()
            .filter(|row| matches!(row.result, core::BulkRowResult::Failed(_)))
            .count();
        if failed > 0 {
            log::warn!("bulk call: {failed} of {total} rows failed");
        }
        Self {
            snapshot: outcome
                .refreshed
                .as_ref()
                .map(|refreshed| ModsSnapshot::from(&*refreshed.snapshot)),
            rows: outcome.rows.into_iter().map(BulkRow::from).collect(),
        }
    }
}
