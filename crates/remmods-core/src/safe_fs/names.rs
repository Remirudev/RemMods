//! Name rules (PLAN §4 Names): validation of new names, the Win32 and portable checks for names
//! already on disk, caseless keys and the lossless key text of raw OS names.

use std::ffi::OsStr;
use std::fmt::Write as _;
use std::path::Path;
use std::str::Chars;

use unicase::UniCase;
use unicode_normalization::UnicodeNormalization;

use crate::platform::{self, RawName};

pub const NAME_MAX_BYTES: usize = 255;
/// N3a: `.disabled_` + name must still fit 255 bytes.
pub const LOGIC_NAME_MAX_BYTES: usize = 245;
/// N3b: name + `-x` must still fit 255 bytes.
pub const PAYLOAD_NAME_MAX_BYTES: usize = 253;
/// NTFS component limit in UTF-16 units.
pub const WINDOWS_NAME_MAX_UNITS: usize = 255;
pub const DISABLED_PREFIX: &str = ".disabled_";
/// MAX_PATH 260 minus the NUL.
pub const WINDOWS_PATH_MAX_FILE: usize = 259;
/// MAX_PATH minus room for an 8.3 name minus the NUL.
pub const WINDOWS_PATH_MAX_DIR: usize = 247;

const FORBIDDEN_CHARS: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
const PLAIN_DEVICE_NAMES: [&str; 6] = ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"];
const PORT_DEVICE_NAMES: [&str; 2] = ["COM", "LPT"];
const TEMP_MARKER: &[u8] = b"_tmp_";
const VERBATIM: &str = r"\\?\";
const VERBATIM_UNC: &str = r"\\?\UNC\";
const TEMP_SUFFIX_LEN: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NameIssue {
    Empty,
    DotName,
    TooLong,
    ForbiddenChar(char),
    ControlChar(char),
    BidiControl(char),
    TrailingDotOrSpace,
    ReservedDeviceName,
    DisabledPrefix,
    TempName,
}

impl NameIssue {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::DotName => "dot_name",
            Self::TooLong => "too_long",
            Self::ForbiddenChar(_) => "forbidden_char",
            Self::ControlChar(_) => "control_char",
            Self::BidiControl(_) => "bidi_control",
            Self::TrailingDotOrSpace => "trailing_dot_or_space",
            Self::ReservedDeviceName => "reserved_device_name",
            Self::DisabledPrefix => "disabled_prefix",
            Self::TempName => "temp_name",
        }
    }
}

/// Which raw form an escaped key text decodes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyForm {
    Bytes,
    Wide,
}

/// Rules N1–N10 for a name RemMods would create; the first hit.
pub fn validate_name(name: &str) -> Result<(), NameIssue> {
    match name_issues(name).first() {
        Some(issue) => Err(*issue),
        None => Ok(()),
    }
}

