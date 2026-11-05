// Fixtures are crafted archives written into TempDirs, so the std writers are fine here.
#![allow(clippy::disallowed_methods, clippy::expect_used, dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::SystemTime;

use remmods_core::archive::{
    ArchiveError, ArchiveReader, CancelToken, ExtractPlan, ExtractReport, ExtractTarget, Limits,
    PlannedFile, ProgressCounter, StagedWrite, normalize,
};
use remmods_core::archive::{EntryKind, NameEncoding};
use tempfile::TempDir;

const LOCAL_SIG: u32 = 0x0403_4B50;
const CENTRAL_SIG: u32 = 0x0201_4B50;
const END_SIG: u32 = 0x0605_4B50;
/// 1980-01-01, the first DOS date.
const DOS_DATE: u16 = (1 << 5) | 1;

/// A temp dir and its canonical path (Windows runners hand out 8.3 aliases).
pub fn temp_dir() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let root = dunce::canonicalize(dir.path()).expect("canonicalize temp dir");
    (dir, root)
}

/// One entry of a hand-written zip. Data is always stored as given (method 0 unless `method`
/// says otherwise; the bytes are not compressed either way).
#[derive(Clone, Debug)]
pub struct CraftEntry {
    pub name: Vec<u8>,
    pub data: Vec<u8>,
    pub utf8_flag: bool,
    pub unix_mode: Option<u32>,
    pub external_attr: Option<u32>,
    pub encrypted_flag: bool,
    /// Overrides the central directory's uncompressed size.
    pub central_size: Option<u32>,
    /// Overrides the CRC in both headers.
    pub crc: Option<u32>,
    /// Central directory "disk number start"; the end record names the highest one.
    pub disk: u16,
    pub method: u16,
}

impl CraftEntry {
    pub fn file(name: &str, data: &[u8]) -> Self {
        Self::raw(name.as_bytes(), data)
    }

    pub fn raw(name: &[u8], data: &[u8]) -> Self {
        Self {
            name: name.to_vec(),
            data: data.to_vec(),
            utf8_flag: false,
            unix_mode: None,
            external_attr: None,
            encrypted_flag: false,
            central_size: None,
            crc: None,
            disk: 0,
            method: 0,
        }
    }
}

/// A stored-only zip writer with full control over every header field.
#[derive(Clone, Debug, Default)]
pub struct ZipCraft {
    pub entries: Vec<CraftEntry>,
}

impl ZipCraft {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, entry: CraftEntry) -> Self {
        self.entries.push(entry);
        self
    }

    pub fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut offsets = Vec::new();
        for entry in &self.entries {
            offsets.push(u32::try_from(out.len()).expect("zip fits 4 GiB"));
            put32(&mut out, LOCAL_SIG);
            put16(&mut out, 20);
            put16(&mut out, flags(entry));
            put16(&mut out, entry.method);
            put16(&mut out, 0);
            put16(&mut out, DOS_DATE);
            put32(&mut out, crc_of(entry));
            put32(&mut out, len32(&entry.data));
            put32(&mut out, len32(&entry.data));
            put16(&mut out, len16(&entry.name));
            put16(&mut out, 0);
            out.extend_from_slice(&entry.name);
            out.extend_from_slice(&entry.data);
        }
        let central_start = out.len();
        for (entry, offset) in self.entries.iter().zip(&offsets) {
            put32(&mut out, CENTRAL_SIG);
            put16(
                &mut out,
                if entry.unix_mode.is_some() {
                    (3 << 8) | 20
                } else {
                    20
                },
            );
            put16(&mut out, 20);
            put16(&mut out, flags(entry));
            put16(&mut out, entry.method);
            put16(&mut out, 0);
            put16(&mut out, DOS_DATE);
            put32(&mut out, crc_of(entry));
            put32(&mut out, len32(&entry.data));
            put32(
                &mut out,
                entry.central_size.unwrap_or_else(|| len32(&entry.data)),
            );
            put16(&mut out, len16(&entry.name));
            put16(&mut out, 0);
            put16(&mut out, 0);
            put16(&mut out, entry.disk);
            put16(&mut out, 0);
            let external = entry
                .external_attr
                .unwrap_or_else(|| entry.unix_mode.map_or(0, |mode| mode << 16));
            put32(&mut out, external);
            put32(&mut out, *offset);
            out.extend_from_slice(&entry.name);
        }
        let central_len = out.len() - central_start;
        let count = u16::try_from(self.entries.len()).expect("ZipCraft writes ≤ 65535 entries");
        put32(&mut out, END_SIG);
        put16(
            &mut out,
            self.entries
                .iter()
                .map(|entry| entry.disk)
                .max()
                .unwrap_or(0),
        );
        put16(&mut out, 0);
        put16(&mut out, count);
        put16(&mut out, count);
        put32(
            &mut out,
            u32::try_from(central_len).expect("central directory fits"),
        );
        put32(
            &mut out,
            u32::try_from(central_start).expect("central directory fits"),
        );
        put16(&mut out, 0);
        out
    }

    pub fn write(&self, dir: &Path, name: &str) -> PathBuf {
        write_file(dir, name, &self.bytes())
    }
}

