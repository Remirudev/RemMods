//! Test-only builders of the formats this crate reads. Pure bytes, never touches the file system.

const NAME: &[u8] = b"Texture";
const TEXT_RECORD_TAG: &[u8] = b"text\x10\0\0\0";
const BULK_SEPARATE_FILE: u32 = 0x100;
const FIELD_SIZE: u16 = 4;

/// One mip of `texture_package`.
#[derive(Clone, Debug)]
pub struct MipSpec<'a> {
    pub width: i32,
    pub height: i32,
    pub payload: &'a [u8],
    /// Stored in the `.ubulk` at this offset instead of inline.
    pub bulk_at: Option<i64>,
    /// Bulk flags on top of the ones the placement needs.
    pub extra_flags: u32,
}

fn push_i32(out: &mut Vec<u8>, value: i32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn size_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn size_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// The body of a cooked texture, and the bulk map entries its mips refer to.
fn texture_body(format: &str, mips: &[MipSpec<'_>]) -> (Vec<u8>, Vec<u8>) {
    let mut body = b"property tags stand here".to_vec();
    push_u32(&mut body, 0);
    push_i32(&mut body, size_i32(format.len() + 1));
    body.extend_from_slice(format.as_bytes());
    body.push(0);
    push_i32(&mut body, 0);
    push_i32(&mut body, size_i32(mips.len()));
    let mut map = Vec::new();
    for (index, mip) in mips.iter().enumerate() {
        push_i32(&mut body, size_i32(index));
        let (offset, flags) = match mip.bulk_at {
            Some(offset) => (offset, BULK_SEPARATE_FILE),
            None => (i64::try_from(body.len()).unwrap_or(i64::MAX), 0),
        };
        map.extend_from_slice(&offset.to_le_bytes());
        map.extend_from_slice(&offset.to_le_bytes());
        map.extend_from_slice(&i64::try_from(mip.payload.len()).unwrap_or(0).to_le_bytes());
        push_u32(&mut map, flags | mip.extra_flags);
        push_u32(&mut map, 0);
        if mip.bulk_at.is_none() {
            body.extend_from_slice(mip.payload);
        }
        push_i32(&mut body, mip.width);
        push_i32(&mut body, mip.height);
        push_i32(&mut body, 1);
    }
    (body, map)
}

/// A cooked (zen) package holding one texture in `format` (`PF_…`).
pub fn texture_package(format: &str, mips: &[MipSpec<'_>]) -> Vec<u8> {
    let (body, map) = texture_body(format, mips);
    let mut out = vec![0u8; 44];
    push_u32(&mut out, 1);
    push_u32(&mut out, size_u32(NAME.len()));
    out.extend_from_slice(&[0; 8]);
    out.extend_from_slice(&[0; 8]);
    out.extend_from_slice(&u16::try_from(NAME.len()).unwrap_or(0).to_be_bytes());
    out.extend_from_slice(NAME);
    out.extend_from_slice(&i64::try_from(map.len()).unwrap_or(0).to_le_bytes());
    out.extend_from_slice(&map);
    let header_size = size_u32(out.len());
    out[4..8].copy_from_slice(&header_size.to_le_bytes());
    out.extend_from_slice(&body);
    out
}

/// One 2×2 BGRA texture, all pixels alike; `color` is `[b, g, r, a]`.
pub fn bgra_package(color: [u8; 4]) -> Vec<u8> {
    let payload: Vec<u8> = color.iter().copied().cycle().take(16).collect();
    texture_package(
        "PF_B8G8R8A8",
        &[MipSpec {
            width: 2,
            height: 2,
            payload: &payload,
            bulk_at: None,
            extra_flags: 0,
        }],
    )
}

/// A text table asset: `key → text` records between other bytes.
pub fn text_table(records: &[(&str, &str)]) -> Vec<u8> {
    let mut out = b"package header".to_vec();
    for (key, text) in records {
        out.extend_from_slice(TEXT_RECORD_TAG);
        push_u32(&mut out, size_u32(key.len() + 1));
        push_u32(&mut out, size_u32(text.len() + 1));
        out.extend_from_slice(&[0xAB; 8]);
        out.extend_from_slice(key.as_bytes());
        out.push(0);
        out.extend_from_slice(text.as_bytes());
        out.push(0);
    }
    out.extend_from_slice(b"package end");
    out
}

/// Writes a table at the end of `out`: its vtable (`fields` slots, `slots` naming the used ones),
/// then one u32 reference per used slot. Returns the table's position and where its references sit.
fn push_table(out: &mut Vec<u8>, fields: u16, slots: &[u16]) -> (usize, usize) {
    let vtable = out.len();
    let table_size = FIELD_SIZE * (u16::try_from(slots.len()).unwrap_or(0) + 1);
    out.extend_from_slice(&(4 + fields * 2).to_le_bytes());
    out.extend_from_slice(&table_size.to_le_bytes());
    for field in 0..fields {
        let at = slots.iter().position(|&slot| slot == field);
        let offset = at.map_or(0, |at| FIELD_SIZE * (u16::try_from(at).unwrap_or(0) + 1));
        out.extend_from_slice(&offset.to_le_bytes());
    }
    let table = out.len();
    push_i32(out, size_i32(table - vtable));
    let references = out.len();
    out.resize(references + slots.len() * 4, 0);
    (table, references)
}

fn point(out: &mut [u8], at: usize, target: usize) {
    out[at..at + 4].copy_from_slice(&size_u32(target - at).to_le_bytes());
}

fn push_string(out: &mut Vec<u8>, text: &str) -> usize {
    let at = out.len();
    push_u32(out, size_u32(text.len()));
    out.extend_from_slice(text.as_bytes());
    out.push(0);
    at
}

/// The FlatBuffer of the game's item list: `(prefab, text key)` entries.
pub fn item_list(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut out = vec![0u8; 4];
    let (root, root_data) = push_table(&mut out, 2, &[1]);
    point(&mut out, 0, root);
    let (data, data_entries) = push_table(&mut out, 1, &[0]);
    point(&mut out, root_data, data);
    let vector = out.len();
    point(&mut out, data_entries, vector);
    push_u32(&mut out, size_u32(entries.len()));
    out.resize(out.len() + entries.len() * 4, 0);
    for (index, (prefab, key)) in entries.iter().enumerate() {
        let (table, references) = push_table(&mut out, 6, &[2, 5]);
        point(&mut out, vector + 4 + index * 4, table);
        let prefab_at = push_string(&mut out, prefab);
        point(&mut out, references, prefab_at);
        let key_at = push_string(&mut out, key);
        point(&mut out, references + 4, key_at);
    }
    out
}
