use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use atomic_write_file::AtomicWriteFile;

use super::retry::with_retry;
use super::{FsError, ensure_dir};
use crate::platform;

/// atomic-write-file names its temp file `.<name>.` plus 6 random ASCII letters and digits.
const TEMP_SUFFIX_LEN: usize = 6;

/// Replaces `path` with `bytes` via a temp file in the same directory (fsync of file and
/// directory), so readers see either the old or the new content, never a mix.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), FsError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        ensure_dir(parent)?;
    }
    with_retry("write", path, platform::replace_retry_policy, || {
        write_once(path, bytes)
    })
}

/// atomic-write-file marks a write as finished before it renames, so a failed rename leaves its
/// temp file behind (once per retry). Temp files that appeared during this attempt are removed.
fn write_once(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let before = temp_files_for(path);
    let result = write_and_commit(path, bytes);
    if result.is_err()
        && let Some(before) = before
    {
        remove_new_temp_files(path, &before);
    }
    result
}

fn write_and_commit(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = AtomicWriteFile::open(path)?;
    file.write_all(bytes)?;
    file.commit()
}

/// Names of atomic-write-file temp files for `path`; `None` if the folder cannot be listed.
fn temp_files_for(path: &Path) -> Option<HashSet<OsString>> {
    let name = path.file_name()?;
    let entries = fs::read_dir(folder_of(path)).ok()?;
    let temp_names = entries
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .filter(|candidate| is_temp_name(candidate, name))
        .collect();
    Some(temp_names)
}

fn remove_new_temp_files(path: &Path, before: &HashSet<OsString>) {
    let Some(after) = temp_files_for(path) else {
        return;
    };
    for name in after.difference(before) {
        let leftover = folder_of(path).join(name);
        if let Err(error) = fs::remove_file(&leftover) {
            log::warn!("removing temp file {} failed: {error}", leftover.display());
        }
    }
}

fn folder_of(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

fn is_temp_name(candidate: &OsStr, name: &OsStr) -> bool {
    let suffix = candidate
        .as_encoded_bytes()
        .strip_prefix(b".")
        .and_then(|rest| rest.strip_prefix(name.as_encoded_bytes()))
        .and_then(|rest| rest.strip_prefix(b"."));
    suffix.is_some_and(|suffix| {
        suffix.len() == TEMP_SUFFIX_LEN && suffix.iter().all(u8::is_ascii_alphanumeric)
    })
}
