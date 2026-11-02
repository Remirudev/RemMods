use std::fs::File;
use std::io::{BufReader, Read};
use std::time::SystemTime;

use chrono::{Local, NaiveDate, TimeZone};
use zip::read::ZipFileEntry;
use zip::result::ZipError;
use zip::{CompressionMethod, ZipArchive};

use super::error::{ArchiveError, UnsupportedReason};
use super::memory;
use super::model::{
    ArchiveFormat, ArchiveReader, EntryKind, ExtractPlan, ExtractReport, ExtractTarget, Limits,
    Listing, RawEntry, RawEntryName,
};
use super::open::{check_counts, check_entry_count, check_total, guarded};
use super::progress::{CancelToken, ProgressCounter};
use super::sink::{CappedSink, FailureSlot, READ_CHUNK, copy_entry};
use super::validate::check_plan;

/// Enough for the LZMA, PPMd and xz property bytes that `memory` parses.
const PROPS_HEAD: usize = 64;
/// FILE_ATTRIBUTE_REPARSE_POINT in the DOS half of the external attributes.
const DOS_REPARSE_POINT: u32 = 0x400;

pub(crate) struct ZipBackend {
    archive: ZipArchive<BufReader<File>>,
    listing: Listing,
}

pub(crate) fn open(file: File, limits: &Limits) -> Result<Box<dyn ArchiveReader>, ArchiveError> {
    let mut archive = ZipArchive::new(BufReader::new(file)).map_err(map_zip_error)?;
    check_entry_count(archive.len() as u64, limits)?;
    let mut entries = Vec::with_capacity(archive.len());
    let mut methods = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let entry = archive.by_index_data(index).map_err(map_zip_error)?;
        methods.push(entry.compression());
        entries.push(raw_entry(index, &entry)?);
    }
    let listing = Listing {
        format: ArchiveFormat::Zip,
        entries,
        duplicate_raw: archive.has_duplicate_names(),
        solid: false,
    };
    check_counts(&listing, limits)?;
    check_total(&listing, limits)?;
    if archive.has_overlapping_files().map_err(map_zip_error)? {
        return Err(ArchiveError::corrupt("overlapping entries"));
    }
    for (entry, &method) in listing.entries.iter().zip(&methods) {
        if entry.kind == EntryKind::File {
            check_method(&mut archive, entry, method, limits)?;
        }
    }
    Ok(Box::new(ZipBackend { archive, listing }))
}

impl ArchiveReader for ZipBackend {
    fn listing(&self) -> &Listing {
        &self.listing
    }

    fn extract(
        &mut self,
        plan: &ExtractPlan,
        target: &mut dyn ExtractTarget,
        progress: &ProgressCounter,
        cancel: &CancelToken,
    ) -> Result<ExtractReport, ArchiveError> {
        check_plan(&self.listing, plan)?;
        guarded(|| self.extract_planned(plan, target, progress, cancel))
    }
}

impl ZipBackend {
    fn extract_planned(
        &mut self,
        plan: &ExtractPlan,
        target: &mut dyn ExtractTarget,
        progress: &ProgressCounter,
        cancel: &CancelToken,
    ) -> Result<ExtractReport, ArchiveError> {
        let failure = FailureSlot::default();
        let mut buf = vec![0u8; READ_CHUNK];
        let mut report = ExtractReport::default();
        for file in &plan.files {
            if cancel.is_cancelled() {
                return Err(ArchiveError::Cancelled);
            }
            let staged = target.create(file).map_err(ArchiveError::target)?;
            let mut sink = CappedSink::new(staged, file, cancel, progress, &failure);
            let mut reader = self
                .archive
                .by_index(file.index as usize)
                .map_err(map_zip_error)?;
            copy_entry(&mut reader, &mut sink, &mut buf, &failure)
                .map_err(|error| size_cap(error, file.index))?;
            drop(reader);
            report.bytes += sink.finish(file.mtime)?;
            report.files += 1;
        }
        progress.flush();
        Ok(report)
    }
}

