//! The mods scan (PLAN §4 Scan, §7) and the toggle (P2). Every rename goes through the OpGate's
//! `WriteGuard`.

mod api;
mod api_toggle;
mod conflicts;
mod details;
mod error;
mod issues;
mod listing;
mod meta_store;
mod metadata;
mod model;
mod modref;
mod path_length;
pub(crate) mod payload;
pub(crate) mod rows;
mod scan;
mod scan_log;
mod service;
mod strategy;
mod toggle_apply;
mod toggle_files;
mod toggle_plan;
mod unmanaged;
mod walk;

pub use conflicts::{original_conflict_names, original_enabled};
pub use details::{read_details, resolve_ref, shell_form};
pub use error::ModsError;
pub use issues::{ModIssue, NameProblem};
pub use meta_store::{
    DisableRecord, LoadedMeta, MOD_META_FILE, MOD_META_VERSION, ModMeta, ModMetaEntry, ModMetaStore,
};
pub use metadata::{MODINFO_MAX_BYTES, decode_modinfo_text, parse_modinfo};
pub use model::{
    ArchiveEntry, ConflictLink, DETAILS_MAX_FILES, LeftoverEntry, ModDetails, ModInfo,
    ModInfoDefaulted, ModInfoOrigin, ModKind, ModRow, ModState, PayloadCounts, PayloadFile,
    PayloadKind, RootInfo, RootKind, ScanContent, ScanInput, ScanOutput, ScanProblem, ScanSnapshot,
    ScanStats, ToggleBlock, UnmanagedEntry, UnmanagedKind,
};
pub use modref::{ModRef, ParsedRef, RefKind, paks_identity};
pub use scan::{scan_paks, scan_paks_with, scan_paks_with_base};
pub use service::{ModsService, Refreshed};
pub use toggle_apply::{SkippedRow, ToggleOutcome, ToggleRequest, ToggleTarget};
pub use toggle_files::{FileClass, ModFile, ModFiles, list_toggle_files};
pub use toggle_plan::{RecordChange, RowPlan, plan_row, record_matches};
