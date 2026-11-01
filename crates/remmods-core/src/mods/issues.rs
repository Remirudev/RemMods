use super::model::ScanProblem;
use crate::safe_fs::names::NameIssue;

/// `NameIssue` without the offending character, for IPC.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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

impl From<NameIssue> for NameProblem {
    fn from(issue: NameIssue) -> Self {
        match issue {
            NameIssue::Empty => Self::Empty,
            NameIssue::DotName => Self::DotName,
            NameIssue::TooLong => Self::TooLong,
            NameIssue::ForbiddenChar(_) => Self::ForbiddenChar,
            NameIssue::ControlChar(_) => Self::ControlChar,
            NameIssue::BidiControl(_) => Self::BidiControl,
            NameIssue::TrailingDotOrSpace => Self::TrailingDotOrSpace,
            NameIssue::ReservedDeviceName => Self::ReservedDeviceName,
            NameIssue::DisabledPrefix => Self::DisabledPrefix,
            NameIssue::TempName => Self::TempName,
        }
    }
}

/// Findings on a row. A row lists them in declaration order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModIssue {
    AmbiguousKey,
    CaseCollision {
        others: Vec<String>,
    },
    CaseVariantRoot,
    NonUnicodeName,
    /// The name fails `windows_name_issue` (Win32 rejects it).
    InvalidName {
        problem: NameProblem,
    },
    /// The name fails only `portable_name_issue` (Windows accepts it, RemMods would not create it).
    NameNotPortable {
        problem: NameProblem,
    },
    DisabledPrefixName,
    EmptyName,
    LogicDisabledButLoaded,
    LogicEnabledNotLoaded,
    ModinfoInvalid,
    ModinfoCaseVariants,
    PathTooLong {
        longest: u32,
    },
    LinkedRoot,
    LinkedMod,
    ContainsLink,
    LooseCopy {
        files: Vec<String>,
    },
    LooseCaseVariants,
    NonUnicodeFiles {
        count: u32,
    },
    /// File names failing `windows_name_issue`.
    InvalidFileNames {
        count: u32,
    },
    /// File names failing only `portable_name_issue` (incl. the payload budget).
    NonPortableFileNames {
        count: u32,
    },
    Unreadable {
        problem: ScanProblem,
    },
}

impl ModIssue {
    /// Position in the declaration order.
    pub(crate) fn rank(&self) -> u8 {
        match self {
            Self::AmbiguousKey => 0,
            Self::CaseCollision { .. } => 1,
            Self::CaseVariantRoot => 2,
            Self::NonUnicodeName => 3,
            Self::InvalidName { .. } => 4,
            Self::NameNotPortable { .. } => 5,
            Self::DisabledPrefixName => 6,
            Self::EmptyName => 7,
            Self::LogicDisabledButLoaded => 8,
            Self::LogicEnabledNotLoaded => 9,
            Self::ModinfoInvalid => 10,
            Self::ModinfoCaseVariants => 11,
            Self::PathTooLong { .. } => 12,
            Self::LinkedRoot => 13,
            Self::LinkedMod => 14,
            Self::ContainsLink => 15,
            Self::LooseCopy { .. } => 16,
            Self::LooseCaseVariants => 17,
            Self::NonUnicodeFiles { .. } => 18,
            Self::InvalidFileNames { .. } => 19,
            Self::NonPortableFileNames { .. } => 20,
            Self::Unreadable { .. } => 21,
        }
    }
}
