//! The chunk tables of a `.utoc`: where each chunk sits and how its blocks are stored.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use remmods_pak::{TOC_HEADER_SIZE, TocHeader, parse_header};

use crate::UnpackError;

const CHUNK_ID_SIZE: u64 = 12;
const OFFSET_AND_LENGTH_BYTES: usize = 10;
const OFFSET_AND_LENGTH_SIZE: u64 = OFFSET_AND_LENGTH_BYTES as u64;
const HASH_SEED_SIZE: u64 = 4;
const BLOCK_ENTRY_BYTES: usize = 12;
const BLOCK_ENTRY_SIZE: u64 = BLOCK_ENTRY_BYTES as u64;
/// Far above any container seen; a crafted count must not allocate gigabytes.
const MAX_TABLE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Block {
    pub offset: u64,
    pub compressed: u32,
    pub uncompressed: u32,
    /// 0 = stored, else 1 + index into the method names.
    pub method: u8,
}

/// Parsed from the header onward; the file itself is not kept open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChunkTable {
    pub(crate) block_size: u64,
    /// (offset, length) in the uncompressed stream, by chunk index.
    pub(crate) chunks: Vec<(u64, u64)>,
    pub(crate) blocks: Vec<Block>,
    pub(crate) methods: Vec<String>,
}

fn be40(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .fold(0, |value, &byte| (value << 8) | u64::from(byte))
}

fn le(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .rev()
        .fold(0, |value, &byte| (value << 8) | u64::from(byte))
}

fn seed_entries(header: &TocHeader) -> u64 {
    let seeds = if header.version >= 4 {
        u64::from(header.perfect_hash_seeds_count)
    } else {
        0
    };
    let overflow = if header.version >= 5 {
        u64::from(header.chunks_without_perfect_hash_count)
    } else {
        0
    };
    seeds.saturating_add(overflow)
}

impl ChunkTable {
    pub(crate) fn read(utoc: &Path) -> Result<Self, UnpackError> {
        let mut file = File::open(utoc)?;
        let mut head = [0u8; TOC_HEADER_SIZE];
        file.read_exact(&mut head)?;
        let header = parse_header(&head)?;
        if header.partition_count > 1 {
            return Err(UnpackError::Partitioned);
        }
        if header.compression_block_size == 0 {
            return Err(UnpackError::Malformed("block size"));
        }
        let layout = Layout::of(&header).ok_or(UnpackError::Malformed("table size"))?;
        let start = layout.start();
        file.seek(SeekFrom::Start(start))?;
        let mut bytes = vec![0u8; layout.table_len()?];
        file.read_exact(&mut bytes)?;
        layout.parse(&header, &bytes)
    }
}

/// Byte sizes of the tables that follow the chunk ids, in file order.
struct Layout {
    offsets: u64,
    seeds: u64,
    blocks: u64,
    methods: u64,
}

impl Layout {
    fn of(header: &TocHeader) -> Option<Self> {
        Some(Self {
            offsets: u64::from(header.entry_count).checked_mul(OFFSET_AND_LENGTH_SIZE)?,
            seeds: seed_entries(header).checked_mul(HASH_SEED_SIZE)?,
            blocks: u64::from(header.compressed_block_entry_count).checked_mul(BLOCK_ENTRY_SIZE)?,
            methods: u64::from(header.compression_method_name_count)
                .checked_mul(u64::from(header.compression_method_name_length))?,
        })
    }

    fn start(&self) -> u64 {
        // `entry_count` fits 32 bits, so the sum cannot overflow.
        let header = u64::try_from(TOC_HEADER_SIZE).unwrap_or(u64::MAX);
        header.saturating_add(self.offsets / OFFSET_AND_LENGTH_SIZE * CHUNK_ID_SIZE)
    }

    fn table_len(&self) -> Result<usize, UnpackError> {
        let total = self
            .offsets
            .checked_add(self.seeds)
            .and_then(|sum| sum.checked_add(self.blocks))
            .and_then(|sum| sum.checked_add(self.methods))
            .filter(|&total| total <= MAX_TABLE_BYTES)
            .ok_or(UnpackError::TooLarge)?;
        usize::try_from(total).map_err(|_| UnpackError::TooLarge)
    }

    fn parse(&self, header: &TocHeader, bytes: &[u8]) -> Result<ChunkTable, UnpackError> {
        let cut = |value: u64| usize::try_from(value).map_err(|_| UnpackError::TooLarge);
        let offsets_end = cut(self.offsets)?;
        let blocks_start = cut(self.offsets.saturating_add(self.seeds))?;
        let methods_start = cut(self.offsets + self.seeds + self.blocks)?;
        let malformed = || UnpackError::Malformed("table bounds");
        let chunks = bytes
            .get(..offsets_end)
            .ok_or_else(malformed)?
            .as_chunks::<OFFSET_AND_LENGTH_BYTES>()
            .0
            .iter()
            .map(|entry| (be40(&entry[..5]), be40(&entry[5..])))
            .collect();
        let blocks = bytes
            .get(blocks_start..methods_start)
            .ok_or_else(malformed)?
            .as_chunks::<BLOCK_ENTRY_BYTES>()
            .0
            .iter()
            .map(|entry| parse_block(entry))
            .collect::<Result<_, _>>()?;
        let methods = parse_methods(
            bytes.get(methods_start..).ok_or_else(malformed)?,
            header.compression_method_name_length,
        );
        Ok(ChunkTable {
            block_size: u64::from(header.compression_block_size),
            chunks,
            blocks,
            methods,
        })
    }
}

fn parse_block(entry: &[u8]) -> Result<Block, UnpackError> {
    let size = |value: u64| u32::try_from(value).map_err(|_| UnpackError::Malformed("block"));
    Ok(Block {
        offset: le(&entry[..5]),
        compressed: size(le(&entry[5..8]))?,
        uncompressed: size(le(&entry[8..11]))?,
        method: entry[11],
    })
}

fn parse_methods(bytes: &[u8], name_length: u32) -> Vec<String> {
    let Ok(length) = usize::try_from(name_length) else {
        return Vec::new();
    };
    if length == 0 {
        return Vec::new();
    }
    bytes
        .chunks_exact(length)
        .map(|name| {
            let end = name
                .iter()
                .position(|&byte| byte == 0)
                .unwrap_or(name.len());
            String::from_utf8_lossy(&name[..end]).into_owned()
        })
        .collect()
}
