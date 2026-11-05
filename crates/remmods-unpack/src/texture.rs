//! The pixel data of a cooked `UTexture2D` package: format, mip sizes and where each mip's payload
//! sits (inline in the package or in its `.ubulk`).

use crate::UnpackError;
use crate::package::{self, Package, i32_at, u32_at};

const BULK_SEPARATE_FILE: u32 = 0x100;
/// Optional (`.uptnl`) and memory-mapped (`.m.ubulk`) payloads: neither sits in the `.ubulk`.
const BULK_OTHER_FILE: u32 = 0x800 | 0x1000;
/// `FTexturePlatformData`: the packed word's "has CPU copy / opt data" bit adds 8 bytes.
const PACKED_HAS_OPT_DATA: u32 = 1 << 30;
const MAX_MIPS: i32 = 16;
const MAX_FORMAT_NAME: usize = 32;
/// Far above any item picture; a crafted size must not allocate gigabytes.
pub(crate) const MAX_EDGE: u32 = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PixelFormat {
    Dxt1,
    Dxt5,
    Bc7,
    Bgra8,
}

fn known_format(name: &str) -> Option<PixelFormat> {
    match name {
        "PF_DXT1" => Some(PixelFormat::Dxt1),
        "PF_DXT5" => Some(PixelFormat::Dxt5),
        "PF_BC7" => Some(PixelFormat::Bc7),
        "PF_B8G8R8A8" => Some(PixelFormat::Bgra8),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Mip {
    pub width: u32,
    pub height: u32,
    offset: usize,
    size: usize,
    separate: bool,
    readable: bool,
}

#[derive(Debug)]
pub(crate) struct Texture<'a> {
    pub format: PixelFormat,
    pub mips: Vec<Mip>,
    body: &'a [u8],
    bulk: Option<&'a [u8]>,
}

/// The first `PF_…` FString at or after `from`: its start and its name.
fn format_name(body: &[u8], from: usize) -> Option<(usize, &str)> {
    let mut search = from;
    loop {
        let found = body.get(search..)?.windows(3).position(|w| w == b"PF_")?;
        let start = search + found;
        search = start + 3;
        if let Some(name) = name_at(body, start) {
            return Some((start, name));
        }
    }
}

/// The text at `start` when an FString's length comes right before it.
fn name_at(body: &[u8], start: usize) -> Option<&str> {
    let length = usize::try_from(i32_at(body, start.checked_sub(4)?)?).ok()?;
    if !(4..=MAX_FORMAT_NAME).contains(&length) {
        return None;
    }
    let bytes = body.get(start..start.checked_add(length)?)?;
    let (terminator, text) = bytes.split_last()?;
    if *terminator != 0 || !text.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'_') {
        return None;
    }
    std::str::from_utf8(text).ok()
}

fn size_of(value: i32) -> Option<u32> {
    u32::try_from(value).ok().filter(|&edge| edge <= MAX_EDGE)
}

fn parse_mips(package: &Package<'_>, start: usize) -> Option<Vec<Mip>> {
    let body = package.body;
    let count = i32_at(body, start.checked_add(4)?)?;
    if !(1..=MAX_MIPS).contains(&count) {
        return None;
    }
    let mut position = start.checked_add(8)?;
    let mut mips = Vec::new();
    for _ in 0..count {
        let index = usize::try_from(i32_at(body, position)?).ok()?;
        position = position.checked_add(4)?;
        let entry = package.bulk.get(index)?;
        let offset = usize::try_from(entry.offset).ok()?;
        let size = usize::try_from(entry.size).ok()?;
        let separate = entry.flags & BULK_SEPARATE_FILE != 0;
        if !separate {
            if offset != position {
                return None;
            }
            position = position.checked_add(size)?;
        }
        let width = size_of(i32_at(body, position)?)?;
        let height = size_of(i32_at(body, position.checked_add(4)?)?)?;
        position = position.checked_add(12)?;
        mips.push(Mip {
            width,
            height,
            offset,
            size,
            separate,
            readable: entry.flags & BULK_OTHER_FILE == 0,
        });
    }
    Some(mips)
}

pub(crate) fn parse<'a>(
    data: &'a [u8],
    bulk: Option<&'a [u8]>,
) -> Result<Texture<'a>, UnpackError> {
    let package = package::parse(data)?;
    let body = package.body;
    let mut search = 0;
    while let Some((start, name)) = format_name(body, search) {
        search = start + 3;
        let Some(format) = known_format(name) else {
            return Err(UnpackError::UnsupportedFormat(name.to_owned()));
        };
        let packed = start.checked_sub(8).and_then(|at| u32_at(body, at));
        let Some(packed) = packed else { continue };
        let length = name.len() + 1;
        let extra = if packed & PACKED_HAS_OPT_DATA != 0 {
            8
        } else {
            0
        };
        if let Some(mips) = parse_mips(&package, start + length + extra) {
            return Ok(Texture {
                format,
                mips,
                body,
                bulk,
            });
        }
    }
    Err(UnpackError::NotATexture)
}

impl Texture<'_> {
    fn payload(&self, mip: &Mip) -> Option<&[u8]> {
        if !mip.readable {
            return None;
        }
        let source = if mip.separate { self.bulk? } else { self.body };
        source.get(mip.offset..mip.offset.checked_add(mip.size)?)
    }

    /// The smallest mip with a readable payload that still covers `max_edge`, else the largest
    /// readable one.
    pub(crate) fn best_mip(&self, max_edge: u32) -> Option<(Mip, &[u8])> {
        let readable = self
            .mips
            .iter()
            .filter_map(|mip| self.payload(mip).map(|payload| (*mip, payload)));
        let edge = |mip: &Mip| mip.width.max(mip.height);
        let (covering, smaller): (Vec<_>, Vec<_>) =
            readable.partition(|(mip, _)| edge(mip) >= max_edge);
        covering
            .into_iter()
            .min_by_key(|(mip, _)| edge(mip))
            .or_else(|| smaller.into_iter().max_by_key(|(mip, _)| edge(mip)))
    }
}
