use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sevenz_rust2::{ArchiveEntry, Error as SevenError, Password};

use super::error::{ArchiveError, UnsupportedReason};
use super::memory::{self, CoderMemory};
use super::model::{
    ArchiveFormat, ArchiveReader, EntryKind, ExtractPlan, ExtractReport, ExtractTarget, Limits,
    Listing, PlannedFile, RawEntry, RawEntryName,
};
use super::open::{check_counts, check_entry_count, check_total, guarded};
use super::progress::{CancelToken, ProgressCounter};
use super::sink::{CappedSink, FailureSlot, READ_CHUNK, copy_entry};
use super::validate::check_plan;
use super::zip_backend::unix_kind;

const START_HEADER: usize = 32;
const MAX_NEXT_HEADER: u64 = 64 << 20;
const ENCODED_HEADER: u8 = 0x17;
const AES_METHOD: [u8; 4] = [0x06, 0xF1, 0x07, 0x01];
const ATTR_UNIX_EXTENSION: u32 = 0x8000;
const ATTR_REPARSE_POINT: u32 = 0x400;
const NT_TICKS_PER_SEC: u64 = 10_000_000;
const NT_TO_UNIX_SECS: u64 = 11_644_473_600;

pub(crate) struct SevenZipBackend {
    reader: sevenz_rust2::ArchiveReader<BufReader<File>>,
    listing: Listing,
}

pub(crate) fn open(
    mut file: File,
    len: u64,
    limits: &Limits,
) -> Result<Box<dyn ArchiveReader>, ArchiveError> {
    precheck_header(&mut file, len, limits)?;
    file.seek(SeekFrom::Start(0))
        .map_err(|error| ArchiveError::io("read", error))?;
    let mut reader = sevenz_rust2::ArchiveReader::new(BufReader::new(file), Password::empty())
        .map_err(map_error)?;
    // The default is `available_parallelism()`; multi-threaded LZMA2 multiplies decoder memory
    // past the header check.
    reader.set_thread_count(1);
    let archive = reader.archive();
    check_entry_count(archive.files.len() as u64, limits)?;
    let entries = archive
        .files
        .iter()
        .enumerate()
        .map(|(index, entry)| raw_entry(index, entry))
        .collect::<Result<Vec<_>, _>>()?;
    let listing = Listing {
        format: ArchiveFormat::SevenZip,
        entries,
        duplicate_raw: false,
        solid: archive.is_solid,
    };
    check_counts(&listing, limits)?;
    let coders = || archive.blocks.iter().flat_map(|block| &block.coders);
    if coders().any(|coder| coder.encoder_method_id() == AES_METHOD) {
        return Err(ArchiveError::Encrypted);
    }
    check_total(&listing, limits)?;
    let mut needed = 0u64;
    for coder in coders() {
        needed = needed.max(
            match memory::sevenz_coder(coder.encoder_method_id(), coder.properties()) {
                CoderMemory::Known(bytes) => bytes,
                CoderMemory::BadProps => u64::MAX,
                CoderMemory::Unknown => {
                    return Err(ArchiveError::unsupported(UnsupportedReason::Method));
                }
            },
        );
    }
    check_memory(needed, limits)?;
    Ok(Box::new(SevenZipBackend { reader, listing }))
}

/// The crate decodes an encoded header with no memory limit, so its coders are checked first.
fn precheck_header(file: &mut File, len: u64, limits: &Limits) -> Result<(), ArchiveError> {
    file.seek(SeekFrom::Start(0))
        .map_err(|error| ArchiveError::io("read", error))?;
    let mut start = [0u8; START_HEADER];
    file.read_exact(&mut start).map_err(ArchiveError::read)?;
    let offset = u64::from_le_bytes(le8(&start[12..20]));
    let size = u64::from_le_bytes(le8(&start[20..28]));
    if size > MAX_NEXT_HEADER {
        return Err(ArchiveError::corrupt("next header too large"));
    }
    let begin = (START_HEADER as u64)
        .checked_add(offset)
        .filter(|begin| begin.checked_add(size).is_some_and(|end| end <= len))
        .ok_or_else(|| ArchiveError::corrupt("next header past the end"))?;
    if size == 0 {
        return Ok(());
    }
    file.seek(SeekFrom::Start(begin))
        .map_err(|error| ArchiveError::io("read", error))?;
    let mut header = Vec::new();
    file.take(size)
        .read_to_end(&mut header)
        .map_err(ArchiveError::read)?;
    if header.first() == Some(&ENCODED_HEADER) {
        let mut encrypted = false;
        memory::sevenz_header_coders(&header, |id, _| {
            encrypted |= id == AES_METHOD;
            Some(())
        });
        if encrypted {
            return Err(ArchiveError::Encrypted);
        }
        let needed = memory::sevenz_header_memory(&header)
            .ok_or_else(|| ArchiveError::corrupt("encoded header does not parse"))?;
        check_memory(needed, limits)?;
    }
    Ok(())
}

fn le8(bytes: &[u8]) -> [u8; 8] {
    let mut out = [0u8; 8];
    out.copy_from_slice(&bytes[..8]);
    out
}

fn check_memory(needed: u64, limits: &Limits) -> Result<(), ArchiveError> {
    if needed > limits.memory {
        return Err(ArchiveError::MemoryLimit {
            needed,
            limit: limits.memory,
        });
    }
    Ok(())
}

