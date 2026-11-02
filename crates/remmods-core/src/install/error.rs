use super::model::SourceReject;
use crate::archive::ArchiveError;

/// Why a start was refused before anything was written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallBlock {
    Issues,
    InvalidName,
    UnresolvedClash,
    NoSelection,
    VariantConflict,
    ReplaceNotConfirmed,
    ReplaceMismatch,
    Ambiguous,
    TargetChanged,
}

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error(transparent)]
    Archive(#[from] ArchiveError),
    #[error("another install is running")]
    Busy,
    #[error("unknown or expired inspection")]
    UnknownInspection,
    #[error("the source changed since it was inspected")]
    SourceChanged,
    #[error("install blocked ({reason:?})")]
    Blocked { reason: InstallBlock },
    #[error("not enough space: need {needed}, available {available}")]
    NoSpace { needed: u64, available: u64 },
    #[error("cancelled")]
    Cancelled,
    #[error("source rejected ({reason:?})")]
    Source { reason: SourceReject },
}

pub(crate) fn blocked(reason: InstallBlock) -> InstallError {
    InstallError::Blocked { reason }
}
