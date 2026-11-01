use crate::keys::display_path;
use crate::read::TocFile;
use crate::{INVALID_INDEX, PakError};

const DIRECTORY_ENTRY_SIZE: usize = 16;
const FILE_ENTRY_SIZE: usize = 12;
const MIN_STRING_SIZE: usize = 4;
/// Defence against crafted indexes whose shared names expand into huge listings.
const MAX_PATH_BYTES: usize = 4096;
const MAX_LISTING_BYTES: usize = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub name: u32,
    pub first_child: u32,
    pub next_sibling: u32,
    pub first_file: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileEntry {
    pub name: u32,
    pub next_file: u32,
    pub user_data: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryIndex {
    pub mount_point: String,
    pub directories: Vec<DirectoryEntry>,
    pub files: Vec<FileEntry>,
    pub strings: Vec<String>,
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, len: usize, what: &'static str) -> Result<&'a [u8], PakError> {
        let end = self.pos.checked_add(len).ok_or(PakError::Malformed(what))?;
        let slice = self
            .bytes
            .get(self.pos..end)
            .ok_or(PakError::Malformed(what))?;
        self.pos = end;
        Ok(slice)
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.pos)
    }

    fn u32(&mut self, what: &'static str) -> Result<u32, PakError> {
        let bytes = self.take(4, what)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn i32(&mut self, what: &'static str) -> Result<i32, PakError> {
        Ok(i32::from_le_bytes(self.u32(what)?.to_le_bytes()))
    }

    /// A non-negative count whose entries fit in the remaining bytes, checked before allocating.
    fn count(&mut self, entry_size: usize) -> Result<usize, PakError> {
        let count =
            usize::try_from(self.i32("count")?).map_err(|_| PakError::Malformed("count"))?;
        match count.checked_mul(entry_size) {
            Some(bytes) if bytes <= self.remaining() => Ok(count),
            _ => Err(PakError::Malformed("count")),
        }
    }

    fn string(&mut self) -> Result<String, PakError> {
        let len = self.i32("string")?;
        if len == 0 {
            return Ok(String::new());
        }
        if len > 0 {
            let len = usize::try_from(len).map_err(|_| PakError::Malformed("string"))?;
            return latin1(self.take(len, "string")?);
        }
        if len == i32::MIN {
            return Err(PakError::Malformed("string"));
        }
        let units =
            usize::try_from(len.unsigned_abs()).map_err(|_| PakError::Malformed("string"))?;
        let byte_len = units.checked_mul(2).ok_or(PakError::Malformed("string"))?;
        utf16(self.take(byte_len, "string")?)
    }
}

fn latin1(bytes: &[u8]) -> Result<String, PakError> {
    match bytes.split_last() {
        Some((0, chars)) => Ok(chars.iter().map(|&b| char::from(b)).collect()),
        _ => Err(PakError::Malformed("string")),
    }
}

fn utf16(bytes: &[u8]) -> Result<String, PakError> {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect();
    match units.split_last() {
        Some((0, chars)) => char::decode_utf16(chars.iter().copied())
            .collect::<Result<String, _>>()
            .map_err(|_| PakError::Malformed("utf16")),
        _ => Err(PakError::Malformed("string")),
    }
}

/// §4.3; must consume `bytes` exactly.
pub fn parse_directory_index(bytes: &[u8]) -> Result<DirectoryIndex, PakError> {
    let mut reader = Reader { bytes, pos: 0 };
    let mount_point = reader.string()?;
    let dir_count = reader.count(DIRECTORY_ENTRY_SIZE)?;
    let mut directories = Vec::with_capacity(dir_count);
    for _ in 0..dir_count {
        directories.push(DirectoryEntry {
            name: reader.u32("count")?,
            first_child: reader.u32("count")?,
            next_sibling: reader.u32("count")?,
            first_file: reader.u32("count")?,
        });
    }
    let file_count = reader.count(FILE_ENTRY_SIZE)?;
    let mut files = Vec::with_capacity(file_count);
    for _ in 0..file_count {
        files.push(FileEntry {
            name: reader.u32("count")?,
            next_file: reader.u32("count")?,
            user_data: reader.u32("count")?,
        });
    }
    let string_count = reader.count(MIN_STRING_SIZE)?;
    let mut strings = Vec::with_capacity(string_count);
    for _ in 0..string_count {
        strings.push(reader.string()?);
    }
    if reader.remaining() != 0 {
        return Err(PakError::Malformed("trailing bytes"));
    }
    Ok(DirectoryIndex {
        mount_point,
        directories,
        files,
        strings,
    })
}

struct Walk<'a> {
    index: &'a DirectoryIndex,
    entry_count: u32,
    dir_seen: Vec<bool>,
    file_seen: Vec<bool>,
    listing_bytes: usize,
    out: Vec<TocFile>,
}