fn flags(entry: &CraftEntry) -> u16 {
    u16::from(entry.encrypted_flag) | if entry.utf8_flag { 1 << 11 } else { 0 }
}

fn crc_of(entry: &CraftEntry) -> u32 {
    entry.crc.unwrap_or_else(|| crc32fast::hash(&entry.data))
}

fn len32(bytes: &[u8]) -> u32 {
    u32::try_from(bytes.len()).expect("entry fits 4 GiB")
}

fn len16(bytes: &[u8]) -> u16 {
    u16::try_from(bytes.len()).expect("name fits 64 KiB")
}

fn put16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub fn write_file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, bytes).expect("write fixture");
    path
}

pub type CreateHook = Box<dyn FnMut(&PlannedFile) + Send>;

/// An in-memory `ExtractTarget`. Writers stream into shared buffers; call [`Self::settle`]
/// after `extract` to copy them into the public fields.
#[derive(Default)]
pub struct RecordingTarget {
    pub created: Vec<Vec<String>>,
    pub bytes: BTreeMap<Vec<String>, Vec<u8>>,
    pub finished: Vec<(Vec<String>, Option<SystemTime>)>,
    /// Declared size of every created file.
    pub declared: BTreeMap<Vec<String>, u64>,
    /// Runs inside every `create`, before the writer exists.
    pub on_create: Option<CreateHook>,
    live: Arc<Mutex<Live>>,
}

#[derive(Default)]
struct Live {
    bytes: BTreeMap<Vec<String>, Vec<u8>>,
    finished: Vec<(Vec<String>, Option<SystemTime>)>,
}

impl RecordingTarget {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn settle(&mut self) {
        let live = self.live.lock().unwrap_or_else(PoisonError::into_inner);
        self.bytes = live.bytes.clone();
        self.finished = live.finished.clone();
    }

    /// Every created file received at most its declared size.
    pub fn within_declared(&self) -> bool {
        self.bytes.iter().all(|(dest, data)| {
            self.declared
                .get(dest)
                .is_some_and(|&size| data.len() as u64 <= size)
        })
    }
}

impl ExtractTarget for RecordingTarget {
    fn create(&mut self, file: &PlannedFile) -> io::Result<Box<dyn StagedWrite>> {
        if let Some(hook) = self.on_create.as_mut() {
            hook(file);
        }
        self.created.push(file.dest.clone());
        self.declared.insert(file.dest.clone(), file.size);
        self.live
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .bytes
            .insert(file.dest.clone(), Vec::new());
        Ok(Box::new(RecordingWrite {
            dest: file.dest.clone(),
            live: Arc::clone(&self.live),
        }))
    }
}

struct RecordingWrite {
    dest: Vec<String>,
    live: Arc<Mutex<Live>>,
}

