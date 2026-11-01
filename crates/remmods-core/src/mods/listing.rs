//! Directory reads for the scan: every `read_dir` goes through [`Lister`], which counts them.

use std::ffi::OsString;
use std::fs::{self, DirEntry};
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::model::ScanProblem;
use crate::platform;
use crate::safe_fs::names::key_text;

/// What an entry is; links are resolved one hop (a broken link is `Other`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntryKind {
    Dir,
    File,
    LinkToDir,
    LinkToFile,
    /// Broken links, sockets, devices: never part of a mod.
    Other,
}

impl EntryKind {
    pub(crate) fn is_dir_like(self) -> bool {
        matches!(self, Self::Dir | Self::LinkToDir)
    }

    pub(crate) fn is_file_like(self) -> bool {
        matches!(self, Self::File | Self::LinkToFile)
    }

    pub(crate) fn is_link(self) -> bool {
        matches!(self, Self::LinkToDir | Self::LinkToFile)
    }
}

pub(crate) struct Entry {
    pub name: OsString,
    /// `names::key_text(name)`: lossless, used for keys, refs and ordering.
    pub key: String,
    /// The listed directory's path plus this entry's on-disk name (mod-side form).
    pub path: PathBuf,
    pub kind: EntryKind,
    dir_entry: DirEntry,
    /// The followed size and mtime of a link to a file.
    link_stat: Option<FileStat>,
    /// The entry's type could not be read.
    pub problem: Option<ScanProblem>,
}

impl Entry {
    /// Logical length of a file or of a link's target.
    pub(crate) fn size(&self) -> Result<u64, ScanProblem> {
        self.stat().map(|stat| stat.size)
    }

    /// Length and mtime from the one metadata read the scan already does.
    pub(crate) fn stat(&self) -> Result<FileStat, ScanProblem> {
        if let Some(stat) = self.link_stat {
            return Ok(stat);
        }
        self.dir_entry
            .metadata()
            .map(|metadata| FileStat::of(&metadata))
            .map_err(|error| problem("stat", &error))
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct FileStat {
    pub size: u64,
    /// `None` when the platform cannot report it.
    pub modified: Option<SystemTime>,
}

impl FileStat {
    fn of(metadata: &fs::Metadata) -> Self {
        Self {
            size: metadata.len(),
            modified: metadata.modified().ok(),
        }
    }
}

/// Counts every `read_dir` call (success or failure) and every entry seen.
#[derive(Debug, Default)]
pub(crate) struct Lister {
    pub dirs_read: u32,
    pub entries_seen: u32,
}

impl Lister {
    pub(crate) fn list(&mut self, dir: &Path) -> io::Result<Vec<Entry>> {
        self.dirs_read = self.dirs_read.saturating_add(1);
        let mut entries = Vec::new();
        for dir_entry in fs::read_dir(dir)? {
            let dir_entry = dir_entry?;
            self.entries_seen = self.entries_seen.saturating_add(1);
            entries.push(describe(dir_entry));
        }
        // read_dir order differs between file systems; everything downstream sees name order.
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }
}

pub(crate) fn problem(op: &'static str, error: &io::Error) -> ScanProblem {
    ScanProblem {
        op,
        class: platform::classify(error),
    }
}

fn describe(dir_entry: DirEntry) -> Entry {
    let name = dir_entry.file_name();
    let path = dir_entry.path();
    let mut entry = Entry {
        key: key_text(&name),
        name,
        path,
        kind: EntryKind::Other,
        dir_entry,
        link_stat: None,
        problem: None,
    };
    match entry.dir_entry.file_type() {
        Ok(file_type) if file_type.is_symlink() => resolve_link(&mut entry),
        Ok(file_type) if file_type.is_dir() => entry.kind = EntryKind::Dir,
        Ok(file_type) if file_type.is_file() => entry.kind = EntryKind::File,
        Ok(_) => {}
        Err(error) => entry.problem = Some(problem("stat", &error)),
    }
    entry
}

/// Symlinks and junctions alike: the target decides, one hop; a broken link is ignored.
fn resolve_link(entry: &mut Entry) {
    match fs::metadata(&entry.path) {
        Ok(target) if target.is_dir() => entry.kind = EntryKind::LinkToDir,
        Ok(target) if target.is_file() => {
            entry.kind = EntryKind::LinkToFile;
            entry.link_stat = Some(FileStat::of(&target));
        }
        Ok(_) => {}
        Err(error) => log::debug!("broken link {}: {error}", entry.path.display()),
    }
}
