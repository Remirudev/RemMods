//! Types that cross the IPC boundary: serde + specta only, no Tauri. No `PathBuf` (display strings
//! only) and no `u64` (JavaScript numbers lose precision past 2^53).

mod about;
mod bulk;
mod busy;
mod content;
mod error;
mod game;
mod graphics;
mod install;
mod install_convert;
mod manage;
mod mod_issues;
mod mods;
mod platform;
mod profiles;
mod repair;
mod settings;
mod smoke;
mod toggle;
mod update;

pub use about::Notices;
pub use bulk::{
    BulkDeletePrepared, BulkDeleteRequest, BulkOutcome, BulkRow, BulkRowResult, BulkTrashRequest,
    PreparedRow,
};
pub use busy::{BusyPhase, FolderBusyInfo};
pub use content::{
    AssetEntry, AssetOrigin, BaseIndexState, CharacterHint, ContainerStatus, ContainerSummary,
    ContentConflictLink, ContentSummary, ModContents, Slot, SlotHint,
};
pub use error::{AppError, OsErrorClass};
pub use game::{
    DetectionNote, FolderProblem, GameCandidate, GameLocation, GameStatus, InstallInfo,
    LocationChoice, LocationSource, NoteCode, PickOutcome, RevealProblem, RunningState, SteamKind,
    UpdateNotice, UpdateTracking,
};
pub use graphics::{GraphicsMode, GraphicsState, GraphicsStep};
pub use install::{
    ArchiveFormat, ArchiveProblem, CandidatePick, CaseClash, ClashPick, DropPhase, ExistingMatch,
    ExistingRelation, InspectRequest, Inspection, InstallBlock, InstallCandidate, InstallIssue,
    InstallIssueCode, InstallOutcome, InstallPhase, InstallProgress, InstallSourceRef,
    InstalledMod, IssueSeverity, KeptReason, NameEncoding, NameOverride, PickedSource,
    RejectedSource, ReplacedOld, RootReason, SourceKind, SourceReject, StartRequest, VariantGroup,
};
pub use manage::{
    DeleteOutcome, DeletePrepared, DeleteRequest, ManageAction, ManageBlock, MetadataRequest,
    ModInfoEdit, ModTarget, RekeyReport, RenameOutcome, RenameRequest, TrashKind, TrashOutcome,
    TrashRequest, TrashUnavailableReason,
};
pub use mod_issues::{ModIssue, NameProblem};
pub use mods::{
    ArchiveEntry, ConflictLink, LeftoverEntry, ModDetails, ModInfo, ModInfoDefaulted,
    ModInfoOrigin, ModKind, ModRow, ModState, ModsSnapshot, PayloadCounts, PayloadFile,
    PayloadKind, RootInfo, RootKind, ScanProblem, ScanStats, UnmanagedEntry, UnmanagedKind,
};
pub use platform::{AppPathsDto, OsKind, PlatformFeature, PlatformFeatures, PlatformInfo};
pub use profiles::{
    ApplyOutcome, ApplyRequest, ChangeReason, DiffChange, DiffSkip, EntryState, ExportReport,
    ImportProblem, ImportReport, LastApplied, Profile, ProfileDiff, ProfileEntryDto, ProfileMode,
    ProfileSave, ProfileSource, ProfileSummary, ProfilesView, SkipReason, StoreProblem,
    TransferOutcome, TransferRequest,
};
pub use repair::{RepairAction, RepairItem, RepairKind, RepairList};
pub use settings::{
    Appearance, AppearanceUpdate, ColorScheme, Settings, SettingsUpdate, TitleBarMode,
};
pub use smoke::{SmokeCheck, SmokeReport};
pub use toggle::{
    SkippedRow, ToggleBlock, ToggleOutcome, ToggleRequest, ToggleTarget, WatchState, WatchStatus,
};
pub use update::{
    AppInstallKind, AppUpdateChannel, AppUpdateCheck, AppUpdateProgress, UpdateBlock, UpdateStage,
};
