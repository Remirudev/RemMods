use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::header::{ContainerFlags, TocHeader, parse_header};
use crate::index::parse_directory_index;
use crate::{PakError, TOC_HEADER_SIZE};

const HEADER_READ_LEN: u64 = 144;
const CHUNK_ID_SIZE: u64 = 12;
const OFFSET_AND_LENGTH_SIZE: u64 = 10;
const HASH_SEED_SIZE: u64 = 4;
const BLOCK_ENTRY_SIZE: u64 = 12;
const BLOCK_HASH_SIZE: u64 = 20;
const MAX_SIGNATURE_HASH_SIZE: i32 = 64;
/// `FIoStoreTocEntryMetaOld` (v2..=7): 32-byte `FIoChunkHash` + flags byte.
const CHUNK_META_SIZE_OLD: u64 = 33;
/// `FIoStoreTocEntryMeta` (v8): 20-byte `FIoHash` + flags byte + 3 padding bytes.
const CHUNK_META_SIZE_IO_HASH: u64 = 24;
/// Version `ReplaceIoChunkHashWithIoHash`.
const IO_HASH_VERSION: u8 = 8;

/// The 20 meaningful bytes of a chunk hash (`FIoHash`; the first 20 of an old `FIoChunkHash`).
pub type ContentHash = [u8; 20];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TocFile {
    /// Display path (§4.4).
    pub path: String,
    pub chunk_index: u32,
    /// Hash of the uncompressed chunk from the chunk metas after the index; `None` when the
    /// metas are missing or truncated or the hash is all zero.
    pub content_hash: Option<ContentHash>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TocListing {
    pub header: TocHeader,
    /// As stored in the index.
    pub mount_point: String,
    /// Sorted by display path.
    pub files: Vec<TocFile>,
}

/// §4.2 without the signature part: the offset of the signature size (signed) or of the index.
pub fn directory_index_offset(header: &TocHeader) -> Result<u64, PakError> {
    let entries = u64::from(header.entry_count);
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
    let terms = [
        (entries, CHUNK_ID_SIZE),
        (entries, OFFSET_AND_LENGTH_SIZE),
        (seeds, HASH_SEED_SIZE),
        (overflow, HASH_SEED_SIZE),
        (
            u64::from(header.compressed_block_entry_count),
            BLOCK_ENTRY_SIZE,
        ),
        (
            u64::from(header.compression_method_name_count),
            u64::from(header.compression_method_name_length),
        ),
    ];
    terms
        .iter()
        .try_fold(HEADER_READ_LEN, |sum, &(count, size)| {
            sum.checked_add(count.checked_mul(size)?)
        })
        .ok_or(PakError::Malformed("offset"))
}

/// Bytes per chunk meta: 33 up to v7, 24 from v8.
pub fn chunk_meta_size(version: u8) -> u64 {
    if version >= IO_HASH_VERSION {
        CHUNK_META_SIZE_IO_HASH
    } else {
        CHUNK_META_SIZE_OLD
    }
}

/// Opens read-only, reads only header, signature size, index and chunk metas (§4.2).
pub fn read_toc(path: &Path) -> Result<TocListing, PakError> {
    let mut file = File::open(path)?;
    let total_len = file.metadata()?.len();
    let mut head = Vec::with_capacity(TOC_HEADER_SIZE);
    (&mut file).take(HEADER_READ_LEN).read_to_end(&mut head)?;
    listing_from(&head, |offset, len| {
        check_bounds(offset, len, total_len)?;
        let mut buf = vec![0u8; len];
        file.seek(SeekFrom::Start(offset))?;
        file.read_exact(&mut buf)?;
        Ok(buf)
    })
}

pub fn read_toc_bytes(bytes: &[u8]) -> Result<TocListing, PakError> {
    let total_len = u64::try_from(bytes.len()).map_err(|_| PakError::Malformed("bounds"))?;
    listing_from(bytes, |offset, len| {
        check_bounds(offset, len, total_len)?;
        let start = usize::try_from(offset).map_err(|_| PakError::Malformed("bounds"))?;
        let end = start
            .checked_add(len)
            .ok_or(PakError::Malformed("bounds"))?;
        bytes
            .get(start..end)
            .map(<[u8]>::to_vec)
            .ok_or(PakError::Malformed("bounds"))
    })
}

fn check_bounds(offset: u64, len: usize, total_len: u64) -> Result<(), PakError> {
    let len = u64::try_from(len).map_err(|_| PakError::Malformed("bounds"))?;
    match offset.checked_add(len) {
        Some(end) if end <= total_len => Ok(()),
        _ => Err(PakError::Malformed("bounds")),
    }
}

fn listing_from(
    head: &[u8],
    mut read_at: impl FnMut(u64, usize) -> Result<Vec<u8>, PakError>,
) -> Result<TocListing, PakError> {
    let header = parse_header(head)?;
    let mut offset = directory_index_offset(&header)?;
    if header.flags.has(ContainerFlags::SIGNED) {
        offset = skip_signatures(&header, offset, &mut read_at)?;
    }
    let index_len =
        usize::try_from(header.directory_index_size).map_err(|_| PakError::Malformed("bounds"))?;
    let index_bytes = read_at(offset, index_len)?;
    let index = parse_directory_index(&index_bytes)?;
    let mut files = index.files(header.entry_count)?;
    if let Some(metas) = offset
        .checked_add(u64::from(header.directory_index_size))
        .and_then(|start| read_chunk_metas(&header, start, &mut read_at))
    {
        attach_hashes(&mut files, &metas, chunk_meta_size(header.version));
    }
    Ok(TocListing {
        header,
        mount_point: index.mount_point,
        files,
    })
}

fn skip_signatures(
    header: &TocHeader,
    offset: u64,
    read_at: &mut impl FnMut(u64, usize) -> Result<Vec<u8>, PakError>,
) -> Result<u64, PakError> {
    let size_bytes: [u8; 4] = read_at(offset, 4)?
        .try_into()
        .map_err(|_| PakError::Malformed("signature"))?;
    let hash_size = i32::from_le_bytes(size_bytes);
    if !(0..=MAX_SIGNATURE_HASH_SIZE).contains(&hash_size) {
        return Err(PakError::Malformed("signature"));
    }
    let hash_size = u64::from(hash_size.unsigned_abs());
    let blocks = u64::from(header.compressed_block_entry_count);
    offset
        .checked_add(4)
        .and_then(|o| o.checked_add(hash_size.checked_mul(2)?))
        .and_then(|o| o.checked_add(blocks.checked_mul(BLOCK_HASH_SIZE)?))
        .ok_or(PakError::Malformed("offset"))
}

/// The chunk-meta table right after the index, or `None` when it does not fit in the file.
fn read_chunk_metas(
    header: &TocHeader,
    start: u64,
    read_at: &mut impl FnMut(u64, usize) -> Result<Vec<u8>, PakError>,
) -> Option<Vec<u8>> {
    let len = u64::from(header.entry_count).checked_mul(chunk_meta_size(header.version))?;
    read_at(start, usize::try_from(len).ok()?).ok()
}

fn attach_hashes(files: &mut [TocFile], metas: &[u8], meta_size: u64) {
    let Ok(meta_size) = usize::try_from(meta_size) else {
        return;
    };
    for file in files {
        let start = usize::try_from(file.chunk_index)
            .ok()
            .and_then(|i| i.checked_mul(meta_size));
        file.content_hash = start
            .and_then(|start| metas.get(start..start.checked_add(20)?))
            .and_then(|bytes| ContentHash::try_from(bytes).ok())
            .filter(|hash| hash.iter().any(|&b| b != 0));
    }
}
