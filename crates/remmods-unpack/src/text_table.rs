//! The game's text tables (`Localize/<language>/GTB_*`): `key → text` records inside a package.

use crate::package::u32_at;

/// Each record starts with this tag, then u32 key length, u32 text length, an 8-byte hash, the
/// key and the text. Both lengths include the terminating NUL and padding.
const RECORD_TAG: &[u8] = b"text\x10\0\0\0";
const LENGTHS_AT: usize = 8;
const KEY_AT: usize = 24;
/// No key or text comes near this; larger values mean the tag was data.
const MAX_KEY: usize = 400;
const MAX_TEXT: usize = 4000;

fn terminated(bytes: &[u8]) -> Option<String> {
    let end = bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8(bytes[..end].to_vec()).ok()
}

/// One record at `start`, and where the next search continues.
fn record_at(bytes: &[u8], start: usize) -> Option<(String, String, usize)> {
    let key_len = usize::try_from(u32_at(bytes, start.checked_add(LENGTHS_AT)?)?).ok()?;
    let text_len = usize::try_from(u32_at(bytes, start.checked_add(LENGTHS_AT + 4)?)?).ok()?;
    if key_len > MAX_KEY || text_len > MAX_TEXT {
        return None;
    }
    let key_start = start.checked_add(KEY_AT)?;
    let text_start = key_start.checked_add(key_len)?;
    let end = text_start.checked_add(text_len)?;
    let key = terminated(bytes.get(key_start..text_start)?)?;
    let text = terminated(bytes.get(text_start..end)?)?;
    Some((key, text, end))
}

/// Every `key → text` record, in file order. Bytes that only look like a record are skipped.
pub fn parse_text_table(bytes: &[u8]) -> Vec<(String, String)> {
    let mut records = Vec::new();
    let mut position = 0;
    while let Some(found) = bytes
        .get(position..)
        .and_then(|rest| rest.windows(RECORD_TAG.len()).position(|w| w == RECORD_TAG))
    {
        let start = position + found;
        match record_at(bytes, start) {
            Some((key, text, end)) => {
                records.push((key, text));
                position = end;
            }
            None => position = start + RECORD_TAG.len(),
        }
    }
    records
}
