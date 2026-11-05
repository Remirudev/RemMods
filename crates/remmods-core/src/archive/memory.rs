//! Decoder memory from header properties, checked before any decoding. sevenz-rust2 has no
//! memory-limit API and xz/LZMA/PPMd dictionaries are allocated up front, so a crafted header
//! could otherwise make the app allocate gigabytes.

use crate::platform;

const MIB: u64 = 1 << 20;
const GIB: u64 = 1 << 30;
/// BZip2, Deflate, Deflate64, Copy and the BCJ/Delta filters stay far below this.
const SMALL_CODER: u64 = 64 * MIB;
const LZMA2_MAX_PROP: u8 = 40;

/// `min(4 GiB, physical RAM / 2)`; 2 GiB when the RAM size is unknown.
pub fn memory_limit() -> u64 {
    platform::total_memory().map_or(2 * GIB, |total| (total / 2).min(4 * GIB))
}

pub(crate) enum CoderMemory {
    Known(u64),
    /// A dictionary-based method whose properties do not parse.
    BadProps,
    Unknown,
}

pub(crate) fn sevenz_coder(method_id: &[u8], props: &[u8]) -> CoderMemory {
    let parsed = match method_id {
        [0x03, 0x01, 0x01] => props.get(1..5).map(|dict| u64::from(u32_le(dict))),
        [0x21] => props.first().and_then(|&prop| lzma2_dict(prop)),
        [0x03, 0x04, 0x01] => props.get(1..5).map(|mem| ppmd_memory(u32_le(mem))),
        // Copy, Delta, the BCJ family, BZip2, Deflate, Deflate64, ARM64, RISC-V and AES (the
        // last one is refused as Encrypted elsewhere).
        [0x00]
        | [0x03]
        | [0x03, 0x03, _, _]
        | [0x04, 0x02, 0x02]
        | [0x04, 0x01, 0x08 | 0x09]
        | [0x0A]
        | [0x0B]
        | [0x06, 0xF1, 0x07, 0x01] => return CoderMemory::Known(SMALL_CODER),
        _ => return CoderMemory::Unknown,
    };
    parsed.map_or(CoderMemory::BadProps, CoderMemory::Known)
}

/// None = unknown method or unparsable properties.
pub fn sevenz_coder_memory(method_id: &[u8], props: &[u8]) -> Option<u64> {
    match sevenz_coder(method_id, props) {
        CoderMemory::Known(bytes) => Some(bytes),
        CoderMemory::BadProps | CoderMemory::Unknown => None,
    }
}

/// zip LZMA data starts with 2 B version, 2 B props size, 5 B props; dict = u32 LE at props[1..5].
pub fn zip_lzma_memory(raw_head: &[u8]) -> Option<u64> {
    let props_size = u16::from_le_bytes([*raw_head.get(2)?, *raw_head.get(3)?]);
    if props_size < 5 {
        return None;
    }
    raw_head.get(5..9).map(|dict| u64::from(u32_le(dict)))
}

/// 12 B stream header, then the first block header's filter flags; LZMA2 (0x21) props byte p:
/// dict = (2 | (p & 1)) << (p / 2 + 11), p = 40 → 4 GiB − 1.
pub fn zip_xz_memory(raw_head: &[u8]) -> Option<u64> {
    if raw_head.get(..6)? != [0xFD, b'7', b'z', b'X', b'Z', 0x00] {
        return None;
    }
    let block = raw_head.get(12..)?;
    let size_byte = *block.first()?;
    if size_byte == 0 {
        return Some(0);
    }
    let header = block.get(..(usize::from(size_byte) + 1) * 4)?;
    let mut cursor = XzCursor {
        data: header,
        pos: 2,
    };
    let flags = header[1];
    if flags & 0x40 != 0 {
        cursor.varint()?;
    }
    if flags & 0x80 != 0 {
        cursor.varint()?;
    }
    let mut needed = 0;
    for _ in 0..(flags & 0x03) + 1 {
        let id = cursor.varint()?;
        let size = usize::try_from(cursor.varint()?).ok()?;
        let props = cursor.data.get(cursor.pos..cursor.pos.checked_add(size)?)?;
        cursor.pos += size;
        let filter = match (id, props) {
            (0x21, [prop]) => lzma2_dict(*prop)?,
            (0x21, _) => return None,
            _ => SMALL_CODER,
        };
        needed = needed.max(filter);
    }
    Some(needed)
}

/// 7z: props[1..5] u32 LE; zip: ((p >> 4) & 0xFF) + 1 MiB.
pub fn ppmd_memory(order_mem: u32) -> u64 {
    u64::from(order_mem)
}

/// zip PPMd data starts with a 2-byte LE parameter word.
pub(crate) fn zip_ppmd_memory(raw_head: &[u8]) -> Option<u64> {
    let word = u32::from(u16::from_le_bytes([*raw_head.first()?, *raw_head.get(1)?]));
    Some(ppmd_memory((((word >> 4) & 0xFF) + 1) << 20))
}

/// Bounded parser of a 7z encoded header (first byte 0x17): PackInfo, then the UnpackInfo
/// folders' coders (method id + properties); returns the largest `sevenz_coder_memory`.
/// Never allocates; a malformed or truncated field or an unknown method → None.
pub fn sevenz_header_memory(next_header: &[u8]) -> Option<u64> {
    let mut needed = 0;
    sevenz_header_coders(next_header, |id, props| {
        needed = needed.max(sevenz_coder_memory(id, props)?);
        Some(())
    })?;
    Some(needed)
}

