//! RAR through `rars` (feature `rar-builtin`; without it `open` refuses with `RarDisabled`).

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Write};
use std::sync::{Arc, Mutex, PoisonError};

use rars::{
    Archive, ArchiveMember, ArchiveMemberDetail, ArchiveReadOptions, AttrSource, Error as RarError,
    ErrorKind as RarErrorKind, ExtractionDecision,
};

use super::error::{ArchiveError, UnsupportedReason};
use super::model::{
    ArchiveFormat, ArchiveReader, EntryKind, ExtractPlan, ExtractReport, ExtractTarget, Limits,
    Listing, PlannedFile, RawEntry, RawEntryName,
};
use super::open::{check_counts, check_entry_count, check_total, guarded};
use super::progress::{CancelToken, ProgressCounter};
use super::sink::{CappedSink, FailureSlot};
use super::validate::check_plan;
use super::zip_backend::unix_kind;

pub(crate) struct RarBackend {
    archive: Archive,
    listing: Listing,
    members: Vec<MemberInfo>,
    memory: u64,
}

/// What the extract callback needs to recognise and size each member.
struct MemberInfo {
    name: Vec<u8>,
    unpacked: u64,
    /// File data the crate cannot skip in a solid archive (not a directory or redirection).
    data: bool,
}

type SharedSlot = Arc<Mutex<Option<CappedSink>>>;

/// rars needs a `'static` writer, so the sink lives behind a shared slot the backend finishes.
struct SharedSink(SharedSlot);

impl Write for SharedSink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_mut()
        {
            Some(sink) => sink.write(buf),
            None => Err(io::Error::other("sink already closed")),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_mut()
        {
            Some(sink) => sink.flush(),
            None => Ok(()),
        }
    }
}

fn read_options(memory: u64) -> ArchiveReadOptions<'static> {
    ArchiveReadOptions::new()
        .with_rar50_dictionary_size_limit(memory)
        .with_max_reader_workspace_bytes(memory)
}

pub(crate) fn open(file: File, limits: &Limits) -> Result<Box<dyn ArchiveReader>, ArchiveError> {
    let archive = rars::ArchiveReader::read_reader_with_options(
        BufReader::new(file),
        read_options(limits.memory),
    )
    .map_err(|error| map_error(&error, limits.memory))?;
    let mut entries = Vec::new();
    let mut members = Vec::new();
    let mut split = archive_is_volume(&archive);
    let mut solid = archive_is_solid(&archive);
    for (index, member) in archive.members().enumerate() {
        check_entry_count(index as u64 + 1, limits)?;
        split |= member.meta.is_split_before || member.meta.is_split_after;
        solid |= member.is_solid();
        members.push(MemberInfo {
            name: member.meta.name.clone(),
            unpacked: member.meta.unpacked_size,
            data: !member.meta.is_directory && !member.meta.is_redirection,
        });
        entries.push(raw_entry(index, &member)?);
    }
    let listing = Listing {
        format: ArchiveFormat::Rar,
        entries,
        duplicate_raw: false,
        solid,
    };
    check_counts(&listing, limits)?;
    if split {
        return Err(ArchiveError::unsupported(UnsupportedReason::MultiVolume));
    }
    if archive.sfx_offset() != 0 {
        return Err(ArchiveError::unsupported(UnsupportedReason::SelfExtracting));
    }
    check_total(&listing, limits)?;
    Ok(Box::new(RarBackend {
        archive,
        listing,
        members,
        memory: limits.memory,
    }))
}

fn archive_is_volume(archive: &Archive) -> bool {
    archive.as_rar50().is_some_and(|rar| rar.main.is_volume())
        || archive
            .as_rar15_40()
            .is_some_and(|rar| rar.main.is_volume())
}

fn archive_is_solid(archive: &Archive) -> bool {
    archive.as_rar50().is_some_and(|rar| rar.main.is_solid())
        || archive.as_rar15_40().is_some_and(|rar| rar.main.is_solid())
        || archive.as_rar13().is_some_and(|rar| rar.main.is_solid())
}

fn raw_entry(index: usize, member: &ArchiveMember) -> Result<RawEntry, ArchiveError> {
    let meta = &member.meta;
    let kind = if meta.is_directory {
        EntryKind::Dir
    } else if meta.is_redirection {
        EntryKind::Link
    } else if meta.attr_source() == AttrSource::Unix {
        unix_kind(u32::try_from(meta.file_attr & 0xFFFF_FFFF).unwrap_or(0))
    } else {
        EntryKind::File
    };
    Ok(RawEntry {
        index: u32::try_from(index).map_err(|_| ArchiveError::TooManyEntries {
            count: index as u64,
        })?,
        name: member_name(member),
        kind,
        size: if kind == EntryKind::File {
            meta.unpacked_size
        } else {
            0
        },
        crc32: match &member.detail {
            ArchiveMemberDetail::Rar15To40 { crc32, .. } => Some(*crc32),
            ArchiveMemberDetail::Rar50Plus { crc32, .. } => *crc32,
            _ => None,
        },
        mtime: meta.modification_time(),
        encrypted: meta.is_encrypted,
        anti: false,
    })
}

