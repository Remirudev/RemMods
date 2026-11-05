//! The two parts of a cooked (zen) package a texture reader needs: its bulk data map and its body.

use crate::UnpackError;

const NAME_BATCH_OFFSET: usize = 44;
const NAME_HASH_SIZE: usize = 8;
const NAME_HEADER_SIZE: usize = 2;
const BULK_ENTRY_SIZE: usize = 32;
const WIDE_NAME: u8 = 0x80;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BulkEntry {
    pub offset: i64,
    pub size: i64,
    pub flags: u32,
}

#[derive(Debug)]
pub(crate) struct Package<'a> {
    pub body: &'a [u8],
    pub bulk: Vec<BulkEntry>,
}

pub(crate) fn u32_at(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset.checked_add(4)?)?;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

pub(crate) fn i32_at(data: &[u8], offset: usize) -> Option<i32> {
    u32_at(data, offset).map(|value| i32::from_le_bytes(value.to_le_bytes()))
}

pub(crate) fn i64_at(data: &[u8], offset: usize) -> Option<i64> {
    let bytes = data.get(offset..offset.checked_add(8)?)?;
    Some(i64::from_le_bytes(bytes.try_into().ok()?))
}

/// Where the name batch ends: the names are skipped, a texture needs none of them.
fn names_end(data: &[u8]) -> Option<usize> {
    let count = usize::try_from(u32_at(data, NAME_BATCH_OFFSET)?).ok()?;
    if count == 0 {
        return NAME_BATCH_OFFSET.checked_add(4);
    }
    let hashes = NAME_BATCH_OFFSET.checked_add(16)?;
    let heads = hashes.checked_add(count.checked_mul(NAME_HASH_SIZE)?)?;
    let strings = heads.checked_add(count.checked_mul(NAME_HEADER_SIZE)?)?;
    let table = data.get(heads..strings)?;
    let mut end = strings;
    for head in table.as_chunks::<NAME_HEADER_SIZE>().0 {
        let length = usize::from(u16::from_be_bytes(*head) & 0x7FFF);
        let bytes = if head[0] & WIDE_NAME != 0 {
            length.checked_mul(2)?
        } else {
            length
        };
        end = end.checked_add(bytes)?;
    }
    (end <= data.len()).then_some(end)
}

pub(crate) fn parse(data: &[u8]) -> Result<Package<'_>, UnpackError> {
    let malformed = UnpackError::Malformed("package");
    let Some(0) = u32_at(data, 0) else {
        return Err(UnpackError::NotATexture);
    };
    let body_start = u32_at(data, 4)
        .and_then(|size| usize::try_from(size).ok())
        .ok_or(UnpackError::Malformed("package"))?;
    let body = data.get(body_start..).ok_or(malformed)?;
    let map_start = names_end(data).ok_or(UnpackError::Malformed("package names"))?;
    let map_size = i64_at(data, map_start)
        .and_then(|size| usize::try_from(size).ok())
        .ok_or(UnpackError::Malformed("bulk map"))?;
    let entries_start = map_start.checked_add(8);
    let entries = entries_start
        .and_then(|start| data.get(start..start.checked_add(map_size)?))
        .ok_or(UnpackError::Malformed("bulk map"))?;
    let bulk = entries
        .as_chunks::<BULK_ENTRY_SIZE>()
        .0
        .iter()
        .map(|entry| {
            Some(BulkEntry {
                offset: i64_at(entry, 0)?,
                size: i64_at(entry, 16)?,
                flags: u32_at(entry, 24)?,
            })
        })
        .collect::<Option<Vec<_>>>()
        .ok_or(UnpackError::Malformed("bulk map"))?;
    Ok(Package { body, bulk })
}
