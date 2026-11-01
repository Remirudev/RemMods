//! Profiles (spec §5.7–§5.10): RemMods' own `profiles.json`, diffs, apply as one batch with an
//! auto snapshot, import of the original's `profiles.json` (read-only) and export/import.

mod api;
mod api_transfer;
mod apply;
mod capture;
mod diff;
mod error;
mod import_tmm;
mod model;
mod rekey;
mod store;
mod transfer;

pub use api::{ApplyOutcome, ApplyRequest, ProfileSave, ProfileSummary, ProfilesView};
pub use capture::capture_states;
pub use diff::{ChangeReason, DiffChange, DiffSkip, ProfileDiff, SkipReason, diff_profile};
pub use error::{ImportProblem, ProfileError};
pub use import_tmm::{IMPORT_MAX_BYTES, TmmProfile, parse_tmm_profiles};
pub use model::{
    EntryState, LastApplied, PROFILE_NAME_MAX_CHARS, PROFILES_FILE, PROFILES_VERSION, Profile,
    ProfileEntry, ProfileMode, ProfileSource, ProfilesFile, SNAPSHOT_KEEP,
};
pub use store::{LoadedProfiles, ProfileStore, StoreProblem};
pub use transfer::{ExportReport, ImportReport, export_bytes, parse_export};
