//! File-name classification: payload files, archives and the game's own Paks files (spec §4.6).

use std::ffi::OsStr;

use super::model::PayloadKind;

const OFF_SUFFIX: &[u8] = b"-x";
const PAYLOAD_EXTENSIONS: [(&[u8], PayloadKind); 4] = [
    (b".pak", PayloadKind::Pak),
    (b".ucas", PayloadKind::Ucas),
    (b".utoc", PayloadKind::Utoc),
    (b".sig", PayloadKind::Sig),
];
const ARCHIVE_EXTENSIONS: [&[u8]; 3] = [b".zip", b".7z", b".rar"];
const GAME_CHUNK_PREFIX: &[u8] = b"pakchunk";
const GAME_CHUNK_MARKER: &[u8] = b"-windows";
const GAME_GLOBAL_FILES: [&[u8]; 2] = [b"global.ucas", b"global.utoc"];

/// A payload file name: `.pak/.ucas/.utoc/.sig`, live or switched off with `-x`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Payload {
    pub kind: PayloadKind,
    pub live: bool,
    /// Bytes of the suffix (`.pak`, `.pak-x`, …), so the stem is the name without them.
    pub suffix_len: usize,
}

impl Payload {
    pub(crate) fn is_pak(self) -> bool {
        self.kind == PayloadKind::Pak
    }

    /// `.pak/.ucas/.utoc`: the files the game loads (`.sig` only signs them).
    pub(crate) fn is_loadable(self) -> bool {
        self.kind != PayloadKind::Sig
    }
}

pub(crate) fn classify_payload(name: &OsStr) -> Option<Payload> {
    let bytes = name.as_encoded_bytes();
    let (body, live) = match strip_suffix_ci(bytes, OFF_SUFFIX) {
        Some(body) => (body, false),
        None => (bytes, true),
    };
    PAYLOAD_EXTENSIONS
        .iter()
        .find(|(extension, _)| ends_with_ci(body, extension))
        .map(|(extension, kind)| Payload {
            kind: *kind,
            live,
            suffix_len: extension.len() + if live { 0 } else { OFF_SUFFIX.len() },
        })
}

/// The name with a trailing `-x` removed (ASCII case-insensitive); other names unchanged.
pub(crate) fn strip_off_suffix(name: &str) -> &str {
    match name.len().checked_sub(OFF_SUFFIX.len()) {
        Some(cut)
            if name.is_char_boundary(cut)
                && name.as_bytes()[cut..].eq_ignore_ascii_case(OFF_SUFFIX) =>
        {
            &name[..cut]
        }
        _ => name,
    }
}

pub(crate) fn is_archive(name: &OsStr) -> bool {
    let bytes = name.as_encoded_bytes();
    ARCHIVE_EXTENSIONS
        .iter()
        .any(|extension| ends_with_ci(bytes, extension))
}

/// The game's own files at the Paks top level (`pakchunk0-Windows.pak`, `global.utoc`, …).
pub(crate) fn is_game_file(name: &OsStr) -> bool {
    let lower = name.as_encoded_bytes().to_ascii_lowercase();
    if GAME_GLOBAL_FILES.contains(&lower.as_slice()) {
        return true;
    }
    lower.starts_with(GAME_CHUNK_PREFIX)
        && lower
            .windows(GAME_CHUNK_MARKER.len())
            .any(|window| window == GAME_CHUNK_MARKER)
        && classify_payload(name).is_some_and(|payload| payload.live)
}

fn ends_with_ci(bytes: &[u8], suffix: &[u8]) -> bool {
    strip_suffix_ci(bytes, suffix).is_some()
}

fn strip_suffix_ci<'a>(bytes: &'a [u8], suffix: &[u8]) -> Option<&'a [u8]> {
    let cut = bytes.len().checked_sub(suffix.len())?;
    bytes[cut..]
        .eq_ignore_ascii_case(suffix)
        .then(|| &bytes[..cut])
}
