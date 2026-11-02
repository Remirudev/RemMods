//! The mods scan (PLAN §4 Scan, §7) and the toggle (P2). Every rename goes through the OpGate's
//! `WriteGuard`.

mod api;
mod api_manage;
mod api_toggle;
mod conflicts;
mod delete;
mod delete_token;
mod details;
mod error;
mod issues;
mod listing;
pub(crate) mod manage_target;
mod meta_store;
mod metadata;
mod model;
mod modinfo_write;
mod modref;
pub(crate) mod path_length;
pub(crate) mod payload;
mod rename;
mod repair;
pub(crate) mod rows;
mod scan;
mod scan_log;
mod service;
mod strategy;
mod toggle_apply;
mod toggle_files;
mod toggle_plan;
mod trash;
mod unmanaged;
mod walk;

pub use conflicts::{original_conflict_names, original_enabled};
pub use delete::{DeleteOutcome, DeletePrepared, DeleteRequest, plan_set_aside};
pub use delete_token::DeleteTokens;
pub use details::{read_details, resolve_ref, shell_form};
pub use error::ModsError;
pub use issues::{ModIssue, NameProblem};
pub use manage_target::{ManageAction, ManageBlock, ManagedKind, ModTarget, manage_block};
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
pub use modinfo_write::{
    MODINFO_FIELD_MAX_CHARS, MetadataRequest, ModInfoEdit, modinfo_json, stj_escape,
};
pub use modref::{ModRef, ParsedRef, RefKind, paks_identity};
pub use rename::{RekeyReport, RenameOutcome, RenameRequest, plan_rename};
pub use repair::{RepairAction, RepairItem, RepairKind, RepairList, repair_items, suggest_name};
pub use scan::{scan_paks, scan_paks_with, scan_paks_with_base};
pub use service::{ModsService, Refreshed};
pub use toggle_apply::{SkippedRow, ToggleOutcome, ToggleRequest, ToggleTarget};
pub(crate) use toggle_apply::{plan_rows, wants_disable};
pub use toggle_files::{FileClass, ModFile, ModFiles, list_toggle_files};
pub use toggle_plan::{RecordChange, RowPlan, plan_row, record_matches};
pub use trash::{TrashOutcome, TrashRequest};
