//! Profiles (spec §5.7–§5.10): RemMods' own `profiles.json`, diffs, apply as one batch with an
//! auto snapshot, read-only import of another mod manager's `profiles.json`, and export/import of
//! RemMods files.

mod api;
mod api_transfer;
mod apply;
mod capture;
mod diff;
mod error;
mod import_file;
mod import_other;
mod model;
mod rekey;
mod store;
mod transfer;

pub use api::{ApplyOutcome, ApplyRequest, ProfileSave, ProfileSummary, ProfilesView};
pub use capture::capture_states;
pub use diff::{
    ApplyDirection, ChangeReason, DiffChange, DiffSkip, ProfileDiff, SkipReason, diff_profile,
    diff_switch,
};
pub use error::{ImportProblem, ProfileError};
pub use import_file::IMPORT_MAX_BYTES;
pub use import_other::{OtherManagerProfile, parse_other_manager_profiles};
pub(crate) use model::now_rfc3339;
pub use model::{
    EntryState, LastApplied, PROFILE_NAME_MAX_CHARS, PROFILES_FILE, PROFILES_VERSION, Profile,
    ProfileEntry, ProfileMode, ProfileSource, ProfilesFile, SNAPSHOT_KEEP,
};
pub use store::{LoadedProfiles, ProfileStore, StoreProblem};
pub use transfer::{ExportReport, ImportReport, export_bytes, parse_export};
