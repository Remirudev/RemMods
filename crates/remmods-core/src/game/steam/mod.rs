//! Steam's own files, read-only: VDF reader, library list, app manifest, root candidates.

mod library;
mod manifest;
mod roots;
mod vdf;

pub use library::{LibraryFileError, parse_library_folders};
pub use manifest::{AppManifest, ManifestError, parse_app_manifest};
pub use roots::{
    flatpak_library_path, linux_root_candidates, steam_kind_of, windows_root_candidates,
};
pub use vdf::{
    VDF_MAX_BYTES, VDF_MAX_DEPTH, Vdf, VdfError, VdfErrorKind, parse_vdf, parse_vdf_bytes,
};

use std::io::{self, Read};
use std::path::Path;

use crate::platform;

/// Reads a Steam file through `platform::fs_path`, at most one byte past `VDF_MAX_BYTES`, so an
/// oversized file is reported as `TooLarge` by the parser without being read whole.
pub(crate) fn read_steam_file(path: &Path) -> io::Result<Vec<u8>> {
    let file = std::fs::File::open(platform::fs_path(path)?)?;
    let mut bytes = Vec::new();
    file.take(VDF_MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    Ok(bytes)
}