fn raw_entry(index: usize, entry: &ZipFileEntry<'_>) -> Result<RawEntry, ArchiveError> {
    let kind = entry_kind(entry);
    Ok(RawEntry {
        index: u32::try_from(index).map_err(|_| ArchiveError::TooManyEntries {
            count: index as u64,
        })?,
        name: RawEntryName::Bytes(entry.name_raw().to_vec()),
        kind,
        size: if kind == EntryKind::File {
            entry.size()
        } else {
            0
        },
        crc32: Some(entry.crc32()),
        mtime: entry.last_modified().and_then(local_time),
        encrypted: entry.encrypted() || entry.compression() == CompressionMethod::AES,
        anti: false,
    })
}

fn entry_kind(entry: &ZipFileEntry<'_>) -> EntryKind {
    if entry.is_dir() {
        return EntryKind::Dir;
    }
    let attributes = entry.external_attributes();
    if attributes >> 16 == 0 && attributes & DOS_REPARSE_POINT != 0 {
        return EntryKind::Link;
    }
    entry.unix_mode().map_or(EntryKind::File, unix_kind)
}

/// `st_mode` file-type bits, shared with the 7z and RAR backends.
pub(crate) fn unix_kind(mode: u32) -> EntryKind {
    match mode & 0o170_000 {
        0o120_000 => EntryKind::Link,
        0 | 0o100_000 => EntryKind::File,
        0o040_000 => EntryKind::Dir,
        _ => EntryKind::Special,
    }
}

/// zip stores DOS wall-clock time without a zone; read it as local time.
fn local_time(stamp: zip::DateTime) -> Option<SystemTime> {
    let naive = NaiveDate::from_ymd_opt(
        i32::from(stamp.year()),
        u32::from(stamp.month()),
        u32::from(stamp.day()),
    )?
    .and_hms_opt(
        u32::from(stamp.hour()),
        u32::from(stamp.minute()),
        u32::from(stamp.second()),
    )?;
    Local
        .from_local_datetime(&naive)
        .earliest()
        .map(SystemTime::from)
}

fn check_method(
    archive: &mut ZipArchive<BufReader<File>>,
    entry: &RawEntry,
    method: CompressionMethod,
    limits: &Limits,
) -> Result<(), ArchiveError> {
    let plain = [
        CompressionMethod::STORE,
        CompressionMethod::DEFLATE,
        CompressionMethod::DEFLATE64,
        CompressionMethod::BZIP2,
    ];
    let parse: fn(&[u8]) -> Option<u64> = if plain.contains(&method) {
        return Ok(());
    } else if method == CompressionMethod::LZMA {
        memory::zip_lzma_memory
    } else if method == CompressionMethod::XZ {
        memory::zip_xz_memory
    } else if method == CompressionMethod::PPMD {
        memory::zip_ppmd_memory
    } else {
        return Err(ArchiveError::unsupported(UnsupportedReason::Method));
    };
    let mut head = Vec::with_capacity(PROPS_HEAD);
    archive
        .by_index_raw(entry.index as usize)
        .map_err(map_zip_error)?
        .take(PROPS_HEAD as u64)
        .read_to_end(&mut head)
        .map_err(ArchiveError::read)?;
    let needed = parse(&head).unwrap_or(u64::MAX);
    if needed > limits.memory {
        return Err(ArchiveError::MemoryLimit {
            needed,
            limit: limits.memory,
        });
    }
    Ok(())
}

/// zip's own reader stops one read past the declared size with this message; it is the same
/// refusal as the sink's cap.
fn size_cap(error: ArchiveError, index: u32) -> ArchiveError {
    match error {
        ArchiveError::Corrupt { detail } if detail.contains("larger than its declared") => {
            ArchiveError::SizeMismatch { index }
        }
        other => other,
    }
}

fn map_zip_error(error: ZipError) -> ArchiveError {
    match error {
        ZipError::Io(error) => ArchiveError::read(error),
        ZipError::UnsupportedArchive(_) | ZipError::InvalidArchive(_)
            if error.to_string().to_lowercase().contains("multi-disk") =>
        {
            ArchiveError::unsupported(UnsupportedReason::MultiVolume)
        }
        ZipError::CompressionMethodNotSupported(_) => {
            ArchiveError::unsupported(UnsupportedReason::Method)
        }
        ZipError::InvalidPassword => ArchiveError::Encrypted,
        other => ArchiveError::corrupt(other.to_string()),
    }
}
