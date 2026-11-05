use remmods_core::mods as core;
use serde::{Deserialize, Serialize};

use crate::manage::ModTarget;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum RepairKind {
    Leftover,
    AmbiguousKey,
    InvalidName,
    NonUnicodeName,
    LogicDisabledButLoaded,
    LogicEnabledNotLoaded,
    TempFolderLive,
    StuckJournal,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum RepairAction {
    Rename,
    Trash,
    DeletePermanently,
    Reveal,
    FinishDisable,
    FinishEnable,
    OpenLogs,
}

/// `id` is stable within one snapshot; `suggestedName` (invalid or non-Unicode names) already
/// passes the name rules and collides with nothing.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RepairItem {
    pub id: String,
    pub kind: RepairKind,
    pub target: Option<ModTarget>,
    pub label: String,
    pub actions: Vec<RepairAction>,
    pub suggested_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RepairList {
    pub revision: u32,
    pub items: Vec<RepairItem>,
}

impl From<core::RepairKind> for RepairKind {
    fn from(kind: core::RepairKind) -> Self {
        match kind {
            core::RepairKind::Leftover => Self::Leftover,
            core::RepairKind::AmbiguousKey => Self::AmbiguousKey,
            core::RepairKind::InvalidName => Self::InvalidName,
            core::RepairKind::NonUnicodeName => Self::NonUnicodeName,
            core::RepairKind::LogicDisabledButLoaded => Self::LogicDisabledButLoaded,
            core::RepairKind::LogicEnabledNotLoaded => Self::LogicEnabledNotLoaded,
            core::RepairKind::TempFolderLive => Self::TempFolderLive,
            core::RepairKind::StuckJournal => Self::StuckJournal,
        }
    }
}

impl From<core::RepairAction> for RepairAction {
    fn from(action: core::RepairAction) -> Self {
        match action {
            core::RepairAction::Rename => Self::Rename,
            core::RepairAction::Trash => Self::Trash,
            core::RepairAction::DeletePermanently => Self::DeletePermanently,
            core::RepairAction::Reveal => Self::Reveal,
            core::RepairAction::FinishDisable => Self::FinishDisable,
            core::RepairAction::FinishEnable => Self::FinishEnable,
            core::RepairAction::OpenLogs => Self::OpenLogs,
        }
    }
}

impl From<&core::RepairItem> for RepairItem {
    fn from(item: &core::RepairItem) -> Self {
        Self {
            id: item.id.clone(),
            kind: item.kind.into(),
            target: item.target.as_ref().map(ModTarget::from),
            label: item.label.clone(),
            actions: item
                .actions
                .iter()
                .copied()
                .map(RepairAction::from)
                .collect(),
            suggested_name: item.suggested_name.clone(),
        }
    }
}

impl From<&core::RepairList> for RepairList {
    fn from(list: &core::RepairList) -> Self {
        Self {
            revision: list.revision,
            items: list.items.iter().map(RepairItem::from).collect(),
        }
    }
}
