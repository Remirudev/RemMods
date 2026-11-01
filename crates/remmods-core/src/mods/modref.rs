//! Session-only row tokens: `1|kind|fileid|paksid|path` (spec §4.7). Never stored on disk.

use std::fmt;
use std::path::Path;

use super::error::ModsError;
use crate::platform::{self, FileId};

const VERSION: &str = "1";
const NO_FILE_ID: &str = "-";
const FIELDS: usize = 5;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModRef(String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedRef {
    pub kind: RefKind,
    pub file_id: Option<String>,
    pub paks_id: String,
    /// Key texts relative to Paks.
    pub components: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefKind {
    Dir,
    File,
    Loose,
}

impl ModRef {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(token: &str) -> Result<ParsedRef, ModsError> {
        let fields: Vec<&str> = token.splitn(FIELDS, '|').collect();
        let [version, kind, file_id, paks_id, path] = fields[..] else {
            return Err(ModsError::BadRef);
        };
        if version != VERSION || paks_id.is_empty() {
            return Err(ModsError::BadRef);
        }
        let kind = RefKind::from_code(kind).ok_or(ModsError::BadRef)?;
        let file_id = parse_file_id(file_id)?;
        let components: Vec<String> = path.split('/').map(str::to_owned).collect();
        let min_components = if kind == RefKind::Loose { 2 } else { 1 };
        if components.len() < min_components || components.iter().any(String::is_empty) {
            return Err(ModsError::BadRef);
        }
        Ok(ParsedRef {
            kind,
            file_id,
            paks_id: paks_id.to_owned(),
            components,
        })
    }

    /// Wraps a token unchanged and unchecked (UI input, test literals); `parse` and `resolve_ref`
    /// still validate it.
    pub fn from_token(token: impl Into<String>) -> ModRef {
        Self(token.into())
    }

    /// The scan's own formatter.
    pub(crate) fn build(
        kind: RefKind,
        file_id: Option<FileId>,
        paks_id: &str,
        components: &[&str],
    ) -> ModRef {
        let file_id = file_id.map_or_else(|| NO_FILE_ID.to_owned(), format_file_id);
        Self(format!(
            "{VERSION}|{}|{file_id}|{paks_id}|{}",
            kind.code(),
            components.join("/")
        ))
    }
}

impl fmt::Display for ModRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl RefKind {
    fn code(self) -> &'static str {
        match self {
            Self::Dir => "d",
            Self::File => "f",
            Self::Loose => "l",
        }
    }

    fn from_code(code: &str) -> Option<Self> {
        match code {
            "d" => Some(Self::Dir),
            "f" => Some(Self::File),
            "l" => Some(Self::Loose),
            _ => None,
        }
    }
}

/// `platform::file_id(paks)` as `"{volume:x}-{index:x}"`; when that fails, `"g{generation}"`.
pub fn paks_identity(paks: &Path, generation: u32) -> String {
    match platform::fs_path(paks).and_then(|path| platform::file_id(&path)) {
        Ok(id) => format_file_id(id),
        Err(error) => {
            log::debug!("no file id for {}: {error}", paks.display());
            format!("g{generation}")
        }
    }
}

pub(crate) fn format_file_id(id: FileId) -> String {
    format!("{:x}-{:x}", id.volume, id.index)
}

fn parse_file_id(text: &str) -> Result<Option<String>, ModsError> {
    if text == NO_FILE_ID {
        return Ok(None);
    }
    let is_hex = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_hexdigit());
    match text.split_once('-') {
        Some((volume, index)) if is_hex(volume) && is_hex(index) => Ok(Some(text.to_owned())),
        _ => Err(ModsError::BadRef),
    }
}
