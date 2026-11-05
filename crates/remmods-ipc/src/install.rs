//! Install DTOs (SPEC-P4 §6.1). Paths never cross IPC: sources are registry ids or archive-row
//! tokens, and every byte count is an `f64`.

use serde::{Deserialize, Serialize};

use crate::mods::{ModsSnapshot, RootKind};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum SourceKind {
    Zip,
    SevenZip,
    Rar,
    Folder,
    Image,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum SourceReject {
    UnsupportedType,
    Missing,
    Link,
    InsidePaks,
    TooMany,
}

/// A picked or dropped source the shell registered; `id` is `src-` + 32 hex.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PickedSource {
    pub id: String,
    pub name: String,
    pub kind: SourceKind,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RejectedSource {
    pub name: String,
    pub reason: SourceReject,
}

/// `token`: a registry id from a picker or drop; `archive`: the ModRef of an archive row inside
/// Paks.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum InstallSourceRef {
    Token { id: String },
    Archive { row: String },
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum NameEncoding {
    Auto,
    ShiftJis,
    Gbk,
    Big5,
    EucKr,
    Ibm866,
    Windows1251,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ArchiveFormat {
    Zip,
    SevenZip,
    Rar,
    Folder,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum RootReason {
    PathSegment,
    ConfigLua,
    Default,
    Chosen,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum IssueSeverity {
    Block,
    Warn,
    Info,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum InstallIssueCode {
    UnsafePath,
    Link,
    Special,
    AntiItem,
    DuplicateIdentical,
    DuplicateDiffers,
    DuplicateRaw,
    CaseClash,
    InvalidName,
    NotPortable,
    Undecodable,
    LegacyNames,
    NestedArchive,
    ShipsDisabled,
    ShipsBothStates,
    ScriptMod,
    NoPayload,
    MixedRoots,
    OutsideRoot,
    ExtraFiles,
    PathTooLong,
    AlsoInRoot,
    Ambiguous,
    Replaces,
    NoSpace,
    EmptyDirs,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstallIssue {
    pub severity: IssueSeverity,
    pub code: InstallIssueCode,
    pub path: String,
    pub count: u32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ExistingRelation {
    SameName,
    FoldEqual,
    DisabledPrefix,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExistingMatch {
    pub mod_ref: String,
    pub key: String,
    pub rel: String,
    pub relation: ExistingRelation,
    pub ambiguous: bool,
}

/// One folder the source would install. `nameIssue` is the name-rule code (e.g.
/// `reserved_device_name`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstallCandidate {
    pub id: u32,
    pub name: String,
    pub inner: String,
    pub files: u32,
    pub payload: u32,
    pub shipped_off: u32,
    #[specta(type = specta_typescript::Number)]
    pub bytes: f64,
    pub group: Option<u32>,
    pub selected: bool,
    pub name_issue: Option<String>,
    pub fixed_name: Option<String>,
    pub existing: Option<ExistingMatch>,
    pub also_in: Vec<RootKind>,
    pub path_too_long: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VariantGroup {
    pub id: u32,
    pub candidates: Vec<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CaseClash {
    pub id: u32,
    pub paths: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NameOverride {
    pub id: u32,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InspectRequest {
    pub source: InstallSourceRef,
    pub encoding: NameEncoding,
    pub root: Option<RootKind>,
    pub names: Vec<NameOverride>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    pub id: String,
    pub source_name: String,
    pub format: ArchiveFormat,
    pub encoding: NameEncoding,
    pub legacy_names: bool,
    pub entries: u32,
    pub files: u32,
    #[specta(type = specta_typescript::Number)]
    pub total_bytes: f64,
    pub detected_root: RootKind,
    pub root_reason: RootReason,
    pub root: RootKind,
    pub candidates: Vec<InstallCandidate>,
    pub groups: Vec<VariantGroup>,
    pub clashes: Vec<CaseClash>,
    pub issues: Vec<InstallIssue>,
    pub blocked: bool,
    #[specta(type = Option<specta_typescript::Number>)]
    pub available_bytes: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CandidatePick {
    pub id: u32,
    pub replace: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ClashPick {
    pub id: u32,
    pub keep: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StartRequest {
    pub inspection: String,
    pub picks: Vec<CandidatePick>,
    pub clashes: Vec<ClashPick>,
    pub auto_fix: bool,
    pub apply_while_running: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum InstallPhase {
    Preparing,
    Extracting,
    Committing,
    CleaningUp,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub phase: InstallPhase,
    #[specta(type = specta_typescript::Number)]
    pub done_bytes: f64,
    #[specta(type = specta_typescript::Number)]
    pub total_bytes: f64,
    pub done_files: u32,
    pub total_files: u32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum KeptReason {
    TrashUnavailable,
    TrashFailed,
}

/// What happened to the version a replace moved aside.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ReplacedOld {
    Trashed,
    KeptInRepair { reason: KeptReason },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstalledMod {
    pub key: String,
    pub mod_ref: Option<String>,
    pub rel: String,
    pub files: u32,
    pub replaced: Option<ReplacedOld>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstallOutcome {
    pub snapshot: ModsSnapshot,
    pub installed: Vec<InstalledMod>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum DropPhase {
    Enter,
    Leave,
    Dropped,
}

/// Why core refused an archive (`AppError::ArchiveFailed`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ArchiveProblem {
    NotAnArchive,
    MultiVolume,
    SelfExtracting,
    UnsupportedMethod,
    RarDisabled,
    Encrypted,
    Corrupt,
    MemoryLimit,
    TooManyEntries,
    TooLarge,
}

/// Why a start was refused before anything was written (`AppError::InstallBlocked`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum InstallBlock {
    Issues,
    InvalidName,
    UnresolvedClash,
    NoSelection,
    VariantConflict,
    ReplaceNotConfirmed,
    ReplaceMismatch,
    Ambiguous,
    TargetChanged,
}
