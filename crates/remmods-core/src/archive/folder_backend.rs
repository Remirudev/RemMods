//! A plain folder as an install source. The walk never follows links; extraction re-checks every
//! file right before and after opening it, which is the folder's change detection.

use std::ffi::OsStr;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use super::error::ArchiveError;
use super::model::{
    ArchiveFormat, ArchiveReader, EntryKind, ExtractPlan, ExtractReport, ExtractTarget, Limits,
    Listing, PlannedFile, RawEntry, RawEntryName,
};
use super::open::{check_counts, check_entry_count, check_total, guarded};
use super::progress::{CancelToken, ProgressCounter};
use super::sink::{CappedSink, FailureSlot, READ_CHUNK, copy_entry};
use super::validate::check_plan;
use crate::platform::{self, RawName};
use crate::safe_fs::names::order_key;

pub(crate) struct FolderBackend {
    listing: Listing,
    /// Full path of every entry, in `fs_path` form (children of an `fs_path` root).
    paths: Vec<PathBuf>,
}

#[derive(Clone)]
enum Part {
    Text(String),
    Bytes(Vec<u8>),
}

struct Found {
    parts: Vec<Part>,
    path: PathBuf,
    kind: EntryKind,
    size: u64,
    mtime: Option<std::time::SystemTime>,
}

pub(crate) fn open(root: &Path, limits: &Limits) -> Result<Box<dyn ArchiveReader>, ArchiveError> {
    let mut found = walk(root, limits)?;
    found.sort_by_cached_key(|item| order_key(&display(&item.parts)));
    let mut entries = Vec::with_capacity(found.len());
    let mut paths = Vec::with_capacity(found.len());
    for (index, item) in found.into_iter().enumerate() {
        entries.push(RawEntry {
            index: u32::try_from(index).map_err(|_| ArchiveError::TooManyEntries {
                count: index as u64,
            })?,
            name: entry_name(&item.parts),
            kind: item.kind,
            size: item.size,
            crc32: None,
            mtime: item.mtime,
            encrypted: false,
            anti: false,
        });
        paths.push(item.path);
    }
    let listing = Listing {
        format: ArchiveFormat::Folder,
        entries,
        duplicate_raw: false,
        solid: false,
    };
    check_counts(&listing, limits)?;
    check_total(&listing, limits)?;
    Ok(Box::new(FolderBackend { listing, paths }))
}

fn walk(root: &Path, limits: &Limits) -> Result<Vec<Found>, ArchiveError> {
    let mut found: Vec<Found> = Vec::new();
    let mut pending = vec![(root.to_path_buf(), Vec::new())];
    while let Some((dir, parts)) = pending.pop() {
        let items = fs::read_dir(&dir).map_err(|error| ArchiveError::io("read", error))?;
        for item in items {
            let item = item.map_err(|error| ArchiveError::io("read", error))?;
            check_entry_count(found.len() as u64 + 1, limits)?;
            // Neither call follows a final link (DirEntry metadata is lstat-like on Unix).
            let file_type = item
                .file_type()
                .map_err(|error| ArchiveError::io("read", error))?;
            let meta = item
                .metadata()
                .map_err(|error| ArchiveError::io("read", error))?;
            let kind = if file_type.is_symlink() {
                EntryKind::Link
            } else if file_type.is_dir() {
                EntryKind::Dir
            } else if file_type.is_file() {
                EntryKind::File
            } else {
                EntryKind::Special
            };
            let mut child = parts.clone();
            child.push(part(&item.file_name()));
            if kind == EntryKind::Dir {
                pending.push((item.path(), child.clone()));
            }
            found.push(Found {
                parts: child,
                path: item.path(),
                kind,
                size: if kind == EntryKind::File {
                    meta.len()
                } else {
                    0
                },
                mtime: meta.modified().ok(),
            });
        }
    }
    Ok(found)
}

fn part(name: &OsStr) -> Part {
    match name.to_str() {
        Some(text) => Part::Text(text.to_owned()),
        None => match platform::raw_name(name) {
            RawName::Bytes(bytes) => Part::Bytes(bytes),
            // Unpaired surrogates: U+FFFD marks the name undecodable later.
            RawName::Wide(_) => Part::Text(name.to_string_lossy().into_owned()),
        },
    }
}

fn entry_name(parts: &[Part]) -> RawEntryName {
    if parts.iter().all(|part| matches!(part, Part::Text(_))) {
        return RawEntryName::Unicode(display(parts));
    }
    let mut bytes = Vec::new();
    for (position, part) in parts.iter().enumerate() {
        if position > 0 {
            bytes.push(b'/');
        }
        match part {
            Part::Text(text) => bytes.extend_from_slice(text.as_bytes()),
            Part::Bytes(raw) => bytes.extend_from_slice(raw),
        }
    }
    RawEntryName::Bytes(bytes)
}

fn display(parts: &[Part]) -> String {
    parts
        .iter()
        .map(|part| match part {
            Part::Text(text) => text.clone(),
            Part::Bytes(raw) => String::from_utf8_lossy(raw).into_owned(),
        })
        .collect::<Vec<_>>()
        .join("/")
}

impl ArchiveReader for FolderBackend {
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

impl FolderBackend {
    fn extract_planned(
        &self,
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
            let mut source = self.open_unchanged(file)?;
            let staged = target.create(file).map_err(ArchiveError::target)?;
            let mut sink = CappedSink::new(staged, file, cancel, progress, &failure);
            copy_entry(&mut source, &mut sink, &mut buf, &failure)?;
            report.bytes += sink.finish(file.mtime)?;
            report.files += 1;
        }
        progress.flush();
        Ok(report)
    }

    /// Still a regular file (not a link, FIFO or device) before opening, and the opened handle
    /// is a regular file of the listed length; anything else is `SizeMismatch`. A FIFO is never
    /// opened, so a swapped-in FIFO cannot block the read.
    fn open_unchanged(&self, file: &PlannedFile) -> Result<File, ArchiveError> {
        let mismatch = ArchiveError::SizeMismatch { index: file.index };
        let path = &self.paths[file.index as usize];
        let meta = fs::symlink_metadata(path).map_err(|error| ArchiveError::io("read", error))?;
        if !meta.file_type().is_file() {
            return Err(mismatch);
        }
        let source = File::open(path).map_err(|error| ArchiveError::io("read", error))?;
        let opened = source
            .metadata()
            .map_err(|error| ArchiveError::io("read", error))?;
        if !opened.is_file() || opened.len() != file.size {
            return Err(mismatch);
        }
        Ok(source)
    }
}
