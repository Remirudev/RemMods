use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use super::FsError;

pub const SMOKE_ROOT_PREFIX: &str = "remmods-smoke-";

pub fn ensure_dir(path: &Path) -> Result<(), FsError> {
    fs::create_dir_all(path).map_err(|source| FsError::io("create_dir", path, source))
}

/// Creates a fresh `<temp>/remmods-smoke-<pid>-<millis>` folder for a `--smoke` run.
pub fn create_smoke_root() -> Result<PathBuf, FsError> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis());
    let name = format!("{SMOKE_ROOT_PREFIX}{}-{millis}", process::id());
    let root = std::env::temp_dir().join(name);
    fs::create_dir(&root).map_err(|source| FsError::io("create_dir", &root, source))?;
    Ok(root)
}

/// Deletes a smoke root recursively. Refuses any path that is not a direct child of the temp
/// folder named `remmods-smoke-*`, so a bad argument can never wipe user data.
pub fn remove_smoke_root(root: &Path) -> Result<(), FsError> {
    let in_temp = root.parent() == Some(std::env::temp_dir().as_path());
    let named_like_smoke = root
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(SMOKE_ROOT_PREFIX));
    if !(in_temp && named_like_smoke) {
        let refused = io::Error::new(ErrorKind::InvalidInput, "not a smoke root");
        return Err(FsError::io("remove_smoke_root", root, refused));
    }
    fs::remove_dir_all(root).map_err(|source| FsError::io("remove_dir_all", root, source))
}
