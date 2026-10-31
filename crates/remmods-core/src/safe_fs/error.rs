use std::io;
use std::path::{Path, PathBuf};

use crate::platform::{self, OsErrorClass};

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
            Self::Busy => None,
            Self::Io { class, .. } => Some(*class),
        }
    }
}
