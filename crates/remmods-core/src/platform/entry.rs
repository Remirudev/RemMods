//! Directory entries as the OS sees them: raw name units, the path form handed to `std::fs` and
//! the identity of an entry.

use std::ffi::{OsStr, OsString};
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

/// The OsString a [`RawName`] denotes; None when its form does not match the OS (bytes on
/// Windows, UTF-16 units on Unix).
pub fn os_string(raw: &RawName) -> Option<OsString> {
    imp::os_string(raw)
}

/// `name` without its first `prefix` and last `suffix` encoded bytes. None when prefix + suffix
/// exceeds the length or a removed byte is not ASCII.
pub fn os_str_cut(name: &OsStr, prefix: usize, suffix: usize) -> Option<OsString> {
    let bytes = name.as_encoded_bytes();
    let end = bytes.len().checked_sub(suffix)?;
    let head = bytes.get(..prefix)?;
    let tail = bytes.get(end..)?;
    let middle = bytes.get(prefix..end)?;
    if !head.is_ascii() || !tail.is_ascii() {
        return None;
    }
    // SAFETY: `middle` comes from a valid OsStr and both cuts sit right after or right before an
    // ASCII byte, which the encoded-bytes contract allows as split points.
    let cut = unsafe { OsStr::from_encoded_bytes_unchecked(middle) };
    Some(cut.to_os_string())
}

/// The longest prefix of `name` that is at most `max_bytes` encoded bytes and at most
/// `max_units` UTF-16 units, cut only where the OS form allows it (any byte on Unix, never inside
/// a surrogate pair on Windows).
pub(crate) fn os_str_prefix(name: &OsStr, max_bytes: usize, max_units: usize) -> OsString {
    let cut = match raw_name(name) {
        RawName::Bytes(mut bytes) => {
            bytes.truncate(max_bytes.min(max_units));
            RawName::Bytes(bytes)
        }
        RawName::Wide(units) => RawName::Wide(wide_prefix(&units, max_bytes, max_units)),
    };
    os_string(&cut).unwrap_or_default()
}

fn wide_prefix(units: &[u16], max_bytes: usize, max_units: usize) -> Vec<u16> {
    let (mut bytes, mut index) = (0, 0);
    while index < units.len() {
        let unit = units[index];
        let is_pair = (0xD800..0xDC00).contains(&unit)
            && units
                .get(index + 1)
                .is_some_and(|next| (0xDC00..0xE000).contains(next));
        let (width, encoded) = match unit {
            _ if is_pair => (2, 4),
            0..0x80 => (1, 1),
            0x80..0x800 => (1, 2),
            _ => (1, 3),
        };
        if index + width > max_units || bytes + encoded > max_bytes {
            break;
        }
        bytes += encoded;
        index += width;
    }
    units[..index].to_vec()
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
    use std::ffi::{OsStr, OsString};
    use std::io;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    use std::os::unix::fs::MetadataExt;
    use std::path::{Path, PathBuf};

    use super::{FileId, RawName};

    pub(super) fn raw_name(name: &OsStr) -> RawName {
        RawName::Bytes(name.as_bytes().to_vec())
    }

    pub(super) fn os_string(raw: &RawName) -> Option<OsString> {
        match raw {
            RawName::Bytes(bytes) => Some(OsString::from_vec(bytes.clone())),
            RawName::Wide(_) => None,
        }
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

    pub(super) fn os_string(raw: &RawName) -> Option<OsString> {
        match raw {
            RawName::Wide(units) => Some(OsString::from_wide(units)),
            RawName::Bytes(_) => None,
        }
    }

    pub(super) fn fs_path(path: &Path) -> io::Result<PathBuf> {
        let bytes = path.as_os_str().as_encoded_bytes();
        if bytes.starts_with(VERBATIM.as_bytes()) || bytes.starts_with(NT_OBJECT.as_bytes()) {
            return Ok(path.to_path_buf());
        }
        // `std::path::absolute` runs GetFullPathNameW, which drops trailing dots from components
        // (`Foo.\a.pak` becomes `Foo\a.pak`), so the verbatim form is built lexically instead.
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };
        let mut components = absolute.components();
        let mut verbatim = match components.next() {
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                Prefix::Disk(letter) => {
                    let mut root = OsString::from(VERBATIM);
                    root.push(format!("{}:", char::from(letter)));
                    root
                }
                Prefix::UNC(server, share) => {
                    let mut root = OsString::from(VERBATIM_UNC);
                    root.push(server);
                    root.push(r"\");
                    root.push(share);
                    root
                }
                // Device namespace paths (`\\.\`) are not normalised by Win32 either.
                _ => return Ok(absolute),
            },
            _ => return std::path::absolute(path),
        };
        let mut parts: Vec<&OsStr> = Vec::new();
        for component in components {
            match component {
                Component::Normal(part) => parts.push(part),
                Component::ParentDir => {
                    parts.pop();
                }
                Component::Prefix(_) | Component::RootDir | Component::CurDir => {}
            }
        }
        if parts.is_empty() {
            verbatim.push(r"\");
        }
        for part in parts {
            verbatim.push(r"\");
            verbatim.push(part);
        }
        Ok(PathBuf::from(verbatim))
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
    use std::ffi::{OsStr, OsString};
    use std::io;
    use std::path::{Path, PathBuf};

    use super::{FileId, RawName};

    pub(super) fn raw_name(name: &OsStr) -> RawName {
        RawName::Bytes(name.as_encoded_bytes().to_vec())
    }

    pub(super) fn os_string(raw: &RawName) -> Option<OsString> {
        match raw {
            RawName::Bytes(bytes) => String::from_utf8(bytes.clone()).ok().map(OsString::from),
            RawName::Wide(_) => None,
        }
    }

    pub(super) fn fs_path(path: &Path) -> io::Result<PathBuf> {
        std::path::absolute(path)
    }

    pub(super) fn file_id(_path: &Path) -> io::Result<FileId> {
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }
}
