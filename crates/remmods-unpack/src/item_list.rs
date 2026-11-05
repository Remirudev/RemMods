//! The game's item list as the TKData mod loader extracts it (`customize_item_common_list.bin`,
//! a FlatBuffer): which text key names each item prefab.

use crate::UnpackError;
use crate::package::{i32_at, u32_at};

const FIELD_DATA: usize = 1;
const FIELD_ENTRIES: usize = 0;
const FIELD_PREFAB: usize = 2;
const FIELD_TEXT_KEY: usize = 5;

fn u16_at(bytes: &[u8], offset: usize) -> Option<usize> {
    let pair = bytes.get(offset..offset.checked_add(2)?)?;
    Some(usize::from(u16::from_le_bytes(pair.try_into().ok()?)))
}

/// Follows a u32 offset stored at `at`, relative to `at`.
fn follow(bytes: &[u8], at: usize) -> Option<usize> {
    at.checked_add(usize::try_from(u32_at(bytes, at)?).ok()?)
}

/// Where `field` of the table at `table` is stored, `None` when the table leaves it out.
fn field(bytes: &[u8], table: usize, field: usize) -> Option<usize> {
    let back = isize::try_from(i32_at(bytes, table)?).ok()?;
    let vtable = table.checked_add_signed(-back)?;
    let slot = vtable.checked_add(4)?.checked_add(field.checked_mul(2)?)?;
    if slot.checked_add(2)? > vtable.checked_add(u16_at(bytes, vtable)?)? {
        return None;
    }
    match u16_at(bytes, slot)? {
        0 => None,
        offset => table.checked_add(offset),
    }
}

fn string(bytes: &[u8], at: usize) -> Option<String> {
    let start = follow(bytes, at)?;
    let length = usize::try_from(u32_at(bytes, start)?).ok()?;
    let text = bytes.get(start.checked_add(4)?..start.checked_add(4)?.checked_add(length)?)?;
    String::from_utf8(text.to_vec()).ok()
}

/// `(prefab, text key)` of every entry that has both.
pub fn parse_item_list(bytes: &[u8]) -> Result<Vec<(String, String)>, UnpackError> {
    let root = follow(bytes, 0).ok_or(UnpackError::ItemList("root"))?;
    let data = field(bytes, root, FIELD_DATA)
        .and_then(|at| follow(bytes, at))
        .ok_or(UnpackError::ItemList("data"))?;
    let vector = field(bytes, data, FIELD_ENTRIES)
        .and_then(|at| follow(bytes, at))
        .ok_or(UnpackError::ItemList("entries"))?;
    let count = u32_at(bytes, vector).ok_or(UnpackError::ItemList("entries"))?;
    let count = usize::try_from(count).map_err(|_| UnpackError::ItemList("entries"))?;
    let room = bytes.len().saturating_sub(vector) / 4;
    if count > room {
        return Err(UnpackError::ItemList("entries"));
    }
    let mut items = Vec::with_capacity(count);
    for index in 0..count {
        let slot = vector + 4 + index * 4;
        let Some(table) = follow(bytes, slot) else {
            continue;
        };
        let prefab = field(bytes, table, FIELD_PREFAB).and_then(|at| string(bytes, at));
        let key = field(bytes, table, FIELD_TEXT_KEY).and_then(|at| string(bytes, at));
        if let (Some(prefab), Some(key)) = (prefab, key) {
            items.push((prefab, key));
        }
    }
    Ok(items)
}
