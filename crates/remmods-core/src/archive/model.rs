use std::time::SystemTime;

use super::error::ArchiveError;
use super::memory;
use super::progress::{CancelToken, ProgressCounter};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveFormat {
    Zip,
    SevenZip,
    Rar,
    Folder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Dir,
    Link,
    Special,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RawEntryName {
    /// The format stored Unicode (7z, RAR5, RAR 1.5–4 with a Unicode name, folder names that are
    /// valid Unicode). Never re-decoded by an override.
    Unicode(String),
    /// Legacy bytes (zip names, RAR ≤ 4 without Unicode, non-UTF-8 folder names on Linux).
    Bytes(Vec<u8>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawEntry {
    /// Backend order, stable; the extract plan refers to it.
    pub index: u32,
    pub name: RawEntryName,
    pub kind: EntryKind,
    /// Declared uncompressed size; 0 for Dir/Link/Special.
    pub size: u64,
    pub crc32: Option<u32>,
    pub mtime: Option<SystemTime>,
    pub encrypted: bool,
    /// 7z anti-item.
    pub anti: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listing {
    pub format: ArchiveFormat,
    pub entries: Vec<RawEntry>,
    /// zip `has_duplicate_names()`; false elsewhere.
    pub duplicate_raw: bool,
    pub solid: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NameEncoding {
    #[default]
    Auto,
    ShiftJis,
    Gbk,
    Big5,
    EucKr,
    Ibm866,
    Windows1251,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryProblem {
    Traversal,
    Absolute,
    Drive,
    Unc,
    DotsOnly,
    Nul,
    Empty,
    TooDeep,
    TooLong,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub index: u32,
    /// Normalized components; empty when `problem` is set.
    pub path: Vec<String>,
    /// The decoded name before normalization, for issue lists.
    pub display: String,
    pub kind: EntryKind,
    pub size: u64,
    pub crc32: Option<u32>,
    pub mtime: Option<SystemTime>,
    /// Decoded through CP437 or the override.
    pub legacy: bool,
    /// The override could not decode it (or a 7z/folder name had U+FFFD).
    pub undecodable: bool,
    pub problem: Option<EntryProblem>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_entries: u32,
    pub max_total: u64,
    pub max_path_bytes: u32,
    pub max_depth: u32,
    pub memory: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_entries: 65_536,
            max_total: 64 << 30,
            max_path_bytes: 4096,
            max_depth: 32,
            memory: memory::memory_limit(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceFingerprint {
    pub len: u64,
    pub modified: Option<SystemTime>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedFile {
    /// `RawEntry::index`.
    pub index: u32,
    /// Which staging folder (install candidate id).
    pub slot: u32,
    /// Components below that staging folder, final staged names (incl. `-x`).
    pub dest: Vec<String>,
    pub size: u64,
    pub mtime: Option<SystemTime>,
}

/// Sorted by index; an index appears at most once.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExtractPlan {
    pub files: Vec<PlannedFile>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExtractReport {
    pub files: u32,
    pub bytes: u64,
}

/// Where extracted bytes go; install implements it on top of `safe_fs::StagingDir`.
pub trait ExtractTarget: Send {
    fn create(&mut self, file: &PlannedFile) -> std::io::Result<Box<dyn StagedWrite>>;
}

/// One file being written; `finish` sets the mtime and syncs. Dropped without `finish` = failed.
pub trait StagedWrite: std::io::Write + Send {
    fn finish(self: Box<Self>, mtime: Option<SystemTime>) -> std::io::Result<()>;
}

pub trait ArchiveReader: Send {
    fn listing(&self) -> &Listing;
    /// Writes exactly the planned files; entries not in the plan are skipped (7z solid blocks
    /// still decode them, the bytes go nowhere). Returns only after every created file was
    /// finished with exactly its declared size.
    fn extract(
        &mut self,
        plan: &ExtractPlan,
        target: &mut dyn ExtractTarget,
        progress: &ProgressCounter,
        cancel: &CancelToken,
    ) -> Result<ExtractReport, ArchiveError>;
}

/// Result of [`super::decode_name`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedName {
    pub text: String,
    pub legacy: bool,
    pub undecodable: bool,
}