impl DirectoryIndex {
    /// §4.3 walk + §4.4 display paths, sorted; `entry_count` bounds `user_data`.
    pub fn files(&self, entry_count: u32) -> Result<Vec<TocFile>, PakError> {
        if self.mount_point.contains('\\') {
            return Err(PakError::Malformed("mount"));
        }
        if self.directories.is_empty() {
            return if self.files.is_empty() {
                Ok(Vec::new())
            } else {
                Err(PakError::Malformed("no root"))
            };
        }
        let mut mount = self.mount_point.clone();
        if !mount.ends_with('/') {
            mount.push('/');
        }
        let mut walk = Walk {
            index: self,
            entry_count,
            dir_seen: vec![false; self.directories.len()],
            file_seen: vec![false; self.files.len()],
            listing_bytes: 0,
            out: Vec::new(),
        };
        walk.run(mount)?;
        let mut out = walk.out;
        out.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(out)
    }

    fn name(&self, name: u32) -> Result<&str, PakError> {
        let text = usize::try_from(name)
            .ok()
            .and_then(|i| self.strings.get(i))
            .ok_or(PakError::Malformed("name"))?;
        let bad =
            text.is_empty() || text == "." || text == ".." || text.contains(['/', '\\', '\0']);
        if bad {
            return Err(PakError::Malformed("name"));
        }
        Ok(text)
    }
}

fn entry_slot(link: u32, len: usize) -> Result<Option<usize>, PakError> {
    if link == INVALID_INDEX {
        return Ok(None);
    }
    match usize::try_from(link) {
        Ok(i) if i < len => Ok(Some(i)),
        _ => Err(PakError::Malformed("link")),
    }
}

impl Walk<'_> {
    fn run(&mut self, mount: String) -> Result<(), PakError> {
        self.dir_seen[0] = true;
        let mut stack = vec![(0usize, mount)];
        while let Some((dir, prefix)) = stack.pop() {
            let entry = self.index.directories[dir];
            self.collect_files(entry.first_file, &prefix)?;
            let mut child = entry_slot(entry.first_child, self.dir_seen.len())?;
            while let Some(c) = child {
                if self.dir_seen[c] {
                    return Err(PakError::Malformed("cycle"));
                }
                self.dir_seen[c] = true;
                let child_entry = self.index.directories[c];
                let path = self.join(&prefix, child_entry.name, "/")?;
                stack.push((c, path));
                child = entry_slot(child_entry.next_sibling, self.dir_seen.len())?;
            }
        }
        Ok(())
    }

    fn collect_files(&mut self, first: u32, prefix: &str) -> Result<(), PakError> {
        let mut file = entry_slot(first, self.file_seen.len())?;
        while let Some(f) = file {
            if self.file_seen[f] {
                return Err(PakError::Malformed("cycle"));
            }
            self.file_seen[f] = true;
            let entry = self.index.files[f];
            if entry.user_data >= self.entry_count {
                return Err(PakError::Malformed("user data"));
            }
            let raw = self.join(prefix, entry.name, "")?;
            self.out.push(TocFile {
                path: display_path(&raw),
                chunk_index: entry.user_data,
            });
            file = entry_slot(entry.next_file, self.file_seen.len())?;
        }
        Ok(())
    }

    fn join(&mut self, prefix: &str, name: u32, suffix: &str) -> Result<String, PakError> {
        let name = self.index.name(name)?;
        let len = prefix.len() + name.len() + suffix.len();
        self.listing_bytes = self.listing_bytes.saturating_add(len);
        if len > MAX_PATH_BYTES || self.listing_bytes > MAX_LISTING_BYTES {
            return Err(PakError::Malformed("path length"));
        }
        let mut path = String::with_capacity(len);
        path.push_str(prefix);
        path.push_str(name);
        path.push_str(suffix);
        Ok(path)
    }
}
