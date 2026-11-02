//! Conversions between the install DTOs and core's install types (`u64` sizes become `f64`,
//! ModRefs their token strings).

use remmods_core::archive;
use remmods_core::install as core;
use remmods_core::mods as core_mods;

use crate::install::{
    ArchiveFormat, CandidatePick, CaseClash, ClashPick, ExistingMatch, ExistingRelation,
    InspectRequest, Inspection, InstallBlock, InstallCandidate, InstallIssue, InstallIssueCode,
    InstallOutcome, InstallPhase, InstallProgress, InstalledMod, IssueSeverity, KeptReason,
    NameEncoding, NameOverride, ReplacedOld, RootReason, SourceKind, SourceReject, StartRequest,
    VariantGroup,
};
use crate::mods::{ModsSnapshot, RootKind, bytes};

impl From<core::SourceKind> for SourceKind {
    fn from(kind: core::SourceKind) -> Self {
        match kind {
            core::SourceKind::Zip => Self::Zip,
            core::SourceKind::SevenZip => Self::SevenZip,
            core::SourceKind::Rar => Self::Rar,
            core::SourceKind::Folder => Self::Folder,
        }
    }
}

impl From<core::SourceReject> for SourceReject {
    fn from(reason: core::SourceReject) -> Self {
        match reason {
            core::SourceReject::UnsupportedType => Self::UnsupportedType,
            core::SourceReject::Missing => Self::Missing,
            core::SourceReject::Link => Self::Link,
            core::SourceReject::InsidePaks => Self::InsidePaks,
        }
    }
}

impl From<NameEncoding> for archive::NameEncoding {
    fn from(encoding: NameEncoding) -> Self {
        match encoding {
            NameEncoding::Auto => Self::Auto,
            NameEncoding::ShiftJis => Self::ShiftJis,
            NameEncoding::Gbk => Self::Gbk,
            NameEncoding::Big5 => Self::Big5,
            NameEncoding::EucKr => Self::EucKr,
            NameEncoding::Ibm866 => Self::Ibm866,
            NameEncoding::Windows1251 => Self::Windows1251,
        }
    }
}

impl From<archive::NameEncoding> for NameEncoding {
    fn from(encoding: archive::NameEncoding) -> Self {
        match encoding {
            archive::NameEncoding::Auto => Self::Auto,
            archive::NameEncoding::ShiftJis => Self::ShiftJis,
            archive::NameEncoding::Gbk => Self::Gbk,
            archive::NameEncoding::Big5 => Self::Big5,
            archive::NameEncoding::EucKr => Self::EucKr,
            archive::NameEncoding::Ibm866 => Self::Ibm866,
            archive::NameEncoding::Windows1251 => Self::Windows1251,
        }
    }
}

impl From<archive::ArchiveFormat> for ArchiveFormat {
    fn from(format: archive::ArchiveFormat) -> Self {
        match format {
            archive::ArchiveFormat::Zip => Self::Zip,
            archive::ArchiveFormat::SevenZip => Self::SevenZip,
            archive::ArchiveFormat::Rar => Self::Rar,
            archive::ArchiveFormat::Folder => Self::Folder,
        }
    }
}

impl From<RootKind> for core_mods::RootKind {
    fn from(kind: RootKind) -> Self {
        match kind {
            RootKind::Standard => Self::Standard,
            RootKind::Legacy => Self::Legacy,
            RootKind::Logic => Self::Logic,
        }
    }
}

impl From<core::RootReason> for RootReason {
    fn from(reason: core::RootReason) -> Self {
        match reason {
            core::RootReason::PathSegment => Self::PathSegment,
            core::RootReason::ConfigLua => Self::ConfigLua,
            core::RootReason::Default => Self::Default,
            core::RootReason::Chosen => Self::Chosen,
        }
    }
}

impl From<core::IssueSeverity> for IssueSeverity {
    fn from(severity: core::IssueSeverity) -> Self {
        match severity {
            core::IssueSeverity::Block => Self::Block,
            core::IssueSeverity::Warn => Self::Warn,
            core::IssueSeverity::Info => Self::Info,
        }
    }
}

