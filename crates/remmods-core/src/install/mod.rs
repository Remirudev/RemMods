//! Installing mods from archives and folders (PLAN §4 Install, spec §5): inspect (read-only),
//! then stage into journaled inert folders and commit with one all-or-none rename batch.

mod api;
mod api_start;
mod candidates;
mod commit;
mod detect_root;
mod error;
mod existing;
mod files;
mod inspect;
mod issues;
mod job;
mod model;
mod names_fix;
mod plan;
mod service;
mod source;
mod stage;

pub use detect_root::{RootDetection, detect_root};
pub use error::{InstallBlock, InstallError};
pub use model::{
    Candidate, CandidatePick, CaseClash, ClashPick, ExistingMatch, ExistingRelation,
    InspectOptions, Inspection, InstallIssue, InstallOutcome, InstallPhase, InstallProgress,
    InstallSource, InstalledMod, IssueCode, IssueSeverity, KeptReason, NameOverride, ReplacedOld,
    RootReason, SourceKind, SourceReject, StartRequest, VariantGroup,
};
pub use names_fix::fix_name;
pub use plan::{CandidatePlan, PlanFile, PlannedCandidate, PlannedGroup, plan_candidates};
pub use service::{AfterStageHook, InstallHooks, InstallService};
pub use source::classify_source;
