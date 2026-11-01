use crate::{MAX_DIRECTORY_INDEX_BYTES, PakError, TOC_HEADER_SIZE, TOC_MAGIC};

const BLOCK_ENTRY_SIZE: u32 = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContainerFlags(pub u8);

impl ContainerFlags {
    pub const COMPRESSED: u8 = 0x01;
    pub const ENCRYPTED: u8 = 0x02;
    pub const SIGNED: u8 = 0x04;
    pub const INDEXED: u8 = 0x08;
    pub const ON_DEMAND: u8 = 0x10;

    pub fn has(self, bit: u8) -> bool {
        self.0 & bit == bit
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TocHeader {
    pub version: u8,
    pub header_size: u32,
    pub entry_count: u32,
    pub compressed_block_entry_count: u32,
    pub compressed_block_entry_size: u32,
    pub compression_method_name_count: u32,
    pub compression_method_name_length: u32,
    pub compression_block_size: u32,
    pub directory_index_size: u32,
    pub partition_count: u32,
    pub container_id: u64,
    pub encryption_key_guid: [u8; 16],
    pub flags: ContainerFlags,
    pub perfect_hash_seeds_count: u32,
    pub partition_size: u64,
    pub chunks_without_perfect_hash_count: u32,
}

type HeaderBytes = [u8; TOC_HEADER_SIZE];

fn u32_at(head: &HeaderBytes, offset: usize) -> u32 {
    let mut word = [0u8; 4];
    word.copy_from_slice(&head[offset..offset + 4]);
    u32::from_le_bytes(word)
}

fn u64_at(head: &HeaderBytes, offset: usize) -> u64 {
    let mut word = [0u8; 8];
    word.copy_from_slice(&head[offset..offset + 8]);
    u64::from_le_bytes(word)
}

/// §4.1 checks in order, up to and including TooLarge. `bytes` = at least the first 144 bytes.
pub fn parse_header(bytes: &[u8]) -> Result<TocHeader, PakError> {
    let head: &HeaderBytes = bytes
        .get(..TOC_HEADER_SIZE)
        .and_then(|b| b.try_into().ok())
        .ok_or(PakError::NotIoStore)?;
    if head[..16] != TOC_MAGIC {
        return Err(PakError::NotIoStore);
    }
    let version = head[16];
    if !(2..=8).contains(&version) {
        return Err(PakError::UnsupportedVersion(version));
    }
    let mut guid = [0u8; 16];
    guid.copy_from_slice(&head[64..80]);
    let header = TocHeader {
        version,
        header_size: u32_at(head, 20),
        entry_count: u32_at(head, 24),
        compressed_block_entry_count: u32_at(head, 28),
        compressed_block_entry_size: u32_at(head, 32),
        compression_method_name_count: u32_at(head, 36),
        compression_method_name_length: u32_at(head, 40),
        compression_block_size: u32_at(head, 44),
        directory_index_size: u32_at(head, 48),
        partition_count: u32_at(head, 52),
        container_id: u64_at(head, 56),
        encryption_key_guid: guid,
        flags: ContainerFlags(head[80]),
        perfect_hash_seeds_count: u32_at(head, 84),
        partition_size: u64_at(head, 88),
        chunks_without_perfect_hash_count: u32_at(head, 96),
    };
    check_header(&header)?;
    Ok(header)
}

fn check_header(header: &TocHeader) -> Result<(), PakError> {
    if usize::try_from(header.header_size).ok() != Some(TOC_HEADER_SIZE) {
        return Err(PakError::Malformed("header size"));
    }
    if header.compressed_block_entry_size != BLOCK_ENTRY_SIZE {
        return Err(PakError::Malformed("block entry size"));
    }
    if header.flags.has(ContainerFlags::ENCRYPTED) {
        return Err(PakError::Encrypted);
    }
    if !header.flags.has(ContainerFlags::INDEXED) || header.directory_index_size == 0 {
        return Err(PakError::NoDirectoryIndex);
    }
    if header.directory_index_size > MAX_DIRECTORY_INDEX_BYTES {
        return Err(PakError::TooLarge);
    }
    Ok(())
}
