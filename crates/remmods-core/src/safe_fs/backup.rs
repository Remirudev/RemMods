use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use super::FsError;

const MAX_NAME_ATTEMPTS: u32 = 100;

/// Copies `path` to `<name>.backup.<yyyyMMdd_HHmmss>` (local time) next to it and returns the
/// copy's path. Never overwrites: a taken name gets a `-2`, `-3`, … suffix.
pub fn backup_file(path: &Path) -> Result<PathBuf, FsError> {
    let file_name = path
        .file_name()
        .ok_or_else(|| FsError::io("backup", path, io::Error::from(ErrorKind::InvalidInput)))?;
    let stamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let mut source = File::open(path).map_err(|source| FsError::io("backup", path, source))?;

    for attempt in 1..=MAX_NAME_ATTEMPTS {
        let target_path = path.with_file_name(backup_name(file_name, &stamp, attempt));
        let mut target = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target_path)
        {
            Ok(target) => target,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(FsError::io("backup", &target_path, error)),
        };
        if let Err(error) = copy_and_sync(&mut source, &mut target) {
            drop(target);
            let _ = fs::remove_file(&target_path);
            return Err(FsError::io("backup", &target_path, error));
        }
        return Ok(target_path);
    }
    Err(FsError::io(
        "backup",
        path,
        io::Error::from(ErrorKind::AlreadyExists),
    ))
}

fn backup_name(file_name: &std::ffi::OsStr, stamp: &str, attempt: u32) -> OsString {
    let mut name = file_name.to_os_string();
    name.push(".backup.");
    name.push(stamp);
    if attempt > 1 {
        name.push(format!("-{attempt}"));
    }
    name
}

fn copy_and_sync(source: &mut File, target: &mut File) -> io::Result<()> {
    io::copy(source, target)?;
    target.sync_all()
}
