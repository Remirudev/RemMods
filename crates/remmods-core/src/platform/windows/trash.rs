//! P3a has no Recycle Bin mover on Windows (WG-5): every probe reports NotSupported.

use std::fs;
use std::io;
use std::path::Path;

use super::super::{
    DirTraits, MountId, TopdirPolicy, TrashEnv, TrashLocation, TrashUnavailableReason, fs_path,
};

pub(in crate::platform) fn current_uid() -> Option<u32> {
    None
}

pub(in crate::platform) fn trash_env_from_env() -> TrashEnv {
    TrashEnv {
        home_trash: None,
        uid: None,
        topdirs: TopdirPolicy::Deny,
    }
}

pub(in crate::platform) fn trash_location(
    _env: &TrashEnv,
    _item: &Path,
) -> io::Result<Result<TrashLocation, TrashUnavailableReason>> {
    Ok(Err(TrashUnavailableReason::NotSupported))
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