/// Calls `visit(method_id, props)` for every coder of an encoded header's folders; None when
/// the header does not parse or `visit` returns None.
pub(crate) fn sevenz_header_coders(
    next_header: &[u8],
    mut visit: impl FnMut(&[u8], &[u8]) -> Option<()>,
) -> Option<()> {
    let mut cursor = SevenCursor {
        data: next_header,
        pos: 0,
    };
    if cursor.byte()? != 0x17 {
        return None;
    }
    let mut id = cursor.byte()?;
    if id == 0x06 {
        cursor.skip_pack_info()?;
        id = cursor.byte()?;
    }
    if id != 0x07 || cursor.byte()? != 0x0B {
        return None;
    }
    let folders = cursor.bounded_count()?;
    if cursor.byte()? != 0 {
        return None;
    }
    for _ in 0..folders {
        cursor.folder_coders(&mut visit)?;
    }
    Some(())
}

fn lzma2_dict(prop: u8) -> Option<u64> {
    match prop {
        LZMA2_MAX_PROP => Some(u64::from(u32::MAX)),
        0..LZMA2_MAX_PROP => Some((2 | u64::from(prop & 1)) << (prop / 2 + 11)),
        _ => None,
    }
}

fn u32_le(bytes: &[u8]) -> u32 {
    bytes
        .iter()
        .take(4)
        .enumerate()
        .fold(0, |value, (shift, &byte)| {
            value | u32::from(byte) << (8 * shift)
        })
}

struct XzCursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl XzCursor<'_> {
    fn varint(&mut self) -> Option<u64> {
        let mut value = 0u64;
        for shift in 0..9 {
            let byte = *self.data.get(self.pos)?;
            self.pos += 1;
            value |= u64::from(byte & 0x7F) << (7 * shift);
            if byte & 0x80 == 0 {
                return Some(value);
            }
        }
        None
    }
}

struct SevenCursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> SevenCursor<'a> {
    fn byte(&mut self) -> Option<u8> {
        let byte = *self.data.get(self.pos)?;
        self.pos += 1;
        Some(byte)
    }

    fn bytes(&mut self, count: u64) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(usize::try_from(count).ok()?)?;
        let slice = self.data.get(self.pos..end)?;
        self.pos = end;
        Some(slice)
    }

    /// 7z variable-length number: the leading one bits of the first byte count extra bytes.
    fn number(&mut self) -> Option<u64> {
        let first = self.byte()?;
        let mut mask = 0x80u8;
        let mut value = 0u64;
        for index in 0..8 {
            if first & mask == 0 {
                let high = u64::from(first & mask.wrapping_sub(1));
                return Some(value | (high << (8 * index)));
            }
            value |= u64::from(self.byte()?) << (8 * index);
            mask >>= 1;
        }
        Some(value)
    }

    /// A count that can never exceed the header length (one byte per item at least).
    fn bounded_count(&mut self) -> Option<u64> {
        let count = self.number()?;
        (count <= self.data.len() as u64).then_some(count)
    }

    fn skip_pack_info(&mut self) -> Option<()> {
        self.number()?;
        let streams = self.bounded_count()?;
        loop {
            match self.byte()? {
                0x00 => return Some(()),
                0x09 => {
                    for _ in 0..streams {
                        self.number()?;
                    }
                }
                0x0A => self.skip_digests(streams)?,
                _ => return None,
            }
        }
    }

    fn skip_digests(&mut self, count: u64) -> Option<()> {
        let defined = if self.byte()? == 0 {
            let bits = self.bytes(count.div_ceil(8))?;
            bits.iter().map(|byte| u64::from(byte.count_ones())).sum()
        } else {
            count
        };
        self.bytes(defined.checked_mul(4)?).map(|_| ())
    }

    fn folder_coders(&mut self, visit: &mut impl FnMut(&[u8], &[u8]) -> Option<()>) -> Option<()> {
        let coders = self.number()?;
        if coders == 0 || coders > 64 {
            return None;
        }
        let (mut total_in, mut total_out) = (0u64, 0u64);
        for _ in 0..coders {
            let flag = self.byte()?;
            if flag & 0x80 != 0 {
                return None;
            }
            let id = self.bytes(u64::from(flag & 0x0F))?;
            let (ins, outs) = if flag & 0x10 != 0 {
                (self.number()?, self.number()?)
            } else {
                (1, 1)
            };
            if ins > 64 || outs > 64 {
                return None;
            }
            total_in += ins;
            total_out += outs;
            let props = if flag & 0x20 != 0 {
                let size = self.number()?;
                self.bytes(size)?
            } else {
                &[]
            };
            visit(id, props)?;
        }
        let pairs = total_out.checked_sub(1)?;
        for _ in 0..pairs {
            self.number()?;
            self.number()?;
        }
        let packed = total_in.checked_sub(pairs)?;
        if packed == 0 {
            return None;
        }
        if packed > 1 {
            for _ in 0..packed {
                self.number()?;
            }
        }
        Some(())
    }
}
