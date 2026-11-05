//! The owner's own picture of a mod: an image file kept as it is, in the assets folder, and linked
//! from `mod-meta.json`. Only the webview decodes it, inside an `<img>`.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use super::model::PictureProblem;
use crate::platform;

/// Plenty for a screenshot; also what one IPC call may carry as a `data:` URL.
const MAX_OWN_PICTURE_BYTES: u64 = 16 * 1024 * 1024;
const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";
const JPEG_MAGIC: &[u8] = b"\xff\xd8\xff";
const RIFF_MAGIC: &[u8] = b"RIFF";
const WEBP_MAGIC: &[u8] = b"WEBP";
const WEBP_MAGIC_AT: usize = 8;
pub(crate) const OWN_ID_PREFIX: &str = "own-";
const OWN_ID_HEX_LEN: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OwnFormat {
    Png,
    Jpeg,
    Webp,
}

impl OwnFormat {
    /// By content: a renamed file is no image.
    fn sniff(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(PNG_MAGIC) {
            return Some(Self::Png);
        }
        if bytes.starts_with(JPEG_MAGIC) {
            return Some(Self::Jpeg);
        }
        let webp = bytes.starts_with(RIFF_MAGIC)
            && bytes
                .get(WEBP_MAGIC_AT..WEBP_MAGIC_AT + WEBP_MAGIC.len())
                .is_some_and(|mark| mark == WEBP_MAGIC);
        webp.then_some(Self::Webp)
    }

    pub(crate) fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Webp => "webp",
        }
    }

    pub(crate) fn mime(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Webp => "image/webp",
        }
    }

    fn from_extension(extension: &str) -> Option<Self> {
        [Self::Png, Self::Jpeg, Self::Webp]
            .into_iter()
            .find(|format| format.extension() == extension)
    }
}

/// The format of an own picture's id, `own-<32 hex>.<extension>`: ids reach the store from the UI,
/// and they become file names.
pub(crate) fn own_format(id: &str) -> Option<OwnFormat> {
    let (stem, extension) = id.strip_prefix(OWN_ID_PREFIX)?.split_once('.')?;
    let hex = stem.len() == OWN_ID_HEX_LEN
        && stem
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    hex.then(|| OwnFormat::from_extension(extension)).flatten()
}

pub(crate) fn read_own_picture(source: &Path) -> Result<(OwnFormat, Vec<u8>), PictureProblem> {
    let mut bytes = Vec::new();
    platform::fs_path(source)
        .and_then(File::open)
        .and_then(|file| file.take(MAX_OWN_PICTURE_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|error| {
            log::info!("assets: own picture not read: {error}");
            PictureProblem::Unreadable
        })?;
    if u64::try_from(bytes.len()).is_ok_and(|len| len > MAX_OWN_PICTURE_BYTES) {
        return Err(PictureProblem::TooLarge);
    }
    let format = OwnFormat::sniff(&bytes).ok_or(PictureProblem::NotAnImage)?;
    Ok((format, bytes))
}
