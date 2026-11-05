use std::io;

use crate::platform::{self, OsErrorClass};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsupportedReason {
    MultiVolume,
    SelfExtracting,
    Method,
    RarDisabled,
}

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("not an archive")]
    NotAnArchive,
    #[error("unsupported archive ({reason:?})")]
    Unsupported { reason: UnsupportedReason },
    #[error("archive is encrypted")]
    Encrypted,
    /// `detail` goes to the log only.
    #[error("archive is corrupt: {detail}")]
    Corrupt { detail: String },
    #[error("entry {index} does not match its declared size")]
    SizeMismatch { index: u32 },
    #[error("decoder needs {needed} bytes, limit {limit}")]
    MemoryLimit { needed: u64, limit: u64 },
    #[error("{count} entries")]
    TooManyEntries { count: u64 },
    #[error("{bytes} bytes declared")]
    TooLarge { bytes: u64 },
    #[error("cancelled")]
    Cancelled,
    #[error("{op} failed: {source}")]
    Io {
        op: &'static str,
        class: OsErrorClass,
        #[source]
        source: io::Error,
    },
    #[error("writing to staging failed: {source}")]
    Target {
        class: OsErrorClass,
        #[source]
        source: io::Error,
    },
}

impl ArchiveError {
    pub(crate) fn corrupt(detail: impl Into<String>) -> Self {
        Self::Corrupt {
            detail: detail.into(),
        }
    }

    pub(crate) fn unsupported(reason: UnsupportedReason) -> Self {
        Self::Unsupported { reason }
    }

    pub(crate) fn io(op: &'static str, source: io::Error) -> Self {
        Self::Io {
            op,
            class: platform::classify(&source),
            source,
        }
    }

    pub(crate) fn target(source: io::Error) -> Self {
        Self::Target {
            class: platform::classify(&source),
            source,
        }
    }

    /// A failed read of entry data: damaged data (CRC, truncated stream) is `Corrupt`, anything
    /// else came from the source file.
    pub(crate) fn read(source: io::Error) -> Self {
        match source.kind() {
            io::ErrorKind::InvalidData | io::ErrorKind::UnexpectedEof | io::ErrorKind::Other => {
                Self::corrupt(source.to_string())
            }
            _ => Self::io("read", source),
        }
    }
}
