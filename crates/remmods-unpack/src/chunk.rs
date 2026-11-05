//! Reading one chunk out of a `.ucas`: its blocks are found, read and decompressed one by one.

use std::io::{Read, Seek, SeekFrom};

use oozextract::Extractor;

use crate::UnpackError;
use crate::toc::{Block, ChunkTable};

/// Far above any picture or text table; a crafted length must not allocate gigabytes.
const MAX_CHUNK_BYTES: u64 = 64 * 1024 * 1024;
/// A chunk starts anywhere inside a block, and the block size comes from the file too.
const MAX_STREAM_BYTES: usize = 128 * 1024 * 1024;

const OODLE: &str = "Oodle";

impl ChunkTable {
    /// The uncompressed bytes of one chunk.
    pub(crate) fn read_chunk(
        &self,
        ucas: &mut (impl Read + Seek),
        index: u32,
    ) -> Result<Vec<u8>, UnpackError> {
        let entry = usize::try_from(index)
            .ok()
            .and_then(|index| self.chunks.get(index));
        let &(offset, length) = entry.ok_or(UnpackError::Malformed("chunk index"))?;
        if length > MAX_CHUNK_BYTES {
            return Err(UnpackError::TooLarge);
        }
        if length == 0 {
            return Ok(Vec::new());
        }
        let wanted = usize::try_from(length).map_err(|_| UnpackError::TooLarge)?;
        let first = offset / self.block_size;
        let skip =
            usize::try_from(offset - first * self.block_size).map_err(|_| UnpackError::TooLarge)?;
        let first = usize::try_from(first).map_err(|_| UnpackError::Malformed("chunk offset"))?;
        let needed = skip
            .checked_add(wanted)
            .filter(|&needed| needed <= MAX_STREAM_BYTES)
            .ok_or(UnpackError::TooLarge)?;

        let mut stream = Vec::with_capacity(needed);
        let mut extractor = Extractor::new();
        for block in self.blocks.get(first..).unwrap_or_default() {
            if stream.len() >= needed {
                break;
            }
            self.append_block(ucas, block, &mut extractor, &mut stream)?;
        }
        if stream.len() < needed {
            return Err(UnpackError::Malformed("chunk runs past the blocks"));
        }
        stream.truncate(needed);
        stream.drain(..skip);
        Ok(stream)
    }

    fn append_block(
        &self,
        ucas: &mut (impl Read + Seek),
        block: &Block,
        extractor: &mut Extractor,
        stream: &mut Vec<u8>,
    ) -> Result<(), UnpackError> {
        let compressed =
            usize::try_from(block.compressed).map_err(|_| UnpackError::Malformed("block"))?;
        let uncompressed =
            usize::try_from(block.uncompressed).map_err(|_| UnpackError::Malformed("block"))?;
        let mut input = vec![0u8; compressed];
        ucas.seek(SeekFrom::Start(block.offset))?;
        ucas.read_exact(&mut input)?;
        if block.method == 0 {
            let stored = input
                .get(..uncompressed)
                .ok_or(UnpackError::Malformed("stored block"))?;
            stream.extend_from_slice(stored);
            return Ok(());
        }
        let name = self
            .methods
            .get(usize::from(block.method) - 1)
            .ok_or(UnpackError::UnknownMethod)?;
        if name != OODLE {
            return Err(UnpackError::UnknownMethod);
        }
        let start = stream.len();
        stream.resize(start + uncompressed, 0);
        let written = extractor
            .read_from_slice(&input, &mut stream[start..])
            .map_err(|_| UnpackError::Decompress)?;
        if written != uncompressed {
            return Err(UnpackError::Decompress);
        }
        Ok(())
    }
}
