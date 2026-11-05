//! Test-only `.utoc` writer (SPEC-P2B §4.7). Pure bytes, never touches the file system.

use crate::{ContentHash, INVALID_INDEX, TOC_MAGIC, chunk_meta_size};

const METHOD_NAME_LENGTH: usize = 32;
const BLOCK_SIZE: usize = 0x1_0000;

#[derive(Clone, Debug)]
pub struct TocBuilder {
    mount_point: String,
    files: Vec<String>,
    /// Content hash per entry of `files` (chunk index = position); `None` writes zeros.
    hashes: Vec<Option<ContentHash>>,
    /// Bytes per entry of `files`; a file without any keeps its zero table entries.
    payloads: Vec<Vec<u8>>,
    /// Block bytes as `.ucas` stores them, and the block's method byte.
    wrap: fn(&[u8]) -> (Vec<u8>, u8),
    version: u8,
    flags: u8,
    signed_hash_size: Option<u32>,
    utf16: bool,
    extra_chunks: u32,
}

#[derive(Default)]
struct Dir {
    name: Option<u32>,
    children: Vec<usize>,
    files: Vec<(u32, u32)>,
}

#[derive(Default)]
struct Strings {
    table: Vec<String>,
}

impl Strings {
    fn intern(&mut self, text: &str) -> u32 {
        let at = match self.table.iter().position(|s| s == text) {
            Some(at) => at,
            None => {
                self.table.push(text.to_owned());
                self.table.len() - 1
            }
        };
        u32_of(at)
    }
}

