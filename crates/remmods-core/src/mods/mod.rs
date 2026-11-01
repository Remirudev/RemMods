//! The mods scan (PLAN §4 Scan, §7). Read-only in P1: nothing here writes to the disk.

mod api;
mod conflicts;
mod details;
mod error;
mod issues;
mod listing;
mod metadata;
mod model;
mod modref;
mod path_length;
mod payload;
mod rows;
mod scan;
mod scan_log;
mod service;
mod strategy;
mod unmanaged;
mod walk;

pub use conflicts::{original_conflict_names, original_enabled};
pub use details::{read_details, resolve_ref, shell_form};
pub use error::ModsError;
pub use issues::{ModIssue, NameProblem};
pub use metadata::{MODINFO_MAX_BYTES, decode_modinfo_text, parse_modinfo};
pub use model::{
    ArchiveEntry, ConflictLink, DETAILS_MAX_FILES, LeftoverEntry, ModDetails, ModInfo,
    ModInfoDefaulted, ModInfoOrigin, ModKind, ModRow, ModState, PayloadCounts, PayloadFile,
    PayloadKind, RootInfo, RootKind, ScanContent, ScanInput, ScanOutput, ScanProblem, ScanSnapshot,
    ScanStats, UnmanagedEntry, UnmanagedKind,
};
pub use modref::{ModRef, ParsedRef, RefKind, paks_identity};
pub use scan::scan_paks;
pub use service::{ModsService, Refreshed};
