use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Read};
use std::panic::{self, AssertUnwindSafe};
use std::path::Path;

use super::error::{ArchiveError, UnsupportedReason};
use super::folder_backend;
use super::model::{ArchiveFormat, ArchiveReader, Limits, Listing, SourceFingerprint};
use super::sevenz_backend;
use super::zip_backend;
use crate::platform;

const ZIP_LOCAL: [u8; 4] = [0x50, 0x4B, 0x03, 0x04];
const ZIP_EMPTY: [u8; 4] = [0x50, 0x4B, 0x05, 0x06];
const ZIP_SPANNED: [u8; 4] = [0x50, 0x4B, 0x07, 0x08];
const SEVEN_ZIP: [u8; 6] = [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C];
const RAR4: [u8; 7] = [0x52, 0x61, 0x72, 0x21, 0x1A, 0x07, 0x00];
const RAR5: [u8; 8] = [0x52, 0x61, 0x72, 0x21, 0x1A, 0x07, 0x01, 0x00];

enum Magic {
    Zip,
    SevenZip,
    Rar,
}

/// Opens an archive file and lists it (no payload decompression).
pub fn open(path: &Path, limits: &Limits) -> Result<Box<dyn ArchiveReader>, ArchiveError> {
    let fs_path = platform::fs_path(path).map_err(|error| ArchiveError::io("open", error))?;
    let meta = fs::symlink_metadata(&fs_path).map_err(|error| ArchiveError::io("open", error))?;
    if !meta.file_type().is_file() {
        return Err(ArchiveError::NotAnArchive);
    }
    if path
        .file_name()
        .is_some_and(|name| volume_name_kind(name).is_some())
    {
        return Err(ArchiveError::unsupported(UnsupportedReason::MultiVolume));
    }
    let mut file = File::open(&fs_path).map_err(|error| ArchiveError::io("open", error))?;
    let magic = read_magic(&mut file)?;
    guarded(|| match magic {
        Magic::Zip => zip_backend::open(file, limits),
        Magic::SevenZip => sevenz_backend::open(file, meta.len(), limits),
        Magic::Rar => open_rar(file, limits),
    })
}

/// A folder source: `path` must be a real directory; the walk never follows links.
pub fn open_folder(path: &Path, limits: &Limits) -> Result<Box<dyn ArchiveReader>, ArchiveError> {
    let fs_path = platform::fs_path(path).map_err(|error| ArchiveError::io("open", error))?;
    let meta = fs::symlink_metadata(&fs_path).map_err(|error| ArchiveError::io("open", error))?;
    if !meta.file_type().is_dir() {
        return Err(ArchiveError::NotAnArchive);
    }
    guarded(|| folder_backend::open(&fs_path, limits))
}

pub fn fingerprint(path: &Path) -> io::Result<SourceFingerprint> {
    let meta = fs::symlink_metadata(platform::fs_path(path)?)?;
    Ok(SourceFingerprint {
        len: meta.len(),
        modified: meta.modified().ok(),
    })
}

/// The format of a multi-volume part name (`x.z01`, `x.zip.001`, `x.7z.001`, `x.r00`,
/// `x.part2.rar`, `x.part1.rar`), case-insensitive; None for any other name.
pub fn volume_name_kind(file_name: &OsStr) -> Option<ArchiveFormat> {
    let name = file_name.to_string_lossy().to_lowercase();
    let (stem, ext) = name.rsplit_once('.')?;
    if ext
        .strip_prefix('z')
        .is_some_and(|digits| is_digits(digits, 2) && digits != "00")
    {
        return Some(ArchiveFormat::Zip);
    }
    if ext
        .strip_prefix('r')
        .is_some_and(|digits| is_digits(digits, 2))
    {
        return Some(ArchiveFormat::Rar);
    }
    if is_digits(ext, 3) && ext != "000" {
        return match stem.rsplit_once('.').map(|(_, inner)| inner) {
            Some("zip") => Some(ArchiveFormat::Zip),
            Some("7z") => Some(ArchiveFormat::SevenZip),
            Some("rar") => Some(ArchiveFormat::Rar),
            _ => None,
        };
    }
    if ext == "rar" {
        let part = stem.rsplit_once('.').map(|(_, inner)| inner)?;
        let digits = part.strip_prefix("part")?;
        let all_digits = !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit());
        let number: u32 = digits.parse().ok()?;
        return (all_digits && number >= 1).then_some(ArchiveFormat::Rar);
    }
    None
}

/// Runs a backend call; a panic inside a crate becomes `Corrupt` instead of unwinding further.
pub(crate) fn guarded<T>(
    call: impl FnOnce() -> Result<T, ArchiveError>,
) -> Result<T, ArchiveError> {
    panic::catch_unwind(AssertUnwindSafe(call))
        .unwrap_or_else(|_| Err(ArchiveError::corrupt("panic")))
}

/// Shared open checks in spec order: count, encryption, then the size total.
pub(crate) fn check_counts(listing: &Listing, limits: &Limits) -> Result<(), ArchiveError> {
    check_entry_count(listing.entries.len() as u64, limits)?;
    if listing.entries.iter().any(|entry| entry.encrypted) {
        return Err(ArchiveError::Encrypted);
    }
    Ok(())
}

pub(crate) fn check_entry_count(count: u64, limits: &Limits) -> Result<(), ArchiveError> {
    if count > u64::from(limits.max_entries) {
        return Err(ArchiveError::TooManyEntries { count });
    }
    Ok(())
}

pub(crate) fn check_total(listing: &Listing, limits: &Limits) -> Result<(), ArchiveError> {
    let bytes = listing
        .entries
        .iter()
        .fold(0u64, |sum, entry| sum.saturating_add(entry.size));
    if bytes > limits.max_total {
        return Err(ArchiveError::TooLarge { bytes });
    }
    Ok(())
}

fn is_digits(text: &str, len: usize) -> bool {
    text.len() == len && text.bytes().all(|byte| byte.is_ascii_digit())
}

fn read_magic(file: &mut File) -> Result<Magic, ArchiveError> {
    let mut head = [0u8; 8];
    let mut filled = 0;
    while filled < head.len() {
        match file.read(&mut head[filled..]) {
            Ok(0) => break,
            Ok(read) => filled += read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(ArchiveError::io("read", error)),
        }
    }
    let head = &head[..filled];
    if head.starts_with(&ZIP_SPANNED) {
        return Err(ArchiveError::unsupported(UnsupportedReason::MultiVolume));
    }
    if head.starts_with(&ZIP_LOCAL) || head.starts_with(&ZIP_EMPTY) {
        return Ok(Magic::Zip);
    }
    if head.starts_with(&SEVEN_ZIP) {
        return Ok(Magic::SevenZip);
    }
    if head.starts_with(&RAR4) || head.starts_with(&RAR5) {
        return Ok(Magic::Rar);
    }
    Err(ArchiveError::NotAnArchive)
}

#[cfg(feature = "rar-builtin")]
fn open_rar(file: File, limits: &Limits) -> Result<Box<dyn ArchiveReader>, ArchiveError> {
    super::rar_backend::open(file, limits)
}

#[cfg(not(feature = "rar-builtin"))]
fn open_rar(_file: File, _limits: &Limits) -> Result<Box<dyn ArchiveReader>, ArchiveError> {
    Err(ArchiveError::unsupported(UnsupportedReason::RarDisabled))
}
