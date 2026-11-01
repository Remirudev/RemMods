use std::io;
use std::path::{Path, PathBuf};

use super::batch::BatchError;
use super::journal::JournalProblem;
use super::scope::ScopeViolation;
use crate::platform::{self, OsErrorClass, TrashUnavailableReason};

#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error("another RemMods process holds the app lock")]
    Busy,
    #[error("{op} failed for {path} ({class:?}): {source}", path = path.display())]
    Io {
        op: &'static str,
        path: PathBuf,
        class: OsErrorClass,
        #[source]
        source: io::Error,
    },
    #[error("{path} is outside the write scope ({reason:?})", path = path.display())]
    OutOfScope {
        path: PathBuf,
        reason: ScopeViolation,
    },
    #[error("the target {path} or a name that differs only in case exists", path = path.display())]
    TargetExists { path: PathBuf },
    #[error("the source {path} is missing", path = path.display())]
    SourceMissing { path: PathBuf },
    #[error("invalid rename batch: {reason}")]
    InvalidBatch { reason: &'static str },
    #[error("the folder {path} is in use", path = path.display())]
    FolderBusy {
        path: PathBuf,
        holders: Option<Vec<String>>,
    },
    #[error("{0}")]
    Batch(Box<BatchError>),
    #[error("the operation journal is not usable ({reason:?})")]
    Journal { reason: JournalProblem },
    #[error("no usable trash ({reason:?})")]
    TrashUnavailable { reason: TrashUnavailableReason },
    #[error("{path} is not a set-aside folder", path = path.display())]
    NotAnAside { path: PathBuf },
    #[error("{path} still has live pak files", path = path.display())]
    AsideNotInert { path: PathBuf },
    #[error("{path} is named by a journal record", path = path.display())]
    AsideJournaled { path: PathBuf },
}

impl FsError {
    pub fn io(op: &'static str, path: &Path, source: io::Error) -> Self {
        Self::Io {
            op,
            path: path.to_path_buf(),
            class: platform::classify(&source),
            source,
        }
    }

    pub fn class(&self) -> Option<OsErrorClass> {
        match self {
            Self::Io { class, .. } => Some(*class),
            Self::TargetExists { .. } => Some(OsErrorClass::AlreadyExists),
            Self::FolderBusy { .. } => Some(OsErrorClass::InUse),
            Self::Batch(batch) => batch.cause.class(),
            Self::Busy
            | Self::OutOfScope { .. }
            | Self::SourceMissing { .. }
            | Self::InvalidBatch { .. }
            | Self::Journal { .. }
            | Self::TrashUnavailable { .. }
            | Self::NotAnAside { .. }
            | Self::AsideNotInert { .. }
            | Self::AsideJournaled { .. } => None,
        }
    }
}