fn member_name(member: &ArchiveMember) -> RawEntryName {
    if !member.name_is_unicode() {
        return RawEntryName::Bytes(member.meta.name.clone());
    }
    match member.decoded_name(None) {
        Ok(bytes) => RawEntryName::Unicode(String::from_utf8_lossy(&bytes).into_owned()),
        Err(_) => RawEntryName::Unicode(String::from_utf8_lossy(&member.meta.name).into_owned()),
    }
}

impl ArchiveReader for RarBackend {
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

/// The open sink of the member being decoded; finished when the next member is visited (the
/// crate verified the previous one by then) or after the call returns.
struct Pending {
    file: PlannedFile,
    slot: SharedSlot,
}

impl RarBackend {
    fn extract_planned(
        &mut self,
        plan: &ExtractPlan,
        target: &mut dyn ExtractTarget,
        progress: &ProgressCounter,
        cancel: &CancelToken,
    ) -> Result<ExtractReport, ArchiveError> {
        let mut report = ExtractReport::default();
        let Some(last_planned) = plan.files.last().map(|file| file.index) else {
            progress.flush();
            return Ok(report);
        };
        let planned: HashMap<u32, &PlannedFile> =
            plan.files.iter().map(|file| (file.index, file)).collect();
        let options = self.extract_options(&planned, last_planned, cancel);
        let failure = FailureSlot::default();
        let mut pending: Option<Pending> = None;
        let mut visited: u32 = 0;
        let result = self.archive.extract_with_control(options, |member| {
            let index = visited;
            visited += 1;
            let refuse = |error: ArchiveError| {
                failure.set(error);
                Err(RarError::Cancelled)
            };
            if let Some(done) = pending.take() {
                match finish(done, &mut report) {
                    Ok(()) => {}
                    Err(error) => return refuse(error),
                }
            }
            if index > last_planned {
                return Ok(ExtractionDecision::Stop);
            }
            if self.members.get(index as usize).map(|info| &info.name) != Some(&member.meta.name) {
                return refuse(ArchiveError::corrupt("member order changed"));
            }
            if cancel.is_cancelled() {
                return refuse(ArchiveError::Cancelled);
            }
            let Some(file) = planned.get(&index) else {
                return Ok(self.unplanned_decision(index));
            };
            let staged = match target.create(file) {
                Ok(staged) => staged,
                Err(error) => return refuse(ArchiveError::target(error)),
            };
            let slot = Arc::new(Mutex::new(Some(CappedSink::new(
                staged, file, cancel, progress, &failure,
            ))));
            pending = Some(Pending {
                file: (*file).clone(),
                slot: Arc::clone(&slot),
            });
            Ok(ExtractionDecision::Extract(Box::new(SharedSink(slot))))
        });
        if let Err(error) = result {
            return Err(failure
                .take()
                .unwrap_or_else(|| map_error(&error, self.memory)));
        }
        if let Some(done) = pending.take() {
            finish(done, &mut report)?;
        }
        if report.files as usize != plan.files.len() {
            return Err(ArchiveError::corrupt("a planned member was not visited"));
        }
        progress.flush();
        Ok(report)
    }

    /// Solid archives cannot skip file data: unplanned members before the last planned one are
    /// decoded into `io::sink()`, so they count against the output quotas too.
    fn extract_options<'a>(
        &self,
        planned: &HashMap<u32, &PlannedFile>,
        last_planned: u32,
        cancel: &'a CancelToken,
    ) -> ArchiveReadOptions<'a> {
        let decoded = self.members.iter().zip(0u32..).filter(|(info, index)| {
            *index <= last_planned
                && (planned.contains_key(index) || (self.listing.solid && info.data))
        });
        let (largest, total) = decoded.fold((0u64, 0u64), |(largest, total), (info, _)| {
            (
                largest.max(info.unpacked),
                total.saturating_add(info.unpacked),
            )
        });
        read_options(self.memory)
            .with_cancellation(cancel.rar())
            .with_max_member_output_bytes(largest)
            .with_max_total_output_bytes(total)
    }

    fn unplanned_decision(&self, index: u32) -> ExtractionDecision {
        let data = self
            .members
            .get(index as usize)
            .is_some_and(|info| info.data);
        if self.listing.solid && data {
            ExtractionDecision::Extract(Box::new(io::sink()))
        } else {
            ExtractionDecision::Skip
        }
    }
}

fn finish(done: Pending, report: &mut ExtractReport) -> Result<(), ArchiveError> {
    let sink = done
        .slot
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take()
        .ok_or_else(|| ArchiveError::corrupt("sink finished twice"))?;
    report.bytes += sink.finish(done.file.mtime)?;
    report.files += 1;
    Ok(())
}

fn map_error(error: &RarError, memory: u64) -> ArchiveError {
    match error.kind() {
        RarErrorKind::PasswordRequired | RarErrorKind::BadPassword => ArchiveError::Encrypted,
        RarErrorKind::ResourceLimit => ArchiveError::MemoryLimit {
            needed: u64::MAX,
            limit: memory,
        },
        RarErrorKind::Cancelled => ArchiveError::Cancelled,
        RarErrorKind::UnsupportedFormat | RarErrorKind::UnsupportedFeature => {
            ArchiveError::unsupported(UnsupportedReason::Method)
        }
        _ => ArchiveError::corrupt(error.to_string()),
    }
}
