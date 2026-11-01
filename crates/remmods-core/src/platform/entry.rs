//! Directory entries as the OS sees them: raw name units, the path form handed to `std::fs` and
//! the identity of an entry.

use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};

/// A file name exactly as the OS stores it: bytes on Unix, UTF-16 units on Windows.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RawName {
    Bytes(Vec<u8>),
    Wide(Vec<u16>),
}

/// Identity of a directory entry: device and inode on Unix, volume serial and file index on Windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FileId {
    pub volume: u64,
    pub index: u128,
}

pub fn raw_name(name: &OsStr) -> RawName {
    imp::raw_name(name)
}

/// The form of `path` to hand to `std::fs`. Windows: verbatim (`\\?\`), so names Win32 would
/// normalise (`Foo.`, `Foo `) reach the right entry; other OSes: the lexical absolute path.
pub fn fs_path(path: &Path) -> io::Result<PathBuf> {
    imp::fs_path(path)
}

/// Identity of the entry itself; a final symlink or junction is not followed.
pub fn file_id(path: &Path) -> io::Result<FileId> {
    imp::file_id(path)
}

#[cfg(unix)]
mod imp {
    use std::ffi::OsStr;
    use std::io;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::MetadataExt;
    use std::path::{Path, PathBuf};

    use super::{FileId, RawName};

    pub(super) fn raw_name(name: &OsStr) -> RawName {
        RawName::Bytes(name.as_bytes().to_vec())
    }

    pub(super) fn fs_path(path: &Path) -> io::Result<PathBuf> {
        std::path::absolute(path)
    }

    pub(super) fn file_id(path: &Path) -> io::Result<FileId> {
        let metadata = std::fs::symlink_metadata(path)?;
        Ok(FileId {
            volume: metadata.dev(),
            index: u128::from(metadata.ino()),
        })
    }
}

#[cfg(windows)]
mod imp {
    use std::ffi::{OsStr, OsString};
    use std::fs::OpenOptions;
    use std::io;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use std::path::{Component, Path, PathBuf, Prefix};

    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        GetFileInformationByHandle,
    };

    use super::{FileId, RawName};

    const VERBATIM: &str = r"\\?\";
    const NT_OBJECT: &str = r"\??\";
    const VERBATIM_UNC: &str = r"\\?\UNC\";

    pub(super) fn raw_name(name: &OsStr) -> RawName {
        RawName::Wide(name.encode_wide().collect())
    }

    pub(super) fn fs_path(path: &Path) -> io::Result<PathBuf> {
        let bytes = path.as_os_str().as_encoded_bytes();
        if bytes.starts_with(VERBATIM.as_bytes()) || bytes.starts_with(NT_OBJECT.as_bytes()) {
            return Ok(path.to_path_buf());
        }
        let absolute = std::path::absolute(path)?;
        let kind = match absolute.components().next() {
            Some(Component::Prefix(prefix)) => prefix.kind(),
            _ => return Ok(absolute),
        };
        Ok(match kind {
            Prefix::Disk(_) => prefixed(VERBATIM, absolute.as_os_str()),
            Prefix::UNC(..) => {
                // `\\server\share\rest` becomes `\\?\UNC\server\share\rest`.
                let rest: Vec<u16> = absolute.as_os_str().encode_wide().skip(2).collect();
                prefixed(VERBATIM_UNC, &OsString::from_wide(&rest))
            }
            // Device namespace paths (`\\.\`) are not normalised by Win32 either.
            _ => absolute,
        })
    }

    fn prefixed(prefix: &str, rest: &OsStr) -> PathBuf {
        let mut joined = OsString::from(prefix);
        joined.push(rest);
        PathBuf::from(joined)
    }

    pub(super) fn file_id(path: &Path) -> io::Result<FileId> {
        let file = OpenOptions::new()
            .access_mode(0)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)?;
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        // SAFETY: the handle stays open for the whole call and `info` is a valid, writable struct.
        let ok = unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(FileId {
            volume: u64::from(info.dwVolumeSerialNumber),
            index: (u128::from(info.nFileIndexHigh) << 32) | u128::from(info.nFileIndexLow),
        })
    }
}

#[cfg(not(any(unix, windows)))]
mod imp {
    use std::ffi::OsStr;
    use std::io;
    use std::path::{Path, PathBuf};

    use super::{FileId, RawName};

    pub(super) fn raw_name(name: &OsStr) -> RawName {
        RawName::Bytes(name.as_encoded_bytes().to_vec())
    }

    pub(super) fn fs_path(path: &Path) -> io::Result<PathBuf> {
        std::path::absolute(path)
    }

    pub(super) fn file_id(_path: &Path) -> io::Result<FileId> {
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }
}