impl Write for RecordingWrite {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut live = self.live.lock().unwrap_or_else(PoisonError::into_inner);
        live.bytes
            .entry(self.dest.clone())
            .or_default()
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl StagedWrite for RecordingWrite {
    fn finish(self: Box<Self>, mtime: Option<SystemTime>) -> io::Result<()> {
        let mut live = self.live.lock().unwrap_or_else(PoisonError::into_inner);
        live.finished.push((self.dest.clone(), mtime));
        Ok(())
    }
}

/// A plan with every plain file of the listing, each into slot 0 under its normalized path.
pub fn full_plan(reader: &dyn ArchiveReader) -> ExtractPlan {
    let entries = normalize(reader.listing(), NameEncoding::Auto, &Limits::default());
    let files = entries
        .into_iter()
        .zip(&reader.listing().entries)
        .filter(|(entry, raw)| {
            entry.kind == EntryKind::File && entry.problem.is_none() && !raw.anti && !raw.encrypted
        })
        .map(|(entry, raw)| PlannedFile {
            index: entry.index,
            slot: 0,
            dest: entry.path,
            size: raw.size,
            mtime: raw.mtime,
        })
        .collect();
    ExtractPlan { files }
}

/// A plan with only the listed indices of `full_plan`.
pub fn plan_of(reader: &dyn ArchiveReader, indices: &[u32]) -> ExtractPlan {
    let mut plan = full_plan(reader);
    plan.files.retain(|file| indices.contains(&file.index));
    plan
}

/// `extract` with a silent counter and a fresh token; the target is settled afterwards.
pub fn extract_all(
    reader: &mut dyn ArchiveReader,
    plan: &ExtractPlan,
    target: &mut RecordingTarget,
) -> Result<ExtractReport, ArchiveError> {
    let result = reader.extract(
        plan,
        target,
        &ProgressCounter::silent(),
        &CancelToken::new(),
    );
    target.settle();
    result
}

/// A 7z archive (LZMA2, one solid block) of plain files.
pub fn sevenz_fixture(dir: &Path, name: &str, files: &[(&str, &[u8])]) -> PathBuf {
    let entries = files
        .iter()
        .map(|(entry_name, data)| {
            (
                sevenz_rust2::ArchiveEntry::new_file(entry_name),
                Some(data.to_vec()),
            )
        })
        .collect();
    sevenz_build(dir, name, entries, None, true)
}

/// A 7z archive from prepared entries (`None` data = no stream). `methods` replaces the default
/// LZMA2; `solid` packs every stream into one block.
pub fn sevenz_build(
    dir: &Path,
    name: &str,
    entries: Vec<(sevenz_rust2::ArchiveEntry, Option<Vec<u8>>)>,
    methods: Option<Vec<sevenz_rust2::EncoderConfiguration>>,
    solid: bool,
) -> PathBuf {
    let mut writer = sevenz_rust2::ArchiveWriter::new(Cursor::new(Vec::new())).expect("7z writer");
    if let Some(methods) = methods {
        writer.set_content_methods(methods);
    }
    let (with_data, without): (Vec<_>, Vec<_>) = entries
        .into_iter()
        .partition(|(_, data)| data.as_ref().is_some_and(|d| !d.is_empty()));
    if solid && !with_data.is_empty() {
        let (heads, readers): (Vec<_>, Vec<_>) = with_data
            .into_iter()
            .map(|(entry, data)| {
                (
                    entry,
                    sevenz_rust2::SourceReader::new(Cursor::new(data.unwrap_or_default())),
                )
            })
            .unzip();
        writer
            .push_archive_entries(heads, readers)
            .expect("push solid entries");
    } else {
        for (entry, data) in with_data {
            writer
                .push_archive_entry(entry, data.map(Cursor::new))
                .expect("push entry");
        }
    }
    for (entry, _) in without {
        writer
            .push_archive_entry::<Cursor<Vec<u8>>>(entry, None)
            .expect("push empty entry");
    }
    let bytes = writer.finish().expect("finish 7z").into_inner();
    write_file(dir, name, &bytes)
}

/// A RAR5 archive of plain files, optionally encrypted with `password`.
#[cfg(feature = "rar-builtin")]
pub fn rar_fixture(
    dir: &Path,
    name: &str,
    files: &[(&str, &[u8])],
    password: Option<&[u8]>,
) -> PathBuf {
    let mut builder =
        rars::Builder::new(rars::ArchiveVersion::Rar50).password(password.map(<[u8]>::to_vec));
    for (entry_name, data) in files {
        builder
            .add_bytes(
                entry_name.as_bytes().to_vec(),
                data.to_vec(),
                Some(1_700_000_000),
                None,
            )
            .expect("add rar member");
    }
    rar_build(dir, name, &builder)
}

#[cfg(feature = "rar-builtin")]
pub fn rar_build(dir: &Path, name: &str, builder: &rars::Builder) -> PathBuf {
    write_file(dir, name, &builder.to_bytes().expect("build rar"))
}
