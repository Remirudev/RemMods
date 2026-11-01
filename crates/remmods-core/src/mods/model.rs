use std::path::PathBuf;
use std::time::Duration;

use chrono::{DateTime, SecondsFormat, Utc};

use super::issues::ModIssue;
use super::modref::ModRef;
use crate::content::{ContentConflictLink, ContentSummary};
use crate::platform::OsErrorClass;

/// Most payload files `read_details` lists.
pub const DETAILS_MAX_FILES: usize = 1000;

/// The three mod folders the game loads from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RootKind {
    Standard,
    Legacy,
    Logic,
}

impl RootKind {
    pub const ALL: [RootKind; 3] = [Self::Standard, Self::Legacy, Self::Logic];

    pub fn folder(self) -> &'static str {
        match self {
            Self::Standard => "Mods",
            Self::Legacy => "~mods",
            Self::Logic => "LogicMods",
        }
    }

    pub fn key_prefix(self) -> &'static str {
        match self {
            Self::Standard => "Standard",
            Self::Legacy => "Legacy",
            Self::Logic => "Logic",
        }
    }
}

/// Row kind; `Loose` rows are payload files lying directly in a root (key prefix `Loose`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ModKind {
    Standard,
    Legacy,
    Logic,
    Loose,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModState {
    Enabled,
    Partial,
    Disabled,
    NoPakFiles,
    Unknown,
}

/// Why a row cannot be toggled; the first matching reason wins (spec §5.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToggleBlock {
    Ambiguous,
    Linked,
    UnknownState,
    NoPakFiles,
    InvalidLogicName,
}

/// Payload files of a row: live = loaded by the game, off = renamed with `-x`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PayloadCounts {
    pub live_pak: u32,
    pub off_pak: u32,
    pub live_companion: u32,
    pub off_companion: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModInfoOrigin {
    File,
    Missing,
    Invalid,
}

/// `true` = the field holds its default ("1.0", "Misc", "", "Unknown").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ModInfoDefaulted {
    pub version: bool,
    pub category: bool,
    pub description: bool,
    pub author: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModInfo {
    pub version: String,
    pub category: String,
    pub description: String,
    pub author: String,
    pub origin: ModInfoOrigin,
    pub defaulted: ModInfoDefaulted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictLink {
    pub other: ModRef,
    pub key: String,
    pub name: String,
    pub paks: Vec<String>,
}

/// A read that failed during a scan; `op` is "list", "read" or "stat".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScanProblem {
    pub op: &'static str,
    pub class: OsErrorClass,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModRow {
    pub mod_ref: ModRef,
    /// `"{prefix}:{key text}"`; loose rows `"Loose:{root folder}/{stem key text}"`.
    pub key: String,
    pub kind: ModKind,
    /// Display form of the logical key text (Logic rows without `.disabled_`).
    pub name: String,
    /// Display form of the on-disk key text; loose rows: the stem.
    pub folder_name: String,
    /// Display form of the root folder as spelled on disk.
    pub root: String,
    /// Mod folder (loose rows: the root folder), mod-side form.
    pub path: PathBuf,
    pub state: ModState,
    pub logic_prefixed: bool,
    pub payload: PayloadCounts,
    /// `None` when a read inside the mod failed.
    pub size: Option<u64>,
    pub info: ModInfo,
    pub conflicts_with: Vec<ConflictLink>,
    /// What the row's `.utoc` indexes list (P2b).
    pub content: ContentSummary,
    /// Rows shipping the same assets in live containers; independent of `conflicts_with`.
    pub content_conflicts: Vec<ContentConflictLink>,
    pub ambiguous: bool,
    /// In `ModIssue` declaration order.
    pub issues: Vec<ModIssue>,
    /// `None` = toggleable.
    pub toggle_block: Option<ToggleBlock>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootInfo {
    pub kind: RootKind,
    pub name: String,
    /// Mod-side form.
    pub path: PathBuf,
    pub is_link: bool,
    pub case_variant: bool,
    pub unreadable: Option<ScanProblem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchiveEntry {
    pub mod_ref: ModRef,
    pub rel_path: String,
    pub size: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum UnmanagedKind {
    PaksFile,
    Folder,
    LinkedFolder,
    TempFolder,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnmanagedEntry {
    pub mod_ref: ModRef,
    pub kind: UnmanagedKind,
    pub rel_path: String,
    /// Live payload files found; `LinkedFolder` entries are not inspected and always hold 0.
    pub live_files: u32,
    /// Names of the rows whose payload this entry duplicates.
    pub copy_of: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeftoverEntry {
    pub mod_ref: ModRef,
    pub rel_path: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScanStats {
    pub dirs_read: u32,
    pub entries_seen: u32,
    pub duration: Duration,
    /// `.utoc` indexes read in this scan (cache misses).
    pub containers_read: u32,
    pub containers_cached: u32,
    pub content_duration: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanContent {
    /// Equals `ScanInput.paks` (game-side, plain form).
    pub paks: PathBuf,
    pub paks_id: String,
    pub roots: Vec<RootInfo>,
    pub rows: Vec<ModRow>,
    pub archives: Vec<ArchiveEntry>,
    pub unmanaged: Vec<UnmanagedEntry>,
    pub leftovers: Vec<LeftoverEntry>,
}

#[derive(Clone, Debug)]
pub struct ScanSnapshot {
    /// 1 for the first published snapshot.
    pub revision: u32,
    pub scanned_at: DateTime<Utc>,
    pub stats: ScanStats,
    pub content: ScanContent,
}

impl ScanSnapshot {
    /// e.g. "1970-01-01T00:00:01.500Z".
    pub fn scanned_at_rfc3339(&self) -> String {
        self.scanned_at.to_rfc3339_opts(SecondsFormat::Millis, true)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanInput {
    /// Plain absolute Paks path (game side).
    pub paks: PathBuf,
    /// UTF-16 length of Paks as the game sees it (W1 warning).
    pub game_view_len: usize,
    /// `paks_identity(&paks, location generation)`.
    pub paks_id: String,
}

#[derive(Clone, Debug)]
pub struct ScanOutput {
    pub content: ScanContent,
    pub stats: ScanStats,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayloadKind {
    Pak,
    Ucas,
    Utoc,
    Sig,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PayloadFile {
    pub rel_path: String,
    pub kind: PayloadKind,
    pub live: bool,
    pub size: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModDetails {
    pub mod_ref: ModRef,
    /// Mod-side form.
    pub location: PathBuf,
    pub files: Vec<PayloadFile>,
    pub truncated: bool,
    pub other_files: u32,
}
