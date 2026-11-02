//! Reading a profile file the user picked: size cap and decoding (spec §5.9). Only read: never
//! written, renamed or locked.

use std::path::Path;

use serde_json::Value;

use super::error::{ImportProblem, ProfileError};
use crate::mods::decode_modinfo_text;
use crate::safe_fs::FsError;

pub const IMPORT_MAX_BYTES: u64 = 4 * 1024 * 1024;

/// Merge counts of one import.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct MergeCounts {
    pub added: u32,
    pub updated: u32,
    pub unchanged: u32,
}

/// Size check, then a plain read (no lock, no write).
pub(crate) fn read_import_file(file: &Path) -> Result<Vec<u8>, ProfileError> {
    let fs_path =
        crate::platform::fs_path(file).map_err(|error| FsError::io("read", file, error))?;
    let metadata = std::fs::metadata(&fs_path).map_err(|error| FsError::io("read", file, error))?;
    if metadata.len() > IMPORT_MAX_BYTES {
        return Err(ProfileError::Unparsable(ImportProblem::TooLarge));
    }
    let bytes = std::fs::read(&fs_path).map_err(|error| FsError::io("read", file, error))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > IMPORT_MAX_BYTES {
        return Err(ProfileError::Unparsable(ImportProblem::TooLarge));
    }
    Ok(bytes)
}

/// BOM sniffing (UTF-8/16/32) and lossy UTF-8, then JSON.
pub(crate) fn decode_json(bytes: &[u8]) -> Result<Value, ProfileError> {
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > IMPORT_MAX_BYTES {
        return Err(ProfileError::Unparsable(ImportProblem::TooLarge));
    }
    serde_json::from_str(&decode_modinfo_text(bytes))
        .map_err(|_| ProfileError::Unparsable(ImportProblem::NotJson))
}
