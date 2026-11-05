use remmods_core::profiles as core;
use serde::{Deserialize, Serialize};

use crate::mods::ModsSnapshot;
use crate::skins::LinkedSkins;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ProfileMode {
    Additive,
    Exclusive,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum EntryState {
    Enabled,
    Disabled,
}

/// Snapshot profiles have an empty name; the UI renders their label from `of`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ProfileSource {
    User,
    Snapshot {
        of: String,
    },
    #[serde(rename_all = "camelCase")]
    OtherManager {
        id: String,
        is_default: bool,
    },
    Import,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProfileEntryDto {
    pub key: String,
    pub state: EntryState,
}

/// `mods` is sorted by key bytes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub mode: ProfileMode,
    pub mods: Vec<ProfileEntryDto>,
    pub created: String,
    pub updated: String,
    pub last_applied: Option<String>,
    pub source: ProfileSource,
    /// The skin set switching this group on brings along.
    pub skin_set: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSummary {
    pub profile: Profile,
    pub enabled: u32,
    pub disabled: u32,
    pub missing: u32,
    pub matches_current: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LastApplied {
    pub profile_id: String,
    pub at: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum StoreProblem {
    Corrupt,
    NewerVersion,
    Unreadable,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesView {
    pub profiles: Vec<ProfileSummary>,
    pub last_applied: Option<LastApplied>,
    pub read_only: bool,
    pub problem: Option<StoreProblem>,
}

/// `new` captures the current states, `overwrite` replaces a profile's mods with them, `edit`
/// changes name, description and mode only, `picked` saves exactly the listed mods as a new
/// profile (`id` null) or into an existing one.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ProfileSave {
    New {
        name: String,
        description: String,
        mode: ProfileMode,
    },
    Overwrite {
        id: String,
    },
    Edit {
        id: String,
        name: String,
        description: String,
        mode: ProfileMode,
    },
    Picked {
        id: Option<String>,
        name: String,
        description: String,
        mode: ProfileMode,
        mods: Vec<ProfileEntryDto>,
        skins: Option<String>,
    },
}

/// `on` applies the profile; `off` turns off the mods it lists as on and nothing else.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ApplyDirection {
    On,
    Off,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ChangeReason {
    Listed,
    Unlisted,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DiffChange {
    pub mod_ref: String,
    pub key: String,
    pub name: String,
    pub enable: bool,
    pub reason: ChangeReason,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum SkipReason {
    Ambiguous,
    Linked,
    UnknownState,
    NoPakFiles,
    InvalidLogicName,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DiffSkip {
    pub key: String,
    pub name: Option<String>,
    pub wanted: EntryState,
    pub reason: SkipReason,
}

/// `fingerprint` goes back unchanged in `ApplyRequest`; core refuses the apply as stale when the
/// changes differ by then.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDiff {
    pub profile_id: String,
    pub mode: ProfileMode,
    pub changes: Vec<DiffChange>,
    pub missing: Vec<String>,
    pub skipped: Vec<DiffSkip>,
    pub unchanged: u32,
    pub fingerprint: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRequest {
    pub profile_id: String,
    pub fingerprint: String,
    pub apply_while_running: bool,
    pub direction: ApplyDirection,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ApplyOutcome {
    pub snapshot: ModsSnapshot,
    pub changed: u32,
    pub snapshot_id: Option<String>,
    pub skipped: Vec<DiffSkip>,
    pub skins: Option<LinkedSkins>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub added: u32,
    pub updated: u32,
    pub unchanged: u32,
    pub skipped: u32,
    pub view: ProfilesView,
}

/// `fileName` is for display only; the picked path never crosses IPC.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExportReport {
    pub count: u32,
    pub file_name: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ImportProblem {
    TooLarge,
    NotJson,
    WrongShape,
    WrongFormat,
    NewerVersion,
}

/// The one file-dialog command; the dialog title comes from the UI.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TransferRequest {
    ImportOtherManager {
        title: String,
    },
    Import {
        title: String,
    },
    #[serde(rename_all = "camelCase")]
    Export {
        title: String,
        default_file_name: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TransferOutcome {
    Imported { report: ImportReport },
    Exported { report: ExportReport },
}

impl From<core::ProfileMode> for ProfileMode {
    fn from(mode: core::ProfileMode) -> Self {
        match mode {
            core::ProfileMode::Additive => Self::Additive,
            core::ProfileMode::Exclusive => Self::Exclusive,
        }
    }
}

impl From<ProfileMode> for core::ProfileMode {
    fn from(mode: ProfileMode) -> Self {
        match mode {
            ProfileMode::Additive => Self::Additive,
            ProfileMode::Exclusive => Self::Exclusive,
        }
    }
}

impl From<core::EntryState> for EntryState {
    fn from(state: core::EntryState) -> Self {
        match state {
            core::EntryState::Enabled => Self::Enabled,
            core::EntryState::Disabled => Self::Disabled,
        }
    }
}

impl From<EntryState> for core::EntryState {
    fn from(state: EntryState) -> Self {
        match state {
            EntryState::Enabled => Self::Enabled,
            EntryState::Disabled => Self::Disabled,
        }
    }
}

impl From<ApplyDirection> for core::ApplyDirection {
    fn from(direction: ApplyDirection) -> Self {
        match direction {
            ApplyDirection::On => Self::On,
            ApplyDirection::Off => Self::Off,
        }
    }
}

impl From<&core::ProfileSource> for ProfileSource {
    fn from(source: &core::ProfileSource) -> Self {
        match source {
            core::ProfileSource::User => Self::User,
            core::ProfileSource::Snapshot { of } => Self::Snapshot { of: of.clone() },
            core::ProfileSource::OtherManager { id, is_default } => Self::OtherManager {
                id: id.clone(),
                is_default: *is_default,
            },
            core::ProfileSource::Import => Self::Import,
        }
    }
}

impl From<&core::Profile> for Profile {
    fn from(profile: &core::Profile) -> Self {
        // A BTreeMap<String, _> iterates in key-byte order, which is the DTO's sort contract.
        let mods = profile
            .mods
            .iter()
            .map(|(key, entry)| ProfileEntryDto {
                key: key.clone(),
                state: entry.state.into(),
            })
            .collect();
        Self {
            id: profile.id.clone(),
            name: profile.name.clone(),
            description: profile.description.clone(),
            mode: profile.mode.into(),
            mods,
            created: profile.created.clone(),
            updated: profile.updated.clone(),
            last_applied: profile.last_applied.clone(),
            source: ProfileSource::from(&profile.source),
            skin_set: profile.skin_set().map(str::to_owned),
        }
    }
}

impl From<&core::ProfileSummary> for ProfileSummary {
    fn from(summary: &core::ProfileSummary) -> Self {
        Self {
            profile: Profile::from(&summary.profile),
            enabled: summary.enabled,
            disabled: summary.disabled,
            missing: summary.missing,
            matches_current: summary.matches_current,
        }
    }
}

impl From<&core::LastApplied> for LastApplied {
    fn from(last: &core::LastApplied) -> Self {
        Self {
            profile_id: last.profile_id.clone(),
            at: last.at.clone(),
        }
    }
}

impl From<core::StoreProblem> for StoreProblem {
    fn from(problem: core::StoreProblem) -> Self {
        match problem {
            core::StoreProblem::Corrupt => Self::Corrupt,
            core::StoreProblem::NewerVersion => Self::NewerVersion,
            core::StoreProblem::Unreadable => Self::Unreadable,
        }
    }
}

impl From<&core::ProfilesView> for ProfilesView {
    fn from(view: &core::ProfilesView) -> Self {
        Self {
            profiles: view.profiles.iter().map(ProfileSummary::from).collect(),
            last_applied: view.last_applied.as_ref().map(LastApplied::from),
            read_only: view.read_only,
            problem: view.problem.map(StoreProblem::from),
        }
    }
}

impl From<&ProfileSave> for core::ProfileSave {
    fn from(save: &ProfileSave) -> Self {
        match save {
            ProfileSave::New {
                name,
                description,
                mode,
            } => Self::New {
                name: name.clone(),
                description: description.clone(),
                mode: (*mode).into(),
            },
            ProfileSave::Overwrite { id } => Self::Overwrite { id: id.clone() },
            ProfileSave::Edit {
                id,
                name,
                description,
                mode,
            } => Self::Edit {
                id: id.clone(),
                name: name.clone(),
                description: description.clone(),
                mode: (*mode).into(),
            },
            ProfileSave::Picked {
                id,
                name,
                description,
                mode,
                mods,
                skins,
            } => Self::Picked {
                id: id.clone(),
                name: name.clone(),
                description: description.clone(),
                mode: (*mode).into(),
                mods: mods
                    .iter()
                    .map(|entry| (entry.key.clone(), entry.state.into()))
                    .collect(),
                skins: skins.clone(),
            },
        }
    }
}

impl From<core::ChangeReason> for ChangeReason {
    fn from(reason: core::ChangeReason) -> Self {
        match reason {
            core::ChangeReason::Listed => Self::Listed,
            core::ChangeReason::Unlisted => Self::Unlisted,
        }
    }
}

impl From<&core::DiffChange> for DiffChange {
    fn from(change: &core::DiffChange) -> Self {
        Self {
            mod_ref: change.mod_ref.as_str().to_owned(),
            key: change.key.clone(),
            name: change.name.clone(),
            enable: change.enable,
            reason: change.reason.into(),
        }
    }
}

impl From<core::SkipReason> for SkipReason {
    fn from(reason: core::SkipReason) -> Self {
        match reason {
            core::SkipReason::Ambiguous => Self::Ambiguous,
            core::SkipReason::Linked => Self::Linked,
            core::SkipReason::UnknownState => Self::UnknownState,
            core::SkipReason::NoPakFiles => Self::NoPakFiles,
            core::SkipReason::InvalidLogicName => Self::InvalidLogicName,
        }
    }
}

impl From<&core::DiffSkip> for DiffSkip {
    fn from(skip: &core::DiffSkip) -> Self {
        Self {
            key: skip.key.clone(),
            name: skip.name.clone(),
            wanted: skip.wanted.into(),
            reason: skip.reason.into(),
        }
    }
}

impl From<&core::ProfileDiff> for ProfileDiff {
    fn from(diff: &core::ProfileDiff) -> Self {
        Self {
            profile_id: diff.profile_id.clone(),
            mode: diff.mode.into(),
            changes: diff.changes.iter().map(DiffChange::from).collect(),
            missing: diff.missing.clone(),
            skipped: diff.skipped.iter().map(DiffSkip::from).collect(),
            unchanged: diff.unchanged,
            fingerprint: diff.fingerprint.clone(),
        }
    }
}

impl From<&ApplyRequest> for core::ApplyRequest {
    fn from(request: &ApplyRequest) -> Self {
        Self {
            profile_id: request.profile_id.clone(),
            fingerprint: request.fingerprint.clone(),
            apply_while_running: request.apply_while_running,
            direction: request.direction.into(),
        }
    }
}

impl From<&core::ApplyOutcome> for ApplyOutcome {
    fn from(outcome: &core::ApplyOutcome) -> Self {
        Self {
            snapshot: ModsSnapshot::from(&*outcome.refreshed.snapshot),
            changed: outcome.changed,
            snapshot_id: outcome.snapshot_id.clone(),
            skipped: outcome.skipped.iter().map(DiffSkip::from).collect(),
            skins: outcome.skins.as_ref().map(LinkedSkins::from),
        }
    }
}

impl From<&core::ImportReport> for ImportReport {
    fn from(report: &core::ImportReport) -> Self {
        Self {
            added: report.added,
            updated: report.updated,
            unchanged: report.unchanged,
            skipped: report.skipped,
            view: ProfilesView::from(&report.view),
        }
    }
}

impl From<&core::ExportReport> for ExportReport {
    fn from(report: &core::ExportReport) -> Self {
        Self {
            count: report.count,
            file_name: report.file_name.clone(),
        }
    }
}

impl From<core::ImportProblem> for ImportProblem {
    fn from(problem: core::ImportProblem) -> Self {
        match problem {
            core::ImportProblem::TooLarge => Self::TooLarge,
            core::ImportProblem::NotJson => Self::NotJson,
            core::ImportProblem::WrongShape => Self::WrongShape,
            core::ImportProblem::WrongFormat => Self::WrongFormat,
            core::ImportProblem::NewerVersion => Self::NewerVersion,
        }
    }
}
