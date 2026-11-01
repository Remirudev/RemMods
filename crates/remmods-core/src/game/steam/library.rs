//! `libraryfolders.vdf`: the Steam library folders, old (`"1" "D:\\Steam"`) and new
//! (`"0" { "path" … }`) format. The `apps` maps are never used.

use super::vdf::{Vdf, VdfError, parse_vdf_bytes};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryFileError {
    Vdf(VdfError),
    WrongRoot,
}

impl LibraryFileError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Vdf(error) => error.kind.code(),
            Self::WrongRoot => "wrongRoot",
        }
    }

    pub fn line(&self) -> Option<u32> {
        match self {
            Self::Vdf(error) => Some(error.line),
            Self::WrongRoot => None,
        }
    }
}

/// The listed library paths in file order: entries with an all-digit key; a string value is the
/// path (old format), an object carries it in `path`. Unmounted (`"mounted" "0"`) and empty
/// paths are skipped.
pub fn parse_library_folders(bytes: &[u8]) -> Result<Vec<String>, LibraryFileError> {
    let (root, tree) = parse_vdf_bytes(bytes).map_err(LibraryFileError::Vdf)?;
    if !root.eq_ignore_ascii_case("libraryfolders") {
        return Err(LibraryFileError::WrongRoot);
    }
    let paths = tree
        .entries()
        .iter()
        .filter(|(key, _)| is_library_index(key))
        .filter_map(|(_, value)| library_path(value))
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect();
    Ok(paths)
}

fn is_library_index(key: &str) -> bool {
    !key.is_empty() && key.bytes().all(|byte| byte.is_ascii_digit())
}

fn library_path(value: &Vdf) -> Option<&str> {
    match value {
        Vdf::Str(path) => Some(path),
        Vdf::Obj(_) if value.str("mounted") == Some("0") => None,
        Vdf::Obj(_) => value.str("path"),
    }
}
