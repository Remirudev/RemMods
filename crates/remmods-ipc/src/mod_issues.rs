use remmods_core::mods as core;
use serde::{Deserialize, Serialize};

use crate::mods::ScanProblem;

/// Which naming rule a mod or file name breaks (the UI translates it).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum NameProblem {
    Empty,
    DotName,
    TooLong,
    ForbiddenChar,
    ControlChar,
    BidiControl,
    TrailingDotOrSpace,
    ReservedDeviceName,
    DisabledPrefix,
    TempName,
}

/// A finding on one mod row. Struct variants keep single-word fields: serde's `rename_all` on a
/// tagged enum renames the variants only.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ModIssue {
    AmbiguousKey,
    CaseCollision { others: Vec<String> },
    CaseVariantRoot,
    NonUnicodeName,
    InvalidName { problem: NameProblem },
    NameNotPortable { problem: NameProblem },
    DisabledPrefixName,
    EmptyName,
    LogicDisabledButLoaded,
    LogicEnabledNotLoaded,
    ModinfoInvalid,
    ModinfoCaseVariants,
    PathTooLong { longest: u32 },
    LinkedRoot,
    LinkedMod,
    ContainsLink,
    LooseCopy { files: Vec<String> },
    LooseCaseVariants,
    NonUnicodeFiles { count: u32 },
    InvalidFileNames { count: u32 },
    NonPortableFileNames { count: u32 },
    Unreadable { problem: ScanProblem },
}

impl From<core::NameProblem> for NameProblem {
    fn from(problem: core::NameProblem) -> Self {
        match problem {
            core::NameProblem::Empty => Self::Empty,
            core::NameProblem::DotName => Self::DotName,
            core::NameProblem::TooLong => Self::TooLong,
            core::NameProblem::ForbiddenChar => Self::ForbiddenChar,
            core::NameProblem::ControlChar => Self::ControlChar,
            core::NameProblem::BidiControl => Self::BidiControl,
            core::NameProblem::TrailingDotOrSpace => Self::TrailingDotOrSpace,
            core::NameProblem::ReservedDeviceName => Self::ReservedDeviceName,
            core::NameProblem::DisabledPrefix => Self::DisabledPrefix,
            core::NameProblem::TempName => Self::TempName,
        }
    }
}

impl From<&core::ModIssue> for ModIssue {
    fn from(issue: &core::ModIssue) -> Self {
        match issue {
            core::ModIssue::AmbiguousKey => Self::AmbiguousKey,
            core::ModIssue::CaseCollision { others } => Self::CaseCollision {
                others: others.clone(),
            },
            core::ModIssue::CaseVariantRoot => Self::CaseVariantRoot,
            core::ModIssue::NonUnicodeName => Self::NonUnicodeName,
            core::ModIssue::InvalidName { problem } => Self::InvalidName {
                problem: (*problem).into(),
            },
            core::ModIssue::NameNotPortable { problem } => Self::NameNotPortable {
                problem: (*problem).into(),
            },
            core::ModIssue::DisabledPrefixName => Self::DisabledPrefixName,
            core::ModIssue::EmptyName => Self::EmptyName,
            core::ModIssue::LogicDisabledButLoaded => Self::LogicDisabledButLoaded,
            core::ModIssue::LogicEnabledNotLoaded => Self::LogicEnabledNotLoaded,
            core::ModIssue::ModinfoInvalid => Self::ModinfoInvalid,
            core::ModIssue::ModinfoCaseVariants => Self::ModinfoCaseVariants,
            core::ModIssue::PathTooLong { longest } => Self::PathTooLong { longest: *longest },
            core::ModIssue::LinkedRoot => Self::LinkedRoot,
            core::ModIssue::LinkedMod => Self::LinkedMod,
            core::ModIssue::ContainsLink => Self::ContainsLink,
            core::ModIssue::LooseCopy { files } => Self::LooseCopy {
                files: files.clone(),
            },
            core::ModIssue::LooseCaseVariants => Self::LooseCaseVariants,
            core::ModIssue::NonUnicodeFiles { count } => Self::NonUnicodeFiles { count: *count },
            core::ModIssue::InvalidFileNames { count } => Self::InvalidFileNames { count: *count },
            core::ModIssue::NonPortableFileNames { count } => {
                Self::NonPortableFileNames { count: *count }
            }
            core::ModIssue::Unreadable { problem } => Self::Unreadable {
                problem: (*problem).into(),
            },
        }
    }
}
