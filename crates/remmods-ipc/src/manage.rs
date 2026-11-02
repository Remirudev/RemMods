use remmods_core::mods as core;
use remmods_core::platform;
use serde::{Deserialize, Serialize};

use crate::mods::{ModsSnapshot, bytes};

/// A row as the UI saw it (the `ToggleTarget` shape); core refuses it as stale when `modRef` and
/// `key` no longer name the same row.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModTarget {
    pub mod_ref: String,
    pub key: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RenameRequest {
    pub target: ModTarget,
    pub new_name: String,
    pub apply_while_running: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RekeyReport {
    pub profiles: u32,
    pub meta: bool,
    pub skipped_ambiguous: bool,
    pub failed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RenameOutcome {
    pub snapshot: ModsSnapshot,
    pub old_key: String,
    pub new_key: String,
    pub new_ref: Option<String>,
    pub rekey: RekeyReport,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TrashRequest {
    pub target: ModTarget,
    pub apply_while_running: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum TrashKind {
    Home,
    TopdirShared,
    TopdirUser,
    RecycleBin,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TrashOutcome {
    pub snapshot: ModsSnapshot,
    pub key: String,
    pub items: u32,
    pub kind: TrashKind,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DeletePrepared {
    pub token: String,
    pub key: String,
    pub name: String,
    #[specta(type = Option<specta_typescript::Number>)]
    pub size: Option<f64>,
    pub expires_in_secs: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRequest {
    pub target: ModTarget,
    pub token: String,
    pub apply_while_running: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DeleteOutcome {
    pub snapshot: ModsSnapshot,
    pub key: String,
    pub leftover: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModInfoEdit {
    pub version: String,
    pub category: String,
    pub description: String,
    pub author: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MetadataRequest {
    pub target: ModTarget,
    pub info: ModInfoEdit,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ManageAction {
    Rename,
    Trash,
    Delete,
    Metadata,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ManageBlock {
    Loose,
    Linked,
    Archive,
    Leftover,
    Unmanaged,
    OutsideRoots,
    JournalPending,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum TrashUnavailableReason {
    OtherDevice,
    NoUsableTrash,
    NotSupported,
    RecycleRefused,
    NotShellSafe,
}

impl From<&ModTarget> for core::ModTarget {
    fn from(target: &ModTarget) -> Self {
        Self {
            mod_ref: core::ModRef::from_token(target.mod_ref.clone()),
            key: target.key.clone(),
        }
    }
}

impl From<&core::ModTarget> for ModTarget {
    fn from(target: &core::ModTarget) -> Self {
        Self {
            mod_ref: target.mod_ref.as_str().to_owned(),
            key: target.key.clone(),
        }
    }
}

impl From<&RenameRequest> for core::RenameRequest {
    fn from(request: &RenameRequest) -> Self {
        Self {
            target: (&request.target).into(),
            new_name: request.new_name.clone(),
            apply_while_running: request.apply_while_running,
        }
    }
}

impl From<&core::RekeyReport> for RekeyReport {
    fn from(report: &core::RekeyReport) -> Self {
        Self {
            profiles: report.profiles,
            meta: report.meta,
            skipped_ambiguous: report.skipped_ambiguous,
            failed: report.failed,
        }
    }
}

impl From<&core::RenameOutcome> for RenameOutcome {
    fn from(outcome: &core::RenameOutcome) -> Self {
        Self {
            snapshot: ModsSnapshot::from(&*outcome.refreshed.snapshot),
            old_key: outcome.old_key.clone(),
            new_key: outcome.new_key.clone(),
            new_ref: outcome
                .new_ref
                .as_ref()
                .map(|mod_ref| mod_ref.as_str().to_owned()),
            rekey: RekeyReport::from(&outcome.rekey),
        }
    }
}

impl From<&TrashRequest> for core::TrashRequest {
    fn from(request: &TrashRequest) -> Self {
        Self {
            target: (&request.target).into(),
            apply_while_running: request.apply_while_running,
        }
    }
}

impl From<platform::TrashKind> for TrashKind {
    fn from(kind: platform::TrashKind) -> Self {
        match kind {
            platform::TrashKind::Home => Self::Home,
            platform::TrashKind::TopdirShared => Self::TopdirShared,
            platform::TrashKind::TopdirUser => Self::TopdirUser,
            platform::TrashKind::RecycleBin => Self::RecycleBin,
        }
    }
}

impl From<&core::TrashOutcome> for TrashOutcome {
    fn from(outcome: &core::TrashOutcome) -> Self {
        Self {
            snapshot: ModsSnapshot::from(&*outcome.refreshed.snapshot),
            key: outcome.key.clone(),
            items: outcome.items,
            kind: outcome.kind.into(),
        }
    }
}

impl From<&core::DeletePrepared> for DeletePrepared {
    fn from(prepared: &core::DeletePrepared) -> Self {
        Self {
            token: prepared.token.clone(),
            key: prepared.key.clone(),
            name: prepared.name.clone(),
            size: prepared.size.map(bytes),
            expires_in_secs: prepared.expires_in_secs,
        }
    }
}

impl From<&DeleteRequest> for core::DeleteRequest {
    fn from(request: &DeleteRequest) -> Self {
        Self {
            target: (&request.target).into(),
            token: request.token.clone(),
            apply_while_running: request.apply_while_running,
        }
    }
}

impl From<&core::DeleteOutcome> for DeleteOutcome {
    fn from(outcome: &core::DeleteOutcome) -> Self {
        Self {
            snapshot: ModsSnapshot::from(&*outcome.refreshed.snapshot),
            key: outcome.key.clone(),
            leftover: outcome.leftover.clone(),
        }
    }
}

impl From<&ModInfoEdit> for core::ModInfoEdit {
    fn from(info: &ModInfoEdit) -> Self {
        Self {
            version: info.version.clone(),
            category: info.category.clone(),
            description: info.description.clone(),
            author: info.author.clone(),
        }
    }
}

impl From<&MetadataRequest> for core::MetadataRequest {
    fn from(request: &MetadataRequest) -> Self {
        Self {
            target: (&request.target).into(),
            info: (&request.info).into(),
        }
    }
}

impl From<core::ManageAction> for ManageAction {
    fn from(action: core::ManageAction) -> Self {
        match action {
            core::ManageAction::Rename => Self::Rename,
            core::ManageAction::Trash => Self::Trash,
            core::ManageAction::Delete => Self::Delete,
            core::ManageAction::Metadata => Self::Metadata,
        }
    }
}

impl From<core::ManageBlock> for ManageBlock {
    fn from(block: core::ManageBlock) -> Self {
        match block {
            core::ManageBlock::Loose => Self::Loose,
            core::ManageBlock::Linked => Self::Linked,
            core::ManageBlock::Archive => Self::Archive,
            core::ManageBlock::Leftover => Self::Leftover,
            core::ManageBlock::Unmanaged => Self::Unmanaged,
            core::ManageBlock::OutsideRoots => Self::OutsideRoots,
            core::ManageBlock::JournalPending => Self::JournalPending,
        }
    }
}

impl From<platform::TrashUnavailableReason> for TrashUnavailableReason {
    fn from(reason: platform::TrashUnavailableReason) -> Self {
        match reason {
            platform::TrashUnavailableReason::OtherDevice => Self::OtherDevice,
            platform::TrashUnavailableReason::NoUsableTrash => Self::NoUsableTrash,
            platform::TrashUnavailableReason::NotSupported => Self::NotSupported,
            platform::TrashUnavailableReason::RecycleRefused => Self::RecycleRefused,
            platform::TrashUnavailableReason::NotShellSafe => Self::NotShellSafe,
        }
    }
}
