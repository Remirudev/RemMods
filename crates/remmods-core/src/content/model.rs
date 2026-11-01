use std::path::PathBuf;
use std::time::SystemTime;

use crate::mods::ModRef;

/// Most asset entries `Core::mods_contents` returns.
pub const CONTENTS_MAX_ENTRIES: usize = 5000;
/// Most display paths a content-conflict link carries.
pub const CONFLICT_SAMPLE: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainerSummary {
    /// Like `PayloadFile.rel_path`: below the mod folder; loose rows: the file name.
    pub rel_path: String,
    /// `false` for `.utoc-x`.
    pub live: bool,
    pub status: ContainerStatus,
    /// When the header parsed (or named an unsupported version).
    pub version: Option<u8>,
    /// File entries; 0 unless `Read`.
    pub files: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContentSummary {
    /// Sorted by `rel_path`.
    pub containers: Vec<ContainerSummary>,
    /// Distinct asset keys over Read containers, live and off.
    pub assets: u32,
    /// Distinct raw character codes, sorted.
    pub characters: Vec<String>,
    /// Distinct raw slot codes, sorted.
    pub slots: Vec<String>,
    /// Asset keys shipped by at least two live Read containers of this row.
    pub internal_overlaps: u32,
    /// Asset keys this row shares with other enabled rows only as identical copies.
    pub shared_identical: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentConflictLink {
    pub other: ModRef,
    pub key: String,
    pub name: String,
    /// Asset keys both rows ship in live Read containers with different content (or unknown).
    pub shared: u32,
    /// At most `CONFLICT_SAMPLE` display paths of this row, sorted by asset key.
    pub sample: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetOrigin {
    Adds,
    Replaces,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaseIndexState {
    Complete,
    Partial,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetEntry {
    /// Display path.
    pub path: String,
    /// `ContainerSummary.rel_path`.
    pub container: String,
    pub live: bool,
    pub origin: AssetOrigin,
    /// Raw code.
    pub character: Option<String>,
    /// Raw code.
    pub slot: Option<String>,
    /// Rows of this row's `content_conflicts` that ship a different version of the asset key
    /// (live entries only).
    pub shared_with: Vec<ModRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModContents {
    pub mod_ref: ModRef,
    pub base: BaseIndexState,
    /// Sorted by (path, container), at most `CONTENTS_MAX_ENTRIES`.
    pub entries: Vec<AssetEntry>,
    pub total: u32,
    pub truncated: bool,
}

/// A `.utoc` / `.utoc-x` found by the scan, with the metadata the walk already read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UtocRef {
    /// Fs form.
    pub path: PathBuf,
    pub rel_path: String,
    pub live: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
}
