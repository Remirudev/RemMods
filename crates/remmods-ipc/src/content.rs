use remmods_core::content as core;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ContainerStatus {
    Read,
    NotIoStore,
    UnsupportedVersion,
    Encrypted,
    NoIndex,
    Malformed,
    TooLarge,
    Unreadable,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ContainerSummary {
    pub rel_path: String,
    /// `false` for `.utoc-x`.
    pub live: bool,
    pub status: ContainerStatus,
    pub version: Option<u32>,
    /// File entries; 0 unless `read`.
    pub files: u32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Slot {
    Upper,
    Lower,
    Bottom,
    FullBody,
    Hair,
    Head,
    FullHead,
    Hat,
    Face,
    FacePaint,
    FaceHair,
    EyeMakeup,
    Eyes,
    Cheek,
    Lips,
    Glasses,
    Accessory,
    Shoes,
    Extra,
}

/// A raw character code; `name` only for mappings with local evidence.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CharacterHint {
    pub code: String,
    pub name: Option<String>,
    pub common: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SlotHint {
    pub code: String,
    pub slot: Option<Slot>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ContentSummary {
    pub containers: Vec<ContainerSummary>,
    pub assets: u32,
    pub characters: Vec<CharacterHint>,
    pub slots: Vec<SlotHint>,
    pub internal_overlaps: u32,
    pub shared_identical: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ContentConflictLink {
    /// The other row's ModRef token.
    pub mod_ref: String,
    pub key: String,
    pub name: String,
    pub shared: u32,
    pub sample: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum AssetOrigin {
    Adds,
    Replaces,
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum BaseIndexState {
    Complete,
    Partial,
    Unavailable,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AssetEntry {
    pub path: String,
    pub container: String,
    pub live: bool,
    pub origin: AssetOrigin,
    pub character: Option<CharacterHint>,
    pub slot: Option<SlotHint>,
    /// ModRef tokens of the rows that ship the same asset.
    pub shared_with: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModContents {
    pub mod_ref: String,
    pub base: BaseIndexState,
    pub entries: Vec<AssetEntry>,
    pub total: u32,
    pub truncated: bool,
}

impl From<core::ContainerStatus> for ContainerStatus {
    fn from(status: core::ContainerStatus) -> Self {
        match status {
            core::ContainerStatus::Read => Self::Read,
            core::ContainerStatus::NotIoStore => Self::NotIoStore,
            core::ContainerStatus::UnsupportedVersion => Self::UnsupportedVersion,
            core::ContainerStatus::Encrypted => Self::Encrypted,
            core::ContainerStatus::NoIndex => Self::NoIndex,
            core::ContainerStatus::Malformed => Self::Malformed,
            core::ContainerStatus::TooLarge => Self::TooLarge,
            core::ContainerStatus::Unreadable => Self::Unreadable,
        }
    }
}

impl From<&core::ContainerSummary> for ContainerSummary {
    fn from(container: &core::ContainerSummary) -> Self {
        Self {
            rel_path: container.rel_path.clone(),
            live: container.live,
            status: container.status.into(),
            version: container.version.map(u32::from),
            files: container.files,
        }
    }
}

impl From<core::Slot> for Slot {
    fn from(slot: core::Slot) -> Self {
        match slot {
            core::Slot::Upper => Self::Upper,
            core::Slot::Lower => Self::Lower,
            core::Slot::Bottom => Self::Bottom,
            core::Slot::FullBody => Self::FullBody,
            core::Slot::Hair => Self::Hair,
            core::Slot::Head => Self::Head,
            core::Slot::FullHead => Self::FullHead,
            core::Slot::Hat => Self::Hat,
            core::Slot::Face => Self::Face,
            core::Slot::FacePaint => Self::FacePaint,
            core::Slot::FaceHair => Self::FaceHair,
            core::Slot::EyeMakeup => Self::EyeMakeup,
            core::Slot::Eyes => Self::Eyes,
            core::Slot::Cheek => Self::Cheek,
            core::Slot::Lips => Self::Lips,
            core::Slot::Glasses => Self::Glasses,
            core::Slot::Accessory => Self::Accessory,
            core::Slot::Shoes => Self::Shoes,
            core::Slot::Extra => Self::Extra,
        }
    }
}

impl CharacterHint {
    pub fn from_code(code: &str) -> Self {
        Self {
            code: code.to_owned(),
            name: core::character_name(code).map(str::to_owned),
            common: core::is_common(code),
        }
    }
}

impl SlotHint {
    pub fn from_code(code: &str) -> Self {
        Self {
            code: code.to_owned(),
            slot: core::slot(code).map(Slot::from),
        }
    }
}

impl From<&core::ContentSummary> for ContentSummary {
    fn from(summary: &core::ContentSummary) -> Self {
        Self {
            containers: summary
                .containers
                .iter()
                .map(ContainerSummary::from)
                .collect(),
            assets: summary.assets,
            characters: summary
                .characters
                .iter()
                .map(|code| CharacterHint::from_code(code))
                .collect(),
            slots: summary
                .slots
                .iter()
                .map(|code| SlotHint::from_code(code))
                .collect(),
            internal_overlaps: summary.internal_overlaps,
            shared_identical: summary.shared_identical,
        }
    }
}

impl From<&core::ContentConflictLink> for ContentConflictLink {
    fn from(link: &core::ContentConflictLink) -> Self {
        Self {
            mod_ref: link.other.as_str().to_owned(),
            key: link.key.clone(),
            name: link.name.clone(),
            shared: link.shared,
            sample: link.sample.clone(),
        }
    }
}

impl From<core::AssetOrigin> for AssetOrigin {
    fn from(origin: core::AssetOrigin) -> Self {
        match origin {
            core::AssetOrigin::Adds => Self::Adds,
            core::AssetOrigin::Replaces => Self::Replaces,
            core::AssetOrigin::Unknown => Self::Unknown,
        }
    }
}

impl From<core::BaseIndexState> for BaseIndexState {
    fn from(state: core::BaseIndexState) -> Self {
        match state {
            core::BaseIndexState::Complete => Self::Complete,
            core::BaseIndexState::Partial => Self::Partial,
            core::BaseIndexState::Unavailable => Self::Unavailable,
        }
    }
}

impl From<&core::AssetEntry> for AssetEntry {
    fn from(entry: &core::AssetEntry) -> Self {
        Self {
            path: entry.path.clone(),
            container: entry.container.clone(),
            live: entry.live,
            origin: entry.origin.into(),
            character: entry.character.as_deref().map(CharacterHint::from_code),
            slot: entry.slot.as_deref().map(SlotHint::from_code),
            shared_with: entry
                .shared_with
                .iter()
                .map(|other| other.as_str().to_owned())
                .collect(),
        }
    }
}

impl From<&core::ModContents> for ModContents {
    fn from(contents: &core::ModContents) -> Self {
        Self {
            mod_ref: contents.mod_ref.as_str().to_owned(),
            base: contents.base.into(),
            entries: contents.entries.iter().map(AssetEntry::from).collect(),
            total: contents.total,
            truncated: contents.truncated,
        }
    }
}
