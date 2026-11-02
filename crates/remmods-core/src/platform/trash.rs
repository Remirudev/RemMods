//! Trash types shared by every OS (no cfg): where a trash lives and why there is none.

use std::path::PathBuf;

/// Detect: the real trash for the item — Linux: walk up to the mount's top dir (FDO "$topdir");
/// Windows: the item's volume Recycle Bin through the shell. Deny: never a topdir trash (Windows:
/// never the Recycle Bin). Fixed: tests treat this folder as the topdir of every path below it
/// (Windows: never the Recycle Bin).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TopdirPolicy {
    Detect,
    Deny,
    Fixed(PathBuf),
}

/// home_trash: `$XDG_DATA_HOME/Trash`, else `$HOME/.local/share/Trash`; None on Windows or
/// without a home. uid: the real uid on Linux, None on Windows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrashEnv {
    pub home_trash: Option<PathBuf>,
    pub uid: Option<u32>,
    pub topdirs: TopdirPolicy,
}

impl TrashEnv {
    /// Linux: from the environment (XDG_DATA_HOME must be absolute, else ignored per XDG), the
    /// real uid, `TopdirPolicy::Detect`. Windows: `{ None, None, Detect }` (the Recycle Bin).
    /// Other OSes: `{ None, None, Deny }`.
    pub fn from_env() -> TrashEnv {
        super::imp::trash_env_from_env()
    }
}

/// OtherDevice: no trash shares the item's mount and device (other drive, btrfs subvolume, …);
/// NoUsableTrash: no usable `.Trash/$uid`, `.Trash-$uid` unsafe or not creatable, or NOREPLACE
/// unsupported; NotSupported: no mover on this OS, or (Windows) the env is not `Detect` (tests,
/// `--smoke`); RecycleRefused: the shell would have deleted it permanently (no Recycle Bin on that
/// drive, bin turned off or too small, network path), the sink aborted and nothing changed;
/// NotShellSafe: Win32 would rewrite the path (trailing dot or space, device name, `.`/`..`,
/// > 259 UTF-16 units, not a drive or UNC path), so it never reaches the shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrashUnavailableReason {
    OtherDevice,
    NoUsableTrash,
    NotSupported,
    RecycleRefused,
    NotShellSafe,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrashKind {
    Home,
    TopdirShared,
    TopdirUser,
    RecycleBin,
}

/// What `platform::recycle` did with one item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recycled {
    /// In the Recycle Bin: the item is gone and the shell reported its new item for the root
    /// (`X:\$Recycle.Bin\<SID>\$R…`). Without that report it is never Done.
    Done { bin_path: PathBuf },
    /// The sink refused the permanent delete the shell asked for; the item is untouched.
    Refused,
    /// The plain path is not one Win32 keeps as is; never handed to the shell.
    NotShellSafe,
}

/// root: the trash folder (`…/Trash`, `$topdir/.Trash/$uid`, `$topdir/.Trash-$uid`; the Recycle
/// Bin: the volume root, display and log only, empty when unknown); topdir:
/// None = `Path=` absolute (home trash), Some = `Path=` relative to it; missing: the folders not
/// there yet, outermost first (`root`, `root/files`, `root/info`), created by safe_fs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrashLocation {
    pub kind: TrashKind,
    pub root: PathBuf,
    pub topdir: Option<PathBuf>,
    pub missing: Vec<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MountId {
    pub dev: u64,
    pub mnt: Option<u64>,
}

impl MountId {
    /// Same device, and the same mount when both sides know their mount id.
    pub fn same_mount(&self, other: &MountId) -> bool {
        self.dev == other.dev
            && match (self.mnt, other.mnt) {
                (Some(a), Some(b)) => a == b,
                _ => true,
            }
    }
}

/// An entry as seen without following it. `owner` is None where the OS has no uid (Windows).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DirTraits {
    pub owner: Option<u32>,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub sticky: bool,
}

impl DirTraits {
    /// A real folder (no link) that belongs to `uid` (any owner where the OS has none).
    pub(crate) fn is_private_dir_of(&self, uid: Option<u32>) -> bool {
        self.is_dir && !self.is_symlink && (self.owner.is_none() || self.owner == uid)
    }
}