fn u32_of(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_i32(out: &mut Vec<u8>, value: i32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_fstring(out: &mut Vec<u8>, text: &str, utf16: bool) {
    if text.is_empty() {
        push_i32(out, 0);
        return;
    }
    if !utf16 && text.chars().all(|c| u32::from(c) <= 0xFF) {
        push_i32(
            out,
            i32::try_from(text.chars().count() + 1).unwrap_or(i32::MAX),
        );
        out.extend(
            text.chars()
                .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?')),
        );
        out.push(0);
        return;
    }
    let units: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    push_i32(out, -i32::try_from(units.len()).unwrap_or(i32::MAX));
    for unit in units {
        out.extend_from_slice(&unit.to_le_bytes());
    }
}

impl TocBuilder {
    /// v5, flags 0x09, methods ["Oodle"], perfect-hash seeds = ceil(n/2), overflow 0,
    /// one compressed block per file, zero chunk metas (33 bytes, 24 from v8) unless a file
    /// has a hash, Latin-1 strings.
    pub fn new(mount_point: &str) -> Self {
        Self {
            mount_point: mount_point.to_owned(),
            files: Vec::new(),
            hashes: Vec::new(),
            payloads: Vec::new(),
            wrap: |block| (block.to_vec(), 0),
            version: 5,
            flags: 0x09,
            signed_hash_size: None,
            utf16: false,
            extra_chunks: 0,
        }
    }

    /// `/`-separated, directories created as needed.
    pub fn file(mut self, path_below_mount: &str) -> Self {
        self.files.push(path_below_mount.to_owned());
        self.hashes.push(None);
        self.payloads.push(Vec::new());
        self
    }

    /// Like `file`, with this content hash in its chunk meta.
    pub fn file_with_hash(mut self, path_below_mount: &str, hash: ContentHash) -> Self {
        self.files.push(path_below_mount.to_owned());
        self.hashes.push(Some(hash));
        self.payloads.push(Vec::new());
        self
    }

    /// Like `file_with_hash`, with these bytes as the chunk `build_with_cas` writes.
    pub fn file_with_bytes(
        mut self,
        path_below_mount: &str,
        hash: ContentHash,
        bytes: &[u8],
    ) -> Self {
        self.files.push(path_below_mount.to_owned());
        self.hashes.push(Some(hash));
        self.payloads.push(bytes.to_vec());
        self
    }

    /// How each 64 KiB block lands in the `.ucas`: its stored bytes and its method byte (0 =
    /// stored, 1 = Oodle).
    pub fn block_wrap(mut self, wrap: fn(&[u8]) -> (Vec<u8>, u8)) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn version(mut self, version: u8) -> Self {
        self.version = version;
        self
    }

    pub fn flags(mut self, flags: u8) -> Self {
        self.flags = flags;
        self
    }

    /// Sets 0x04 and writes zero signatures.
    pub fn signed(mut self, hash_size: u32) -> Self {
        self.signed_hash_size = Some(hash_size);
        self
    }

    pub fn utf16_strings(mut self, on: bool) -> Self {
        self.utf16 = on;
        self
    }

    /// Chunks without a file name.
    pub fn extra_chunks(mut self, count: u32) -> Self {
        self.extra_chunks = count;
        self
    }

    fn entry_count(&self) -> u32 {
        u32_of(self.files.len())
            .saturating_add(1)
            .saturating_add(self.extra_chunks)
    }

    fn tree(&self, strings: &mut Strings) -> Vec<Dir> {
        let mut dirs = vec![Dir::default()];
        for (chunk, path) in self.files.iter().enumerate() {
            let mut parts: Vec<&str> = path.split('/').collect();
            let file_name = parts.pop().unwrap_or_default();
            let mut current = 0;
            for part in parts {
                let name = strings.intern(part);
                let found = dirs[current]
                    .children
                    .iter()
                    .copied()
                    .find(|&c| dirs[c].name == Some(name));
                current = match found {
                    Some(c) => c,
                    None => {
                        dirs.push(Dir {
                            name: Some(name),
                            ..Dir::default()
                        });
                        let added = dirs.len() - 1;
                        dirs[current].children.push(added);
                        added
                    }
                };
            }
            let name = strings.intern(file_name);
            dirs[current].files.push((name, u32_of(chunk)));
        }
        dirs
    }

    /// Directory index bytes only.
    pub fn build_index(&self) -> Vec<u8> {
        let mut strings = Strings::default();
        let dirs = self.tree(&mut strings);
        let mut out = Vec::new();
        push_fstring(&mut out, &self.mount_point, self.utf16);
        push_i32(&mut out, i32::try_from(dirs.len()).unwrap_or(i32::MAX));
        let mut next_sibling = vec![INVALID_INDEX; dirs.len()];
        for dir in &dirs {
            for pair in dir.children.windows(2) {
                next_sibling[pair[0]] = u32_of(pair[1]);
            }
        }
        let mut first_file = Vec::with_capacity(dirs.len());
        let mut file_entries = Vec::new();
        for dir in &dirs {
            first_file.push(if dir.files.is_empty() {
                INVALID_INDEX
            } else {
                u32_of(file_entries.len())
            });
            for (i, &(name, chunk)) in dir.files.iter().enumerate() {
                let next = if i + 1 < dir.files.len() {
                    u32_of(file_entries.len() + 1)
                } else {
                    INVALID_INDEX
                };
                file_entries.push((name, next, chunk));
            }
        }
        for (i, dir) in dirs.iter().enumerate() {
            push_u32(&mut out, dir.name.unwrap_or(INVALID_INDEX));
            push_u32(
                &mut out,
                dir.children.first().map_or(INVALID_INDEX, |&c| u32_of(c)),
            );
            push_u32(&mut out, next_sibling[i]);
            push_u32(&mut out, first_file[i]);
        }
        push_i32(
            &mut out,
            i32::try_from(file_entries.len()).unwrap_or(i32::MAX),
        );
        for (name, next, chunk) in file_entries {
            push_u32(&mut out, name);
            push_u32(&mut out, next);
            push_u32(&mut out, chunk);
        }
        push_i32(
            &mut out,
            i32::try_from(strings.table.len()).unwrap_or(i32::MAX),
        );
        for text in &strings.table {
            push_fstring(&mut out, text, self.utf16);
        }
        out
    }

    /// The chunk table and block table (bytes as in the `.utoc`) and the `.ucas`.
    fn stream(&self) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let mut chunks = Vec::new();
        let mut blocks = Vec::new();
        let mut cas = Vec::new();
        let mut position = 0usize;
        for payload in &self.payloads {
            let entry = [position as u64, payload.len() as u64];
            for value in entry {
                chunks.extend_from_slice(&value.to_be_bytes()[3..]);
            }
            position += payload.len().div_ceil(BLOCK_SIZE) * BLOCK_SIZE;
            for block in payload.chunks(BLOCK_SIZE) {
                let (stored, method) = (self.wrap)(block);
                blocks.extend_from_slice(&(cas.len() as u64).to_le_bytes()[..5]);
                blocks.extend_from_slice(&u32_of(stored.len()).to_le_bytes()[..3]);
                blocks.extend_from_slice(&u32_of(block.len()).to_le_bytes()[..3]);
                blocks.push(method);
                cas.extend_from_slice(&stored);
            }
            if payload.is_empty() {
                blocks.resize(blocks.len() + 12, 0);
            }
        }
        (chunks, blocks, cas)
    }

    /// `.utoc` and `.ucas`, with real chunk and block tables.
    pub fn build_with_cas(&self) -> (Vec<u8>, Vec<u8>) {
        (self.build(), self.stream().2)
    }

    /// Whole `.utoc`.
    pub fn build(&self) -> Vec<u8> {
        let index = self.build_index();
        let (chunk_table, block_table, _) = self.stream();
        let entries = self.entry_count();
        let files = u32_of(self.files.len());
        let seeds = files.div_ceil(2);
        let flags = if self.signed_hash_size.is_some() {
            self.flags | 0x04
        } else {
            self.flags
        };
        let mut out = Vec::new();
        out.extend_from_slice(&TOC_MAGIC);
        out.extend_from_slice(&[self.version, 0, 0, 0]);
        for value in [
            144,
            entries,
            u32_of(block_table.len() / 12),
            12,
            1,
            u32_of(METHOD_NAME_LENGTH),
            0x1_0000,
            u32_of(index.len()),
            1,
        ] {
            push_u32(&mut out, value);
        }
        out.extend_from_slice(&0x0123_4567_89AB_CDEF_u64.to_le_bytes());
        out.extend_from_slice(&[0u8; 16]);
        out.extend_from_slice(&[flags, 0, 0, 0]);
        push_u32(&mut out, seeds);
        out.extend_from_slice(&u64::MAX.to_le_bytes());
        push_u32(&mut out, 0);
        out.resize(144, 0);
        let entries = usize::try_from(entries).unwrap_or(0);
        let zeros = |out: &mut Vec<u8>, n: usize| out.resize(out.len() + n, 0);
        zeros(&mut out, entries * 12);
        out.extend_from_slice(&chunk_table);
        zeros(&mut out, entries * 10 - chunk_table.len());
        if self.version >= 4 {
            zeros(&mut out, usize::try_from(seeds).unwrap_or(0) * 4);
        }
        out.extend_from_slice(&block_table);
        let mut method = b"Oodle".to_vec();
        method.resize(METHOD_NAME_LENGTH, 0);
        out.extend_from_slice(&method);
        if let Some(hash_size) = self.signed_hash_size {
            push_u32(&mut out, hash_size);
            let hash = usize::try_from(hash_size).unwrap_or(0);
            zeros(&mut out, 2 * hash + self.files.len() * 20);
        }
        out.extend_from_slice(&index);
        let meta_size = usize::try_from(chunk_meta_size(self.version)).unwrap_or(0);
        for chunk in 0..entries {
            let start = out.len();
            zeros(&mut out, meta_size);
            if let Some(Some(hash)) = self.hashes.get(chunk) {
                out[start..start + hash.len()].copy_from_slice(hash);
            }
        }
        out
    }
}