fn raw_entry(index: usize, entry: &ArchiveEntry) -> Result<RawEntry, ArchiveError> {
    let kind = entry_kind(entry);
    Ok(RawEntry {
        index: u32::try_from(index).map_err(|_| ArchiveError::TooManyEntries {
            count: index as u64,
        })?,
        name: RawEntryName::Unicode(entry.name.clone()),
        kind,
        size: if kind == EntryKind::File {
            entry.size
        } else {
            0
        },
        crc32: entry
            .has_crc
            .then_some(entry.crc)
            .and_then(|crc| u32::try_from(crc).ok()),
        mtime: entry
            .has_last_modified_date
            .then(|| nt_time(u64::from(entry.last_modified_date)))
            .flatten(),
        encrypted: false,
        anti: entry.is_anti_item,
    })
}

fn entry_kind(entry: &ArchiveEntry) -> EntryKind {
    if entry.is_anti_item {
        return EntryKind::Special;
    }
    if entry.is_directory {
        return EntryKind::Dir;
    }
    if entry.has_windows_attributes {
        let attributes = entry.windows_attributes;
        if attributes & ATTR_UNIX_EXTENSION != 0 {
            let kind = unix_kind(attributes >> 16);
            if kind != EntryKind::File {
                return kind;
            }
        }
        if attributes & ATTR_REPARSE_POINT != 0 {
            return EntryKind::Link;
        }
    }
    EntryKind::File
}

/// NT file time (100 ns ticks since 1601) without the crate's panicking conversion.
fn nt_time(ticks: u64) -> Option<SystemTime> {
    let since_1601 = Duration::new(
        ticks / NT_TICKS_PER_SEC,
        u32::try_from((ticks % NT_TICKS_PER_SEC) * 100).ok()?,
    );
    UNIX_EPOCH
        .checked_sub(Duration::from_secs(NT_TO_UNIX_SECS))?
        .checked_add(since_1601)
}

impl ArchiveReader for SevenZipBackend {
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

impl SevenZipBackend {
    fn extract_planned(
        &mut self,
        plan: &ExtractPlan,
        target: &mut dyn ExtractTarget,
        progress: &ProgressCounter,
        cancel: &CancelToken,
    ) -> Result<ExtractReport, ArchiveError> {
        let mut report = ExtractReport::default();
        if plan.files.is_empty() {
            progress.flush();
            return Ok(report);
        }
        let planned: HashMap<u32, &PlannedFile> =
            plan.files.iter().map(|file| (file.index, file)).collect();
        // `for_each_entries` hands out references into `archive().files`; their addresses
        // identify the entry index.
        let positions: HashMap<usize, u32> = self
            .listing
            .entries
            .iter()
            .zip(&self.reader.archive().files)
            .map(|(raw, entry)| (std::ptr::from_ref(entry).addr(), raw.index))
            .collect();
        let failure = FailureSlot::default();
        let mut done = HashSet::new();
        let mut buf = vec![0u8; READ_CHUNK];
        let result = self.reader.for_each_entries(|entry, reader| {
            let refuse = |error: ArchiveError| {
                failure.set(error);
                Err(SevenError::Other("extraction stopped".into()))
            };
            if cancel.is_cancelled() {
                return refuse(ArchiveError::Cancelled);
            }
            let Some(&index) = positions.get(&std::ptr::from_ref(entry).addr()) else {
                return refuse(ArchiveError::corrupt("entry outside the listing"));
            };
            let Some(file) = planned.get(&index) else {
                return Ok(true);
            };
            if !done.insert(index) {
                return Ok(true);
            }
            let staged = match target.create(file) {
                Ok(staged) => staged,
                Err(error) => return refuse(ArchiveError::target(error)),
            };
            let mut sink = CappedSink::new(staged, file, cancel, progress, &failure);
            let finished = copy_entry(reader, &mut sink, &mut buf, &failure)
                .and_then(|()| sink.finish(file.mtime));
            match finished {
                Ok(bytes) => {
                    report.bytes += bytes;
                    report.files += 1;
                    Ok(done.len() < planned.len())
                }
                Err(error) => refuse(error),
            }
        });
        if let Err(error) = result {
            return Err(failure.take().unwrap_or_else(|| map_error(error)));
        }
        if done.len() != planned.len() {
            return Err(ArchiveError::corrupt("a planned entry has no data"));
        }
        progress.flush();
        Ok(report)
    }
}

fn map_error(error: SevenError) -> ArchiveError {
    match error {
        SevenError::PasswordRequired | SevenError::MaybeBadPassword(_) => ArchiveError::Encrypted,
        SevenError::UnsupportedCompressionMethod(method) if method.contains("AES") => {
            ArchiveError::Encrypted
        }
        SevenError::UnsupportedCompressionMethod(_)
        | SevenError::Unsupported(_)
        | SevenError::ExternalUnsupported => ArchiveError::unsupported(UnsupportedReason::Method),
        SevenError::MaxMemLimited { max_kb, actaul_kb } => ArchiveError::MemoryLimit {
            needed: (actaul_kb as u64).saturating_mul(1024),
            limit: (max_kb as u64).saturating_mul(1024),
        },
        SevenError::Io(error, _) | SevenError::FileOpen(error, _) => ArchiveError::read(error),
        other => ArchiveError::corrupt(other.to_string()),
    }
}