impl From<core::IssueCode> for InstallIssueCode {
    fn from(code: core::IssueCode) -> Self {
        use core::IssueCode as C;
        match code {
            C::UnsafePath => Self::UnsafePath,
            C::Link => Self::Link,
            C::Special => Self::Special,
            C::AntiItem => Self::AntiItem,
            C::DuplicateIdentical => Self::DuplicateIdentical,
            C::DuplicateDiffers => Self::DuplicateDiffers,
            C::DuplicateRaw => Self::DuplicateRaw,
            C::CaseClash => Self::CaseClash,
            C::InvalidName => Self::InvalidName,
            C::NotPortable => Self::NotPortable,
            C::Undecodable => Self::Undecodable,
            C::LegacyNames => Self::LegacyNames,
            C::NestedArchive => Self::NestedArchive,
            C::ShipsDisabled => Self::ShipsDisabled,
            C::ShipsBothStates => Self::ShipsBothStates,
            C::ScriptMod => Self::ScriptMod,
            C::NoPayload => Self::NoPayload,
            C::MixedRoots => Self::MixedRoots,
            C::OutsideRoot => Self::OutsideRoot,
            C::ExtraFiles => Self::ExtraFiles,
            C::PathTooLong => Self::PathTooLong,
            C::AlsoInRoot => Self::AlsoInRoot,
            C::Ambiguous => Self::Ambiguous,
            C::Replaces => Self::Replaces,
            C::NoSpace => Self::NoSpace,
            C::EmptyDirs => Self::EmptyDirs,
        }
    }
}

impl From<&core::InstallIssue> for InstallIssue {
    fn from(issue: &core::InstallIssue) -> Self {
        Self {
            severity: issue.severity.into(),
            code: issue.code.into(),
            path: issue.path.clone(),
            count: issue.count,
        }
    }
}

impl From<core::ExistingRelation> for ExistingRelation {
    fn from(relation: core::ExistingRelation) -> Self {
        match relation {
            core::ExistingRelation::SameName => Self::SameName,
            core::ExistingRelation::FoldEqual => Self::FoldEqual,
            core::ExistingRelation::DisabledPrefix => Self::DisabledPrefix,
        }
    }
}

impl From<&core::ExistingMatch> for ExistingMatch {
    fn from(existing: &core::ExistingMatch) -> Self {
        Self {
            mod_ref: existing.mod_ref.as_str().to_owned(),
            key: existing.key.clone(),
            rel: existing.rel.clone(),
            relation: existing.relation.into(),
            ambiguous: existing.ambiguous,
        }
    }
}

impl From<&core::Candidate> for InstallCandidate {
    fn from(candidate: &core::Candidate) -> Self {
        Self {
            id: candidate.id,
            name: candidate.name.clone(),
            inner: candidate.inner.clone(),
            files: candidate.files,
            payload: candidate.payload,
            shipped_off: candidate.shipped_off,
            bytes: bytes(candidate.bytes),
            group: candidate.group,
            selected: candidate.selected,
            name_issue: candidate
                .name_issue
                .as_ref()
                .map(|issue| issue.code().to_owned()),
            fixed_name: candidate.fixed_name.clone(),
            existing: candidate.existing.as_ref().map(ExistingMatch::from),
            also_in: candidate
                .also_in
                .iter()
                .copied()
                .map(RootKind::from)
                .collect(),
            path_too_long: candidate.path_too_long,
        }
    }
}

impl From<&core::VariantGroup> for VariantGroup {
    fn from(group: &core::VariantGroup) -> Self {
        Self {
            id: group.id,
            candidates: group.candidates.clone(),
        }
    }
}

impl From<&core::CaseClash> for CaseClash {
    fn from(clash: &core::CaseClash) -> Self {
        Self {
            id: clash.id,
            paths: clash.paths.clone(),
        }
    }
}

impl From<&NameOverride> for core::NameOverride {
    fn from(name: &NameOverride) -> Self {
        Self {
            id: name.id,
            name: name.name.clone(),
        }
    }
}

/// The options half of an inspect request; the source is resolved by the shell.
impl From<&InspectRequest> for core::InspectOptions {
    fn from(request: &InspectRequest) -> Self {
        Self {
            encoding: request.encoding.into(),
            root: request.root.map(core_mods::RootKind::from),
            names: request.names.iter().map(core::NameOverride::from).collect(),
        }
    }
}

