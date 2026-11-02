//! Install types (spec §5.2): what the preview shows and what a start asks for.

use std::path::PathBuf;

use crate::archive::{ArchiveFormat, NameEncoding};
use crate::mods::{ModRef, RootKind};
use crate::safe_fs::names::NameIssue;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstallSource {
    Path(PathBuf),
    ArchiveRow { mod_ref: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    Zip,
    SevenZip,
    Rar,
    Folder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceReject {
    UnsupportedType,
    Missing,
    Link,
    InsidePaks,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameOverride {
    pub id: u32,
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InspectOptions {
    pub encoding: NameEncoding,
    pub root: Option<RootKind>,
    pub names: Vec<NameOverride>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootReason {
    PathSegment,
    ConfigLua,
    Default,
    Chosen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IssueSeverity {
    Block,
    Warn,
    Info,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IssueCode {
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallIssue {
    pub severity: IssueSeverity,
    pub code: IssueCode,
    pub path: String,
    pub count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExistingRelation {
    SameName,
    FoldEqual,
    DisabledPrefix,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExistingMatch {
    pub mod_ref: ModRef,
    pub key: String,
    pub rel: String,
    pub relation: ExistingRelation,
    pub ambiguous: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// Stable for the same source + encoding + root.
    pub id: u32,
    /// Proposed (or overridden) folder name.
    pub name: String,
    /// `/`-joined path inside the source, "" = top level.
    pub inner: String,
    pub files: u32,
    pub payload: u32,
    pub shipped_off: u32,
    pub bytes: u64,
    pub group: Option<u32>,
    pub selected: bool,
    pub name_issue: Option<NameIssue>,
    pub fixed_name: Option<String>,
    pub existing: Option<ExistingMatch>,
    pub also_in: Vec<RootKind>,
    pub path_too_long: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariantGroup {
    pub id: u32,
    pub candidates: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseClash {
    pub id: u32,
    pub paths: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inspection {
    /// "insp-" + 32 hex.
    pub id: String,
    /// Display name of the archive file or folder.
    pub source_name: String,
    pub format: ArchiveFormat,
    pub encoding: NameEncoding,
    pub legacy_names: bool,
    pub entries: u32,
    pub files: u32,
    pub total_bytes: u64,
    pub detected_root: RootKind,
    pub root_reason: RootReason,
    pub root: RootKind,
    pub candidates: Vec<Candidate>,
    pub groups: Vec<VariantGroup>,
    pub clashes: Vec<CaseClash>,
    pub issues: Vec<InstallIssue>,
    /// Any Block issue.
    pub blocked: bool,
    pub available_bytes: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidatePick {
    pub id: u32,
    pub replace: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClashPick {
    pub id: u32,
    pub keep: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartRequest {
    pub inspection: String,
    pub picks: Vec<CandidatePick>,
    pub clashes: Vec<ClashPick>,
    pub auto_fix: bool,
    pub apply_while_running: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallPhase {
    Preparing,
    Extracting,
    Committing,
    CleaningUp,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallProgress {
    pub phase: InstallPhase,
    pub done_bytes: u64,
    pub total_bytes: u64,
    pub done_files: u32,
    pub total_files: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeptReason {
    TrashUnavailable,
    TrashFailed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplacedOld {
    Trashed,
    KeptInRepair { reason: KeptReason },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstalledMod {
    pub key: String,
    pub mod_ref: Option<ModRef>,
    pub rel: String,
    pub files: u32,
    pub replaced: Option<ReplacedOld>,
}

#[derive(Clone, Debug)]
pub struct InstallOutcome {
    pub refreshed: crate::mods::Refreshed,
    pub installed: Vec<InstalledMod>,
}