/// Every hit of N1–N10, in rule order. Empty and dot names stop the check.
pub fn name_issues(name: &str) -> Vec<NameIssue> {
    if name.is_empty() {
        return vec![NameIssue::Empty];
    }
    if is_dot_name(name) {
        return vec![NameIssue::DotName];
    }
    [
        (name.len() > NAME_MAX_BYTES).then_some(NameIssue::TooLong),
        first_forbidden_char(name).map(NameIssue::ForbiddenChar),
        name.chars()
            .find(|c| c.is_control())
            .map(NameIssue::ControlChar),
        first_bidi_control(name).map(NameIssue::BidiControl),
        has_trailing_dot_or_space(name).then_some(NameIssue::TrailingDotOrSpace),
        is_reserved_device_name(name).then_some(NameIssue::ReservedDeviceName),
        has_disabled_prefix_folded(name).then_some(NameIssue::DisabledPrefix),
        is_install_temp_name(OsStr::new(name)).then_some(NameIssue::TempName),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// A name already on disk that Win32 itself rejects or rewrites; the first hit.
pub fn windows_name_issue(name: &str) -> Option<NameIssue> {
    if name.is_empty() {
        return Some(NameIssue::Empty);
    }
    if is_dot_name(name) {
        return Some(NameIssue::DotName);
    }
    if utf16_len(name) > WINDOWS_NAME_MAX_UNITS {
        return Some(NameIssue::TooLong);
    }
    if let Some(c) = first_forbidden_char(name) {
        return Some(NameIssue::ForbiddenChar(c));
    }
    if let Some(c) = name.chars().find(|c| ('\0'..='\u{1F}').contains(c)) {
        return Some(NameIssue::ControlChar(c));
    }
    if has_trailing_dot_or_space(name) {
        return Some(NameIssue::TrailingDotOrSpace);
    }
    is_reserved_device_name(name).then_some(NameIssue::ReservedDeviceName)
}

/// True when the shell would get exactly this path: valid Unicode; a leading `\\?\` or
/// `\\?\UNC\` is read as the plain form (`\\?\` must then be followed by a drive); components
/// split on `\` and `/`; a leading `X:` drive or `\\server\share` (both names Win32 keeps) and an
/// empty root component are allowed; every other component passes [`windows_name_issue`] (so no
/// `.`/`..`, forbidden or control characters, trailing dot or space, device names incl.
/// CONIN$/CONOUT$ and superscript COM/LPT); the plain form is ≤ 259 UTF-16 units.
pub fn shell_safe_path(path: &Path) -> bool {
    let Some(text) = path.to_str() else {
        return false;
    };
    let plain = if let Some(rest) = text.strip_prefix(VERBATIM_UNC) {
        format!(r"\\{rest}")
    } else if let Some(rest) = text.strip_prefix(VERBATIM) {
        if !rest.split(['\\', '/']).next().is_some_and(is_drive) {
            return false;
        }
        rest.to_owned()
    } else {
        text.to_owned()
    };
    if utf16_len(&plain) > WINDOWS_PATH_MAX_FILE {
        return false;
    }
    let mut components = plain.split(['\\', '/']).peekable();
    let is_unc = plain.starts_with(r"\\") || plain.starts_with("//");
    if is_unc {
        components.next();
        components.next();
        let server = components.next().unwrap_or_default();
        let share = components.next().unwrap_or_default();
        if windows_name_issue(server).is_some() || windows_name_issue(share).is_some() {
            return false;
        }
    } else if components
        .peek()
        .is_some_and(|first| first.is_empty() || is_drive(first))
    {
        components.next();
    }
    components.all(|component| windows_name_issue(component).is_none())
}

/// A name already on disk that Windows accepts but RemMods would not create; the first hit.
/// `max_bytes` is 255 or a role budget (`LOGIC_NAME_MAX_BYTES`, `PAYLOAD_NAME_MAX_BYTES`).
pub fn portable_name_issue(name: &str, max_bytes: usize) -> Option<NameIssue> {
    if name.len() > max_bytes {
        return Some(NameIssue::TooLong);
    }
    if let Some(c) = name.chars().find(|c| ('\u{7F}'..='\u{9F}').contains(c)) {
        return Some(NameIssue::ControlChar(c));
    }
    first_bidi_control(name).map(NameIssue::BidiControl)
}

/// Caseless key: NFC(full case fold(NFD(s))) (Unicode D145). Never stored on disk.
pub fn fold_key(s: &str) -> String {
    if s.is_ascii() {
        return s.to_ascii_lowercase();
    }
    let decomposed: String = s.nfd().collect();
    UniCase::unicode(decomposed.as_str())
        .to_folded_case()
        .nfc()
        .collect()
}

/// Total order over key texts: caseless first, then bytes (key text is injective).
pub fn order_key(key_text: &str) -> (String, Vec<u8>) {
    (fold_key(key_text), key_text.as_bytes().to_vec())
}

/// Lossless text form of an OS name: the name itself when it is Unicode without `\`, else the
/// escaped raw units.
pub fn key_text(name: &OsStr) -> String {
    match name.to_str() {
        Some(text) if !text.contains('\\') => text.to_owned(),
        _ => match platform::raw_name(name) {
            RawName::Bytes(bytes) => escape_bytes(&bytes),
            RawName::Wide(units) => escape_wide(&units),
        },
    }
}

/// `\` → `\\`, each byte that is not part of valid UTF-8 → `\xHH`, everything else as is.
pub fn escape_bytes(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for chunk in bytes.utf8_chunks() {
        push_escaped_text(&mut out, chunk.valid());
        for byte in chunk.invalid() {
            let _ = write!(out, "\\x{byte:02X}");
        }
    }
    out
}

/// `\` → `\\`, each unpaired surrogate → `\u{HHHH}`, everything else as is.
pub fn escape_wide(units: &[u16]) -> String {
    let mut out = String::with_capacity(units.len());
    for decoded in char::decode_utf16(units.iter().copied()) {
        match decoded {
            Ok('\\') => out.push_str("\\\\"),
            Ok(c) => out.push(c),
            Err(error) => {
                let _ = write!(out, "\\u{{{:04X}}}", error.unpaired_surrogate());
            }
        }
    }
    out
}

/// Inverse of [`escape_bytes`] / [`escape_wide`]: a key without `\` is literal; anything the
/// escape functions would not produce is `None`.
pub fn unescape_key(key: &str, form: KeyForm) -> Option<RawName> {
    if !key.contains('\\') {
        return Some(match form {
            KeyForm::Bytes => RawName::Bytes(key.as_bytes().to_vec()),
            KeyForm::Wide => RawName::Wide(key.encode_utf16().collect()),
        });
    }
    let (raw, canonical) = match form {
        KeyForm::Bytes => {
            let bytes = unescape_bytes(key)?;
            let canonical = escape_bytes(&bytes);
            (RawName::Bytes(bytes), canonical)
        }
        KeyForm::Wide => {
            let units = unescape_wide(key)?;
            let canonical = escape_wide(&units);
            (RawName::Wide(units), canonical)
        }
    };
    (canonical == key).then_some(raw)
}

/// Display text: control and bidi control characters become `\u{HHHH}`, so a name cannot hide
/// text or reorder what the UI shows.
pub fn display_escaped(s: &str) -> String {
    if !s.chars().any(needs_display_escape) {
        return s.to_owned();
    }
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        if needs_display_escape(c) {
            let _ = write!(out, "\\u{{{:04X}}}", u32::from(c));
        } else {
            out.push(c);
        }
    }
    out
}

/// Unicode Bidi_Control (12 code points).
pub fn is_bidi_control(c: char) -> bool {
    matches!(
        c,
        '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
    )
}

/// Win32 device names: the stem before the first `.`, trailing spaces removed, compared
/// ASCII-case-insensitively; COM/LPT need exactly one digit 1–9 or ¹²³.
pub fn is_reserved_device_name(name: &str) -> bool {
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches(' ');
    if PLAIN_DEVICE_NAMES
        .iter()
        .any(|device| stem.eq_ignore_ascii_case(device))
    {
        return true;
    }
    let Some((prefix, rest)) = stem.split_at_checked(3) else {
        return false;
    };
    let mut digits = rest.chars();
    PORT_DEVICE_NAMES
        .iter()
        .any(|port| prefix.eq_ignore_ascii_case(port))
        && matches!(
            (digits.next(), digits.next()),
            (Some('1'..='9' | '\u{B9}' | '\u{B2}' | '\u{B3}'), None)
        )
}

/// The original's IsInstallTempFolder: `_`, then `_tmp_` (case-sensitive), and exactly 32 hex
/// digits after the last `_tmp_`.
pub fn is_install_temp_name(name: &OsStr) -> bool {
    let bytes = name.as_encoded_bytes();
    if bytes.first() != Some(&b'_') {
        return false;
    }
    let Some(marker) = bytes
        .windows(TEMP_MARKER.len())
        .rposition(|window| window == TEMP_MARKER)
    else {
        return false;
    };
    bytes
        .get(marker + TEMP_MARKER.len()..)
        .is_some_and(|suffix| {
            suffix.len() == TEMP_SUFFIX_LEN && suffix.iter().all(u8::is_ascii_hexdigit)
        })
}

/// The original's scan rule: the first 10 bytes equal `.disabled_` ignoring ASCII case.
pub fn has_disabled_prefix_ascii(name: &OsStr) -> bool {
    name.as_encoded_bytes()
        .get(..DISABLED_PREFIX.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(DISABLED_PREFIX.as_bytes()))
}

/// N9: the caseless key starts with `.disabled_` (also `.DISABLED_`, `.diſabled_`).
pub fn has_disabled_prefix_folded(name: &str) -> bool {
    fold_key(name).starts_with(DISABLED_PREFIX)
}

pub fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

fn is_drive(component: &str) -> bool {
    let bytes = component.as_bytes();
    bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn is_dot_name(name: &str) -> bool {
    name == "." || name == ".."
}

fn first_forbidden_char(name: &str) -> Option<char> {
    name.chars().find(|c| FORBIDDEN_CHARS.contains(c))
}

fn first_bidi_control(name: &str) -> Option<char> {
    name.chars().find(|&c| is_bidi_control(c))
}

fn has_trailing_dot_or_space(name: &str) -> bool {
    name.ends_with('.') || name.ends_with(' ')
}

fn needs_display_escape(c: char) -> bool {
    c.is_control() || is_bidi_control(c)
}

fn push_escaped_text(out: &mut String, text: &str) {
    for c in text.chars() {
        if c == '\\' {
            out.push_str("\\\\");
        } else {
            out.push(c);
        }
    }
}

fn unescape_bytes(key: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(key.len());
    let mut chars = key.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            let mut buffer = [0; 4];
            out.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
            continue;
        }
        match chars.next()? {
            '\\' => out.push(b'\\'),
            'x' => out.push(u8::try_from(take_hex(&mut chars, 2)?).ok()?),
            _ => return None,
        }
    }
    Some(out)
}

fn unescape_wide(key: &str) -> Option<Vec<u16>> {
    let mut out = Vec::with_capacity(key.len());
    let mut chars = key.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            let mut buffer = [0; 2];
            out.extend_from_slice(c.encode_utf16(&mut buffer));
            continue;
        }
        match chars.next()? {
            '\\' => out.push(u16::from(b'\\')),
            'u' => {
                if chars.next()? != '{' {
                    return None;
                }
                let unit = u16::try_from(take_hex(&mut chars, 4)?).ok()?;
                if chars.next()? != '}' {
                    return None;
                }
                out.push(unit);
            }
            _ => return None,
        }
    }
    Some(out)
}

fn take_hex(chars: &mut Chars<'_>, digits: usize) -> Option<u32> {
    let mut value = 0;
    for _ in 0..digits {
        value = value * 16 + chars.next()?.to_digit(16)?;
    }
    Some(value)
}
