//! freedesktop.org Trash 1.0: which trash an item belongs to. Read-only; safe_fs creates the
//! folders and moves the item.

use std::env;
use std::fs;
use std::io;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use rustix::fs::{AtFlags, CWD, StatxFlags, statx};
use rustix::io::Errno;

use super::super::{
    DirTraits, MountId, TopdirPolicy, TrashEnv, TrashKind, TrashLocation, TrashUnavailableReason,
};

const STICKY: u32 = 0o1000;

type Probe = Result<TrashLocation, TrashUnavailableReason>;

pub(in crate::platform) fn current_uid() -> Option<u32> {
    Some(rustix::process::getuid().as_raw())
}

pub(in crate::platform) fn trash_env_from_env() -> TrashEnv {
    let data_home = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| dirs::home_dir().map(|home| home.join(".local").join("share")));
    TrashEnv {
        home_trash: data_home.map(|data| data.join("Trash")),
        uid: current_uid(),
        topdirs: TopdirPolicy::Detect,
    }
}

pub(in crate::platform) fn mount_identity(path: &Path) -> io::Result<MountId> {
    identity(path, AtFlags::SYMLINK_NOFOLLOW)
}

pub(in crate::platform) fn dir_traits(path: &Path) -> io::Result<DirTraits> {
    let metadata = fs::symlink_metadata(path)?;
    Ok(DirTraits {
        owner: Some(metadata.uid()),
        is_dir: metadata.is_dir(),
        is_symlink: metadata.file_type().is_symlink(),
        sticky: metadata.permissions().mode() & STICKY != 0,
    })
}

/// The item's folder is resolved (links followed) first: a rename goes through links, and a
/// topdir must be the real mount root, not a lexical ancestor reached through a link.
pub(in crate::platform) fn trash_location(env: &TrashEnv, item: &Path) -> io::Result<Probe> {
    fs::symlink_metadata(item)?;
    let parent = item
        .parent()
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    let parent = fs::canonicalize(parent)?;
    let want = followed_identity(&parent)?;

    if let Some(home_trash) = &env.home_trash
        && let Some(found) = home_location(home_trash, &want)?
    {
        return Ok(found);
    }
    let topdir = match &env.topdirs {
        TopdirPolicy::Deny => return Ok(Err(TrashUnavailableReason::OtherDevice)),
        TopdirPolicy::Fixed(fixed) => {
            let fixed = fs::canonicalize(fixed)?;
            if !parent.starts_with(&fixed) {
                return Ok(Err(TrashUnavailableReason::OtherDevice));
            }
            fixed
        }
        TopdirPolicy::Detect => detect_topdir(&parent, &want)?,
    };
    let Some(uid) = env.uid else {
        return Ok(Err(TrashUnavailableReason::NoUsableTrash));
    };
    if let Some(shared) = shared_location(&topdir, uid)? {
        return Ok(Ok(shared));
    }
    user_location(&topdir, uid)
}

fn home_location(home_trash: &Path, want: &MountId) -> io::Result<Option<Probe>> {
    let Some(anchor) = home_trash.ancestors().find(|path| path.exists()) else {
        return Ok(None);
    };
    if !followed_identity(anchor)?.same_mount(want) {
        return Ok(None);
    }
    let location = TrashLocation {
        kind: TrashKind::Home,
        root: home_trash.to_path_buf(),
        topdir: None,
        missing: Vec::new(),
    };
    Ok(Some(with_missing(location, |path| {
        fs::metadata(path).map(|metadata| metadata.is_dir())
    })?))
}

fn detect_topdir(parent: &Path, want: &MountId) -> io::Result<PathBuf> {
    let mut topdir = parent.to_path_buf();
    while let Some(up) = topdir.parent() {
        if !followed_identity(up)?.same_mount(want) {
            break;
        }
        topdir = up.to_path_buf();
    }
    Ok(topdir)
}

/// `$topdir/.Trash` must be a real, sticky folder; `$topdir/.Trash/$uid` (if there) a real
/// folder owned by the user. Anything else falls through to `.Trash-$uid` (FDO).
fn shared_location(topdir: &Path, uid: u32) -> io::Result<Option<TrashLocation>> {
    let shared = topdir.join(".Trash");
    let Some(traits) = traits_if_present(&shared)? else {
        return Ok(None);
    };
    if !traits.is_dir || traits.is_symlink || !traits.sticky {
        return Ok(None);
    }
    let root = shared.join(uid.to_string());
    if let Some(traits) = traits_if_present(&root)?
        && !traits.is_private_dir_of(Some(uid))
    {
        return Ok(None);
    }
    let location = TrashLocation {
        kind: TrashKind::TopdirShared,
        root,
        topdir: Some(topdir.to_path_buf()),
        missing: Vec::new(),
    };
    Ok(with_missing(location, real_dir)?.ok())
}

fn user_location(topdir: &Path, uid: u32) -> io::Result<Probe> {
    let root = topdir.join(format!(".Trash-{uid}"));
    if let Some(traits) = traits_if_present(&root)?
        && !traits.is_private_dir_of(Some(uid))
    {
        return Ok(Err(TrashUnavailableReason::NoUsableTrash));
    }
    let location = TrashLocation {
        kind: TrashKind::TopdirUser,
        root,
        topdir: Some(topdir.to_path_buf()),
        missing: Vec::new(),
    };
    with_missing(location, real_dir)
}

/// Fills `missing` with the absent ones of root, root/files, root/info; an existing one that is
/// not a folder (per `is_dir`) makes the trash unusable.
fn with_missing(
    mut location: TrashLocation,
    is_dir: impl Fn(&Path) -> io::Result<bool>,
) -> io::Result<Probe> {
    let root = location.root.clone();
    for folder in [root.clone(), root.join("files"), root.join("info")] {
        match is_dir(&folder) {
            Ok(true) => {}
            Ok(false) => return Ok(Err(TrashUnavailableReason::NoUsableTrash)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                location.missing.push(folder);
            }
            Err(error) => return Err(error),
        }
    }
    Ok(Ok(location))
}

fn real_dir(path: &Path) -> io::Result<bool> {
    dir_traits(path).map(|traits| traits.is_dir && !traits.is_symlink)
}

fn traits_if_present(path: &Path) -> io::Result<Option<DirTraits>> {
    match dir_traits(path) {
        Ok(traits) => Ok(Some(traits)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn followed_identity(path: &Path) -> io::Result<MountId> {
    identity(path, AtFlags::empty())
}

/// st_dev from std (btrfs subvolumes differ there); the mount id only when statx reports it.
fn identity(path: &Path, flags: AtFlags) -> io::Result<MountId> {
    let metadata = if flags.contains(AtFlags::SYMLINK_NOFOLLOW) {
        fs::symlink_metadata(path)?
    } else {
        fs::metadata(path)?
    };
    let mnt = match statx(CWD, path, flags, StatxFlags::MNT_ID) {
        Ok(stat) if StatxFlags::from_bits_retain(stat.stx_mask).contains(StatxFlags::MNT_ID) => {
            Some(stat.stx_mnt_id)
        }
        Ok(_) | Err(Errno::NOSYS | Errno::PERM) => None,
        Err(errno) => return Err(io::Error::from(errno)),
    };
    Ok(MountId {
        dev: metadata.dev(),
        mnt,
    })
}
