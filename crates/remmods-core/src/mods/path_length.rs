//! W1: paths the game (MAX_PATH) may fail to open, a warning only (spec §4.6).

use super::issues::ModIssue;
use crate::safe_fs::names::{WINDOWS_PATH_MAX_DIR, WINDOWS_PATH_MAX_FILE};

/// `longest` is the largest file length, or the directory length when no file is too long.
pub(crate) fn path_too_long(longest_file: usize, longest_dir: usize) -> Option<ModIssue> {
    let longest = if longest_file > WINDOWS_PATH_MAX_FILE {
        longest_file
    } else if longest_dir > WINDOWS_PATH_MAX_DIR {
        longest_dir
    } else {
        return None;
    };
    Some(ModIssue::PathTooLong {
        longest: u32::try_from(longest).unwrap_or(u32::MAX),
    })
}
