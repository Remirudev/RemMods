// The raw no-replace renames and the trash folder/file creation; only safe_fs calls them, after
// its scope checks.
#![allow(clippy::disallowed_methods)]

use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;

use windows_sys::Win32::Storage::FileSystem::{
    DELETE, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE,
    FILE_SHARE_READ, FILE_SHARE_WRITE, MoveFileExW,
};

use super::super::{NoReplace, fs_path};

/// Flags 0: no MOVEFILE_REPLACE_EXISTING (an existing target fails with 80/183) and no
/// MOVEFILE_COPY_ALLOWED (another volume fails with 17 instead of copying).
pub(in crate::platform) fn rename_noreplace(from: &Path, to: &Path) -> io::Result<NoReplace> {
    let from = wide_nul(fs_path(from)?.as_os_str());
    let to = wide_nul(fs_path(to)?.as_os_str());
    // SAFETY: both buffers are NUL-terminated UTF-16 paths that outlive the call.
    let ok = unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 0) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(NoReplace::Done)
}

/// MoveFileExW moves between folders of one volume the same way; flags 0 never replace or copy.
pub(in crate::platform) fn rename_noreplace_across(
    from: &Path,
    to: &Path,
) -> io::Result<NoReplace> {
    rename_noreplace(from, to)
}

pub(in crate::platform) fn create_private_dir(path: &Path) -> io::Result<()> {
    fs::create_dir(fs_path(path)?)
}

/// CREATE_NEW: fails with ERROR_FILE_EXISTS when the name is taken.
pub(in crate::platform) fn create_new_file(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(fs_path(path)?)
}

/// The game opens its paks without FILE_SHARE_DELETE; asking for DELETE access then fails with a
/// sharing violation while granting every share mode ourselves.
pub(in crate::platform) fn lock_probe(path: &Path, is_dir: bool) -> io::Result<()> {
    let mut flags = FILE_FLAG_OPEN_REPARSE_POINT;
    if is_dir {
        flags |= FILE_FLAG_BACKUP_SEMANTICS;
    }
    OpenOptions::new()
        .access_mode(DELETE)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(flags)
        .open(fs_path(path)?)
        .map(drop)
}

fn wide_nul(text: &OsStr) -> Vec<u16> {
    text.encode_wide().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::super::super::{OsErrorClass, classify};
    use super::*;

    #[test]
    fn an_existing_file_or_folder_is_never_replaced() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path();
        fs::write(root.join("a.pak"), b"a").expect("write");
        fs::write(root.join("b.pak"), b"b").expect("write");
        fs::create_dir(root.join("X")).expect("mkdir");
        fs::create_dir(root.join("Y")).expect("mkdir");

        let file = rename_noreplace(&root.join("a.pak"), &root.join("b.pak"));
        let folder = rename_noreplace(&root.join("X"), &root.join("Y"));

        for result in [file, folder] {
            let error = result.expect_err("refused");
            // PLAN §11: which of 80 and 183 MoveFileExW reports is logged on the runner.
            println!(
                "MoveFileExW onto an existing target: {:?}",
                error.raw_os_error()
            );
            assert_eq!(classify(&error), OsErrorClass::AlreadyExists);
        }
        assert_eq!(fs::read(root.join("b.pak")).expect("read"), b"b");
        assert!(root.join("X").is_dir());
    }
}