impl From<&core::Inspection> for Inspection {
    fn from(inspection: &core::Inspection) -> Self {
        Self {
            id: inspection.id.clone(),
            source_name: inspection.source_name.clone(),
            format: inspection.format.into(),
            encoding: inspection.encoding.into(),
            legacy_names: inspection.legacy_names,
            entries: inspection.entries,
            files: inspection.files,
            total_bytes: bytes(inspection.total_bytes),
            detected_root: inspection.detected_root.into(),
            root_reason: inspection.root_reason.into(),
            root: inspection.root.into(),
            candidates: inspection
                .candidates
                .iter()
                .map(InstallCandidate::from)
                .collect(),
            groups: inspection.groups.iter().map(VariantGroup::from).collect(),
            clashes: inspection.clashes.iter().map(CaseClash::from).collect(),
            issues: inspection.issues.iter().map(InstallIssue::from).collect(),
            blocked: inspection.blocked,
            available_bytes: inspection.available_bytes.map(bytes),
        }
    }
}

impl From<&StartRequest> for core::StartRequest {
    fn from(request: &StartRequest) -> Self {
        Self {
            inspection: request.inspection.clone(),
            picks: request
                .picks
                .iter()
                .map(|pick: &CandidatePick| core::CandidatePick {
                    id: pick.id,
                    replace: pick.replace,
                })
                .collect(),
            clashes: request
                .clashes
                .iter()
                .map(|clash: &ClashPick| core::ClashPick {
                    id: clash.id,
                    keep: clash.keep,
                })
                .collect(),
            auto_fix: request.auto_fix,
            apply_while_running: request.apply_while_running,
        }
    }
}

impl From<core::InstallPhase> for InstallPhase {
    fn from(phase: core::InstallPhase) -> Self {
        match phase {
            core::InstallPhase::Preparing => Self::Preparing,
            core::InstallPhase::Extracting => Self::Extracting,
            core::InstallPhase::Committing => Self::Committing,
            core::InstallPhase::CleaningUp => Self::CleaningUp,
        }
    }
}

impl From<&core::InstallProgress> for InstallProgress {
    fn from(progress: &core::InstallProgress) -> Self {
        Self {
            phase: progress.phase.into(),
            done_bytes: bytes(progress.done_bytes),
            total_bytes: bytes(progress.total_bytes),
            done_files: progress.done_files,
            total_files: progress.total_files,
        }
    }
}

impl From<core::ReplacedOld> for ReplacedOld {
    fn from(replaced: core::ReplacedOld) -> Self {
        match replaced {
            core::ReplacedOld::Trashed => Self::Trashed,
            core::ReplacedOld::KeptInRepair { reason } => Self::KeptInRepair {
                reason: match reason {
                    core::KeptReason::TrashUnavailable => KeptReason::TrashUnavailable,
                    core::KeptReason::TrashFailed => KeptReason::TrashFailed,
                },
            },
        }
    }
}

impl From<&core::InstalledMod> for InstalledMod {
    fn from(installed: &core::InstalledMod) -> Self {
        Self {
            key: installed.key.clone(),
            mod_ref: installed
                .mod_ref
                .as_ref()
                .map(|mod_ref| mod_ref.as_str().to_owned()),
            rel: installed.rel.clone(),
            files: installed.files,
            replaced: installed.replaced.map(ReplacedOld::from),
        }
    }
}

impl From<&core::InstallOutcome> for InstallOutcome {
    fn from(outcome: &core::InstallOutcome) -> Self {
        Self {
            snapshot: ModsSnapshot::from(&*outcome.refreshed.snapshot),
            installed: outcome.installed.iter().map(InstalledMod::from).collect(),
        }
    }
}

impl From<core::InstallBlock> for InstallBlock {
    fn from(block: core::InstallBlock) -> Self {
        match block {
            core::InstallBlock::Issues => Self::Issues,
            core::InstallBlock::InvalidName => Self::InvalidName,
            core::InstallBlock::UnresolvedClash => Self::UnresolvedClash,
            core::InstallBlock::NoSelection => Self::NoSelection,
            core::InstallBlock::VariantConflict => Self::VariantConflict,
            core::InstallBlock::ReplaceNotConfirmed => Self::ReplaceNotConfirmed,
            core::InstallBlock::ReplaceMismatch => Self::ReplaceMismatch,
            core::InstallBlock::Ambiguous => Self::Ambiguous,
            core::InstallBlock::TargetChanged => Self::TargetChanged,
        }
    }
}
