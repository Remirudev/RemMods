//! Which trash an item belongs to on Windows: its volume's Recycle Bin, reached only through
//! `recycle` and only with `TopdirPolicy::Detect` (the shell's env). Read-only.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use windows_sys::Win32::Storage::FileSystem::GetVolumePathNameW;

use super::super::{
    DirTraits, MountId, TopdirPolicy, TrashEnv, TrashKind, TrashLocation, TrashUnavailableReason,
    fs_path,
};

pub(in crate::platform) fn current_uid() -> Option<u32> {
    None
}

pub(in crate::platform) fn trash_env_from_env() -> TrashEnv {
    TrashEnv {
        home_trash: None,
        uid: None,
        topdirs: TopdirPolicy::Detect,
    }
}

/// No drive-type or bin-size probe: the shell decides, and the recycle sink makes sure nothing is
/// deleted when it cannot recycle. `Deny`/`Fixed` (tests, `--smoke`) never reach the bin.
pub(in crate::platform) fn trash_location(
    env: &TrashEnv,
    item: &Path,
) -> io::Result<Result<TrashLocation, TrashUnavailableReason>> {
    if env.topdirs != TopdirPolicy::Detect {
        return Ok(Err(TrashUnavailableReason::NotSupported));
    }
    Ok(Ok(TrashLocation {
        kind: TrashKind::RecycleBin,
        root: volume_root(item).unwrap_or_default(),
        topdir: None,
        missing: Vec::new(),
    }))
}

pub(in crate::platform) fn mount_identity(_path: &Path) -> io::Result<MountId> {
    Err(io::ErrorKind::Unsupported.into())
}

/// std reports junctions and symlinks alike as links here.
pub(in crate::platform) fn dir_traits(path: &Path) -> io::Result<DirTraits> {
    let metadata = fs::symlink_metadata(fs_path(path)?)?;
    Ok(DirTraits {
        owner: None,
        is_dir: metadata.is_dir(),
        is_symlink: metadata.file_type().is_symlink(),
        sticky: false,
    })
}

/// The volume root of `item` (GetVolumePathNameW), for display and logs only.
fn volume_root(item: &Path) -> Option<PathBuf> {
    let input: Vec<u16> = fs_path(item)
        .ok()?
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut buffer = vec![0_u16; input.len() + 2];
    let capacity = u32::try_from(buffer.len()).ok()?;
    // SAFETY: `input` is NUL-terminated and `buffer` has `capacity` writable units.
    let ok = unsafe { GetVolumePathNameW(input.as_ptr(), buffer.as_mut_ptr(), capacity) };
    if ok == 0 {
        log::debug!(
            "GetVolumePathNameW failed for {}: {}",
            item.display(),
            io::Error::last_os_error()
        );
        return None;
    }
    let len = buffer.iter().position(|&unit| unit == 0)?;
    let root = PathBuf::from(OsString::from_wide(&buffer[..len]));
    Some(dunce::simplified(&root).to_path_buf())
}
