use encoding_rs::Encoding;
use unicode_normalization::UnicodeNormalization;

use super::cp437;
use super::model::{
    ArchiveFormat, DecodedName, Entry, EntryKind, EntryProblem, Limits, Listing, NameEncoding,
    RawEntry, RawEntryName,
};

pub fn decode_name(name: &RawEntryName, encoding: NameEncoding) -> DecodedName {
    match name {
        RawEntryName::Unicode(text) => DecodedName {
            text: text.clone(),
            legacy: false,
            undecodable: text.contains('\u{FFFD}'),
        },
        RawEntryName::Bytes(bytes) => decode_bytes(bytes, encoding),
    }
}

fn decode_bytes(bytes: &[u8], encoding: NameEncoding) -> DecodedName {
    if let Ok(text) = std::str::from_utf8(bytes) {
        return DecodedName {
            text: text.to_owned(),
            legacy: false,
            undecodable: false,
        };
    }
    let decoded = legacy_encoding(encoding)
        .map(|codec| codec.decode_without_bom_handling_and_without_replacement(bytes));
    match decoded {
        Some(Some(text)) => DecodedName {
            text: text.into_owned(),
            legacy: true,
            undecodable: false,
        },
        Some(None) => DecodedName {
            text: cp437::decode(bytes),
            legacy: true,
            undecodable: true,
        },
        None => DecodedName {
            text: cp437::decode(bytes),
            legacy: true,
            undecodable: false,
        },
    }
}

fn legacy_encoding(encoding: NameEncoding) -> Option<&'static Encoding> {
    match encoding {
        NameEncoding::Auto => None,
        NameEncoding::ShiftJis => Some(encoding_rs::SHIFT_JIS),
        NameEncoding::Gbk => Some(encoding_rs::GBK),
        NameEncoding::Big5 => Some(encoding_rs::BIG5),
        NameEncoding::EucKr => Some(encoding_rs::EUC_KR),
        NameEncoding::Ibm866 => Some(encoding_rs::IBM866),
        NameEncoding::Windows1251 => Some(encoding_rs::WINDOWS_1251),
    }
}

pub fn normalize(listing: &Listing, encoding: NameEncoding, limits: &Limits) -> Vec<Entry> {
    listing
        .entries
        .iter()
        .map(|raw| normalize_entry(raw, listing.format, encoding, limits))
        .collect()
}

fn normalize_entry(
    raw: &RawEntry,
    format: ArchiveFormat,
    encoding: NameEncoding,
    limits: &Limits,
) -> Entry {
    let decoded = decode_name(&raw.name, encoding);
    let text: String = decoded.text.replace('\\', "/").nfc().collect();
    let mut kind = raw.kind;
    if format == ArchiveFormat::Zip
        && kind == EntryKind::File
        && raw.size == 0
        && text.ends_with('/')
    {
        kind = EntryKind::Dir;
    }
    let (path, problem) = match split_checked(&text, limits) {
        Ok(path) => (path, None),
        Err(problem) => (Vec::new(), Some(problem)),
    };
    Entry {
        index: raw.index,
        path,
        display: decoded.text,
        kind,
        size: raw.size,
        crc32: raw.crc32,
        mtime: raw.mtime,
        legacy: decoded.legacy,
        undecodable: decoded.undecodable,
        problem,
    }
}

/// `text` uses `/` only and is NFC; the checks run in the order of spec §4.4.
fn split_checked(text: &str, limits: &Limits) -> Result<Vec<String>, EntryProblem> {
    if text.contains('\0') {
        return Err(EntryProblem::Nul);
    }
    if text.starts_with("//") {
        return Err(EntryProblem::Unc);
    }
    if text.starts_with('/') {
        return Err(EntryProblem::Absolute);
    }
    let parts: Vec<&str> = text.split('/').collect();
    if parts.first().is_some_and(|first| has_drive_prefix(first)) {
        return Err(EntryProblem::Drive);
    }
    if parts.contains(&"..") {
        return Err(EntryProblem::Traversal);
    }
    if parts.iter().any(|part| is_dots_only(part)) {
        return Err(EntryProblem::DotsOnly);
    }
    let kept: Vec<String> = parts
        .into_iter()
        .filter(|part| !part.is_empty() && *part != ".")
        .map(str::to_owned)
        .collect();
    if kept.is_empty() {
        return Err(EntryProblem::Empty);
    }
    if kept.len() > usize::try_from(limits.max_depth).unwrap_or(usize::MAX) {
        return Err(EntryProblem::TooDeep);
    }
    let bytes = kept.iter().map(String::len).sum::<usize>() + kept.len() - 1;
    if bytes > usize::try_from(limits.max_path_bytes).unwrap_or(usize::MAX) {
        return Err(EntryProblem::TooLong);
    }
    Ok(kept)
}

fn has_drive_prefix(part: &str) -> bool {
    let bytes = part.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

/// Win32 strips trailing dots and spaces, so `...` or `. .` would collapse into another name.
fn is_dots_only(part: &str) -> bool {
    !part.is_empty() && part != "." && part.chars().all(|c| c == '.' || c == ' ')
}
