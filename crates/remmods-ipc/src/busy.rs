use std::time::Duration;

use remmods_core::safe_fs as core;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum BusyPhase {
    Waiting,
    Done,
    TimedOut,
    Cancelled,
    Failed,
}

/// The `folder-busy` payload: a folder rename waits for another program (WG-2). `folder` is the
/// path below Paks; `holders` is empty until WG-1; `cancellable` is false for rollbacks.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FolderBusyInfo {
    pub wait_id: u32,
    pub phase: BusyPhase,
    pub folder: String,
    pub holders: Vec<String>,
    #[specta(type = specta_typescript::Number)]
    pub elapsed_ms: f64,
    #[specta(type = specta_typescript::Number)]
    pub limit_ms: f64,
    pub cancellable: bool,
}

impl From<core::BusyPhase> for BusyPhase {
    fn from(phase: core::BusyPhase) -> Self {
        match phase {
            core::BusyPhase::Waiting => Self::Waiting,
            core::BusyPhase::Done => Self::Done,
            core::BusyPhase::TimedOut => Self::TimedOut,
            core::BusyPhase::Cancelled => Self::Cancelled,
            core::BusyPhase::Failed => Self::Failed,
        }
    }
}

impl From<&core::BusyEvent> for FolderBusyInfo {
    fn from(event: &core::BusyEvent) -> Self {
        Self {
            wait_id: event.wait_id,
            phase: event.phase.into(),
            folder: event.folder.clone(),
            holders: event.holders.clone().unwrap_or_default(),
            elapsed_ms: whole_millis(event.elapsed),
            limit_ms: whole_millis(event.limit),
            cancellable: event.cancellable,
        }
    }
}

/// Whole milliseconds; the waits last seconds, far below f64's exact integer range.
fn whole_millis(duration: Duration) -> f64 {
    duration.as_millis() as f64
}
