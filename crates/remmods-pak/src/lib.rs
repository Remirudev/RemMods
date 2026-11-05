//! Pure-Rust reader of IoStore `.utoc` headers and directory indexes (SPEC-P2B §4).
//!
//! No decompression and no decryption: only the 144-byte header, the signature size, the
//! directory index and the chunk metas (content hashes) are read. Every length, count and offset
//! is checked, so no input panics.
#![forbid(unsafe_code)]

pub mod hints;

#[doc(hidden)]
pub mod fixture;

mod error;
mod header;
mod index;
mod keys;
mod read;

pub use error::PakError;
pub use header::{ContainerFlags, TocHeader, parse_header};
pub use index::{DirectoryEntry, DirectoryIndex, FileEntry, parse_directory_index};
pub use keys::{asset_key, display_path, file_key};
pub use read::{
    ContentHash, TocFile, TocListing, chunk_meta_size, directory_index_offset, read_toc,
    read_toc_bytes,
};

pub const TOC_MAGIC: [u8; 16] = *b"-==--==--==--==-";
pub const TOC_HEADER_SIZE: usize = 144;
pub const MAX_DIRECTORY_INDEX_BYTES: u32 = 64 * 1024 * 1024;
pub const INVALID_INDEX: u32 = u32::MAX;
