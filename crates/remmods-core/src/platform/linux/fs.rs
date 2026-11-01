// The raw no-replace rename; only safe_fs calls it, after its scope checks.
#![allow(clippy::disallowed_methods)]

use std::io;
use std::path::Path;

use rustix::fs::{CWD, RenameFlags, renameat_with};
use rustix::io::Errno;

use super::super::NoReplace;

/// EINVAL: the file system does not know RENAME_NOREPLACE (same-folder renames cannot hit the
/// other EINVAL causes). ENOSYS: the kernel predates renameat2.
pub(in crate::platform) fn rename_noreplace(from: &Path, to: &Path) -> io::Result<NoReplace> {
    match renameat_with(CWD, from, CWD, to, RenameFlags::NOREPLACE) {
        Ok(()) => Ok(NoReplace::Done),
        Err(Errno::INVAL | Errno::NOSYS) => Ok(NoReplace::FlagUnsupported),
        Err(errno) => Err(io::Error::from(errno)),
    }
}

/// Linux has no sharing modes: an open file never blocks a rename.
pub(in crate::platform) fn lock_probe(_path: &Path, _is_dir: bool) -> io::Result<()> {
    Ok(())
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
        fs::write(root.join("Y").join("y.pak"), b"y").expect("write");

        let file = rename_noreplace(&root.join("a.pak"), &root.join("b.pak"));
        let folder = rename_noreplace(&root.join("X"), &root.join("Y"));

        for result in [file, folder] {
            match result {
                Err(error) => assert_eq!(classify(&error), OsErrorClass::AlreadyExists),
                Ok(outcome) => assert_eq!(outcome, NoReplace::FlagUnsupported),
            }
        }
        assert_eq!(fs::read(root.join("a.pak")).expect("read"), b"a");
        assert_eq!(fs::read(root.join("b.pak")).expect("read"), b"b");
        assert!(root.join("X").is_dir());
        assert!(root.join("Y").join("y.pak").is_file());
    }
}
