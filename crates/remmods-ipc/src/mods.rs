use remmods_core::mods as core;
use remmods_core::paths::display_path;
use serde::{Deserialize, Serialize};

use crate::content::{ContentConflictLink, ContentSummary};
use crate::error::OsErrorClass;
use crate::mod_issues::ModIssue;
use crate::toggle::ToggleBlock;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ModKind {
    Standard,
    Legacy,
    Logic,
    Loose,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ModState {
    Enabled,
    Partial,
    Disabled,
    NoPakFiles,
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum RootKind {
    Standard,
    Legacy,
    Logic,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PayloadCounts {
    pub live_pak: u32,
    pub off_pak: u32,
    pub live_companion: u32,
    pub off_companion: u32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ModInfoOrigin {
    File,
    Missing,
    Invalid,
}

/// `true` = the field holds the default, which the UI replaces by a translated placeholder.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModInfoDefaulted {
    pub version: bool,
    pub category: bool,
    pub description: bool,
    pub author: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModInfo {
    pub version: String,
    pub category: String,
    pub description: String,
    pub author: String,
    pub origin: ModInfoOrigin,
    pub defaulted: ModInfoDefaulted,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConflictLink {
    /// The other row's ModRef token.
    pub mod_ref: String,
    pub key: String,
    pub name: String,
    pub paks: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ScanProblem {
    /// `list`, `read` or `stat`.
    pub op: String,
    pub class: OsErrorClass,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModRow {
    pub mod_ref: String,
    pub key: String,
    pub kind: ModKind,
    pub name: String,
    pub folder_name: String,
    pub root: String,
    pub location: String,
    pub state: ModState,
    pub logic_prefixed: bool,
    pub payload: PayloadCounts,
    /// `null` when the folder could not be read completely.
    #[specta(type = Option<specta_typescript::Number>)]
    pub size_bytes: Option<f64>,
    pub info: ModInfo,
    pub conflicts_with: Vec<ConflictLink>,
    pub content: ContentSummary,
    /// Rows that ship the same assets in live containers; independent of `conflicts_with`.
    pub content_conflicts: Vec<ContentConflictLink>,
    pub ambiguous: bool,
    pub issues: Vec<ModIssue>,
    /// Set when the row cannot be toggled; core re-checks it on the fresh scan.
    pub toggle_block: Option<ToggleBlock>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RootInfo {
    pub kind: RootKind,
    pub name: String,
    pub is_link: bool,
    pub case_variant: bool,
    pub unreadable: Option<ScanProblem>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveEntry {
    pub mod_ref: String,
    pub rel_path: String,
    #[specta(type = specta_typescript::Number)]
    pub size_bytes: f64,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum UnmanagedKind {
    PaksFile,
    Folder,
    LinkedFolder,
    TempFolder,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UnmanagedEntry {
    pub mod_ref: String,
    pub kind: UnmanagedKind,
    pub rel_path: String,
    /// Always 0 for a linked folder, which is not inspected.
    pub live_files: u32,
    /// Names of the rows whose files this entry duplicates.
    pub copy_of: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LeftoverEntry {
    pub mod_ref: String,
    pub rel_path: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ScanStats {
    pub dirs_read: u32,
    pub entries_seen: u32,
    #[specta(type = specta_typescript::Number)]
    pub duration_ms: f64,
    /// `.utoc` indexes read in this scan; unchanged files come from the cache.
    pub containers_read: u32,
    pub containers_cached: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModsSnapshot {
    pub revision: u32,
    /// RFC 3339 in UTC with milliseconds.
    pub scanned_at: String,
    pub paks: String,
    pub roots: Vec<RootInfo>,
    pub rows: Vec<ModRow>,
    pub archives: Vec<ArchiveEntry>,
    pub unmanaged: Vec<UnmanagedEntry>,
    pub leftovers: Vec<LeftoverEntry>,
    pub stats: ScanStats,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum PayloadKind {
    Pak,
    Ucas,
    Utoc,
    Sig,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PayloadFile {
    pub rel_path: String,
    pub kind: PayloadKind,
    pub live: bool,
    #[specta(type = specta_typescript::Number)]
    pub size_bytes: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModDetails {
    pub mod_ref: String,
    pub location: String,
    pub files: Vec<PayloadFile>,
    pub truncated: bool,
    pub other_files: u32,
}

impl From<core::ModKind> for ModKind {
    fn from(kind: core::ModKind) -> Self {
        match kind {
            core::ModKind::Standard => Self::Standard,
            core::ModKind::Legacy => Self::Legacy,
            core::ModKind::Logic => Self::Logic,
            core::ModKind::Loose => Self::Loose,
        }
    }
}

impl From<core::ModState> for ModState {
    fn from(state: core::ModState) -> Self {
        match state {
            core::ModState::Enabled => Self::Enabled,
            core::ModState::Partial => Self::Partial,
            core::ModState::Disabled => Self::Disabled,
            core::ModState::NoPakFiles => Self::NoPakFiles,
            core::ModState::Unknown => Self::Unknown,
        }
    }
}

impl From<core::RootKind> for RootKind {
    fn from(kind: core::RootKind) -> Self {
        match kind {
            core::RootKind::Standard => Self::Standard,
            core::RootKind::Legacy => Self::Legacy,
            core::RootKind::Logic => Self::Logic,
        }
    }
}

impl From<core::PayloadCounts> for PayloadCounts {
    fn from(counts: core::PayloadCounts) -> Self {
        Self {
            live_pak: counts.live_pak,
            off_pak: counts.off_pak,
            live_companion: counts.live_companion,
            off_companion: counts.off_companion,
        }
    }
}

impl From<core::ModInfoOrigin> for ModInfoOrigin {
    fn from(origin: core::ModInfoOrigin) -> Self {
        match origin {
            core::ModInfoOrigin::File => Self::File,
            core::ModInfoOrigin::Missing => Self::Missing,
            core::ModInfoOrigin::Invalid => Self::Invalid,
        }
    }
}

impl From<core::ModInfoDefaulted> for ModInfoDefaulted {
    fn from(defaulted: core::ModInfoDefaulted) -> Self {
        Self {
            version: defaulted.version,
            category: defaulted.category,
            description: defaulted.description,
            author: defaulted.author,
        }
    }
}

impl From<&core::ModInfo> for ModInfo {
    fn from(info: &core::ModInfo) -> Self {
        Self {
            version: info.version.clone(),
            category: info.category.clone(),
            description: info.description.clone(),
            author: info.author.clone(),
            origin: info.origin.into(),
            defaulted: info.defaulted.into(),
        }
    }
}

impl From<&core::ConflictLink> for ConflictLink {
    fn from(link: &core::ConflictLink) -> Self {
        Self {
            mod_ref: link.other.as_str().to_owned(),
            key: link.key.clone(),
            name: link.name.clone(),
            paks: link.paks.clone(),
        }
    }
}

impl From<core::ScanProblem> for ScanProblem {
    fn from(problem: core::ScanProblem) -> Self {
        Self {
            op: problem.op.to_owned(),
            class: problem.class.into(),
        }
    }
}

impl From<&core::ModRow> for ModRow {
    fn from(row: &core::ModRow) -> Self {
        Self {
            mod_ref: row.mod_ref.as_str().to_owned(),
            key: row.key.clone(),
            kind: row.kind.into(),
            name: row.name.clone(),
            folder_name: row.folder_name.clone(),
            root: row.root.clone(),
            location: display_path(&row.path),
            state: row.state.into(),
            logic_prefixed: row.logic_prefixed,
            payload: row.payload.into(),
            size_bytes: row.size.map(bytes),
            info: (&row.info).into(),
            conflicts_with: row.conflicts_with.iter().map(ConflictLink::from).collect(),
            content: (&row.content).into(),
            content_conflicts: row
                .content_conflicts
                .iter()
                .map(ContentConflictLink::from)
                .collect(),
            ambiguous: row.ambiguous,
            issues: row.issues.iter().map(ModIssue::from).collect(),
            toggle_block: row.toggle_block.map(ToggleBlock::from),
        }
    }
}

impl From<&core::RootInfo> for RootInfo {
    fn from(root: &core::RootInfo) -> Self {
        Self {
            kind: root.kind.into(),
            name: root.name.clone(),
            is_link: root.is_link,
            case_variant: root.case_variant,
            unreadable: root.unreadable.map(ScanProblem::from),
        }
    }
}

impl From<&core::ArchiveEntry> for ArchiveEntry {
    fn from(entry: &core::ArchiveEntry) -> Self {
        Self {
            mod_ref: entry.mod_ref.as_str().to_owned(),
            rel_path: entry.rel_path.clone(),
            size_bytes: bytes(entry.size),
        }
    }
}

impl From<core::UnmanagedKind> for UnmanagedKind {
    fn from(kind: core::UnmanagedKind) -> Self {
        match kind {
            core::UnmanagedKind::PaksFile => Self::PaksFile,
            core::UnmanagedKind::Folder => Self::Folder,
            core::UnmanagedKind::LinkedFolder => Self::LinkedFolder,
            core::UnmanagedKind::TempFolder => Self::TempFolder,
        }
    }
}

impl From<&core::UnmanagedEntry> for UnmanagedEntry {
    fn from(entry: &core::UnmanagedEntry) -> Self {
        Self {
            mod_ref: entry.mod_ref.as_str().to_owned(),
            kind: entry.kind.into(),
            rel_path: entry.rel_path.clone(),
            live_files: entry.live_files,
            copy_of: entry.copy_of.clone(),
        }
    }
}

impl From<&core::LeftoverEntry> for LeftoverEntry {
    fn from(entry: &core::LeftoverEntry) -> Self {
        Self {
            mod_ref: entry.mod_ref.as_str().to_owned(),
            rel_path: entry.rel_path.clone(),
        }
    }
}

impl From<core::ScanStats> for ScanStats {
    fn from(stats: core::ScanStats) -> Self {
        Self {
            dirs_read: stats.dirs_read,
            entries_seen: stats.entries_seen,
            duration_ms: stats.duration.as_nanos() as f64 / NANOS_PER_MILLI,
            containers_read: stats.containers_read,
            containers_cached: stats.containers_cached,
        }
    }
}

impl From<&core::ScanSnapshot> for ModsSnapshot {
    fn from(snapshot: &core::ScanSnapshot) -> Self {
        let content = &snapshot.content;
        Self {
            revision: snapshot.revision,
            scanned_at: snapshot.scanned_at_rfc3339(),
            paks: display_path(&content.paks),
            roots: content.roots.iter().map(RootInfo::from).collect(),
            rows: content.rows.iter().map(ModRow::from).collect(),
            archives: content.archives.iter().map(ArchiveEntry::from).collect(),
            unmanaged: content.unmanaged.iter().map(UnmanagedEntry::from).collect(),
            leftovers: content.leftovers.iter().map(LeftoverEntry::from).collect(),
            stats: snapshot.stats.into(),
        }
    }
}

impl From<core::PayloadKind> for PayloadKind {
    fn from(kind: core::PayloadKind) -> Self {
        match kind {
            core::PayloadKind::Pak => Self::Pak,
            core::PayloadKind::Ucas => Self::Ucas,
            core::PayloadKind::Utoc => Self::Utoc,
            core::PayloadKind::Sig => Self::Sig,
        }
    }
}

impl From<&core::PayloadFile> for PayloadFile {
    fn from(file: &core::PayloadFile) -> Self {
        Self {
            rel_path: file.rel_path.clone(),
            kind: file.kind.into(),
            live: file.live,
            size_bytes: bytes(file.size),
        }
    }
}

impl From<&core::ModDetails> for ModDetails {
    fn from(details: &core::ModDetails) -> Self {
        Self {
            mod_ref: details.mod_ref.as_str().to_owned(),
            location: display_path(&details.location),
            files: details.files.iter().map(PayloadFile::from).collect(),
            truncated: details.truncated,
            other_files: details.other_files,
        }
    }
}

const NANOS_PER_MILLI: f64 = 1_000_000.0;

/// Sizes cross IPC as JavaScript numbers; exact up to 2^53 bytes (8 PiB).
fn bytes(size: u64) -> f64 {
    size as f64
}
