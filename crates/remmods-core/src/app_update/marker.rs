use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use super::InstallSource;
use crate::platform::{self, OsKind};

pub const INSTALL_SOURCE_FILE: &str = "install-source";
pub const MARKER_MAX_BYTES: u64 = 256;

const AUR_MARKER: &[u8] = b"install-source=aur";
const LOCAL_MARKER: &[u8] = b"install-source=local";

/// `<prefix>/bin/remmods` → `<prefix>/lib/remmods/install-source`; `None` when the executable is
/// not in a `bin` folder.
pub fn marker_path(exe: &Path) -> Option<PathBuf> {
    let bin = exe.parent()?;
    let name = bin.file_name()?.to_str()?;
    let is_bin = match platform::os_kind() {
        OsKind::Windows => name.eq_ignore_ascii_case("bin"),
        OsKind::Linux | OsKind::Other => name == "bin",
    };
    if !is_bin {
        return None;
    }
    let prefix = bin.parent()?;
    Some(prefix.join("lib").join("remmods").join(INSTALL_SOURCE_FILE))
}

/// Exactly `install-source=aur` or `install-source=local`, optionally followed by one `\n`.
pub fn parse_marker(bytes: &[u8]) -> Option<InstallSource> {
    match bytes.strip_suffix(b"\n").unwrap_or(bytes) {
        AUR_MARKER => Some(InstallSource::Aur),
        LOCAL_MARKER => Some(InstallSource::Local),
        _ => None,
    }
}

/// Read-only; `None` unless the marker is a regular file of at most 256 bytes that parses.
pub fn read_marker(exe: &Path) -> Option<InstallSource> {
    let path = marker_path(exe)?;
    match read_small_file(&path) {
        Ok(Some(bytes)) => parse_marker(&bytes),
        Ok(None) => None,
        Err(error) => {
            log::debug!("install-source marker {} not read: {error}", path.display());
            None
        }
    }
}

fn read_small_file(path: &Path) -> std::io::Result<Option<Vec<u8>>> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() || metadata.len() > MARKER_MAX_BYTES {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(MARKER_MAX_BYTES + 1)
        .read_to_end(&mut bytes)?;
    let within = u64::try_from(bytes.len()).is_ok_and(|len| len <= MARKER_MAX_BYTES);
    Ok(within.then_some(bytes))
}
