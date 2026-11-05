use std::io;
use std::os::fd::{BorrowedFd, RawFd};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use rustix::io::{Errno, FdFlags};

mod fs;
mod trash;

pub(super) use fs::{
    create_new_file, create_private_dir, lock_probe, rename_noreplace, rename_noreplace_across,
};
pub(super) use trash::{
    current_uid, dir_traits, enter_mta_for_tests, mount_identity, recycle, trash_env_from_env,
    trash_location,
};

use super::{
    OsErrorClass, OsKind, ProcessRecord, RegistryHive, RegistrySteamValues, RegistryView,
    RetryPolicy, SteamHints, read_proc_tree,
};

/// Below the Proton prefix; the game's name in it does not change with the language.
const PROTON_SAVE_PATH: [&str; 8] = [
    "drive_c",
    "users",
    "steamuser",
    "AppData",
    "Local",
    "TEKKEN 8",
    "Saved",
    "SaveGames",
];

pub(super) const OS_KIND: OsKind = OsKind::Linux;
pub(super) const APP_ORIGIN: &str = "tauri://localhost";

pub(super) fn classify_raw(error: &io::Error) -> Option<OsErrorClass> {
    let class = match Errno::from_io_error(error)? {
        Errno::BUSY | Errno::TXTBSY => OsErrorClass::InUse,
        Errno::ACCESS | Errno::PERM => OsErrorClass::AccessDenied,
        Errno::NOSPC | Errno::DQUOT => OsErrorClass::NoSpace,
        Errno::ROFS => OsErrorClass::ReadOnlyFs,
        Errno::XDEV => OsErrorClass::CrossDevice,
        Errno::NAMETOOLONG => OsErrorClass::NameTooLong,
        Errno::NOENT => OsErrorClass::NotFound,
        Errno::EXIST | Errno::NOTEMPTY => OsErrorClass::AlreadyExists,
        Errno::NOSYS | Errno::OPNOTSUPP => OsErrorClass::Unsupported,
        _ => return None,
    };
    Some(class)
}

/// Linux never retries: nothing holds files the way Windows sharing modes do.
pub(super) fn retry_policy(_class: OsErrorClass) -> RetryPolicy {
    RetryPolicy::ONCE
}

pub(super) fn folder_retry_policy(_class: OsErrorClass) -> RetryPolicy {
    RetryPolicy::ONCE
}

pub(super) fn save_games_root(game_root: &Path, app_id: u32) -> Option<PathBuf> {
    let steamapps = game_root.parent()?.parent()?;
    let prefix = steamapps
        .join("compatdata")
        .join(app_id.to_string())
        .join("pfx");
    Some(
        PROTON_SAVE_PATH
            .iter()
            .fold(prefix, |path, part| path.join(part)),
    )
}

/// rename(2) replaces a target that others hold open, so a failed replace is not transient.
pub(super) fn replace_retry_policy(_class: OsErrorClass) -> RetryPolicy {
    RetryPolicy::ONCE
}

pub(super) fn fatal_message(title: &str, text: &str) {
    eprintln!("{title}: {text}");
}

pub(super) fn processes() -> io::Result<Vec<ProcessRecord>> {
    read_proc_tree(Path::new("/proc"))
}

/// Reading `/proc/<pid>/cmdline` of a process that exits meanwhile fails with ESRCH.
pub(super) fn is_no_such_process(error: &io::Error) -> bool {
    Errno::from_io_error(error) == Some(Errno::SRCH)
}

pub(super) fn total_memory() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    super::parse_meminfo_total(&text)
}

pub(super) fn steam_hints() -> SteamHints {
    SteamHints {
        os: OS_KIND,
        home: dirs::home_dir(),
        data_dir: dirs::data_dir(),
        registry: RegistrySteamValues::default(),
        program_files_x86: None,
        program_files: None,
    }
}

pub(super) fn registry_string(
    _hive: RegistryHive,
    _subkey: &str,
    _value: &str,
    _view: RegistryView,
) -> io::Result<Option<String>> {
    Err(io::ErrorKind::Unsupported.into())
}

/// The child gets its own session, so closing RemMods (or its terminal) does not take the
/// launched program down with it. It also gets none of the descriptors RemMods inherited: the
/// AppImage runtime passes its keepalive pipe and the mount folder (fd 1023) without
/// close-on-exec, and a host program holding them keeps the mount, and a replaced AppImage, alive
/// after RemMods exits. RemMods keeps them open itself: only the child marks them close-on-exec.
pub(super) fn prepare_detached(command: &mut Command) {
    let candidates = fd_candidates();
    // SAFETY: the hook runs in the forked child before exec. It calls only setsid(2) and
    // fcntl(2), which are async-signal-safe, and reads `candidates`, which was collected before
    // the fork; it allocates nothing and touches no memory shared with the parent. FD_CLOEXEC is a
    // flag of the child's own descriptor table. Nothing is closed, so std's exec-error pipe works.
    unsafe {
        command.pre_exec(move || {
            rustix::process::setsid().map_err(io::Error::from)?;
            for &fd in &candidates {
                mark_cloexec(fd)?;
            }
            Ok(())
        });
    }
}

/// Descriptors from 3 up that may be open when the child forks: the entries of `/proc/self/fd`
/// (inherited ones are open from the start; everything RemMods, std or GLib opens later is
/// close-on-exec already), or every number below the soft `RLIMIT_NOFILE` when the listing fails.
/// Collected before the fork: reading a directory is not async-signal-safe.
fn fd_candidates() -> Vec<RawFd> {
    match listed_fds(Path::new("/proc/self/fd")) {
        Ok(fds) => fds,
        Err(error) => {
            log::debug!("listing /proc/self/fd failed ({error}); marking every descriptor");
            (FIRST_INHERITED_FD..fd_scan_limit()).collect()
        }
    }
}

const FIRST_INHERITED_FD: RawFd = 3;
/// Bounds the fallback scan when the soft limit is huge or unlimited; the runtime's fds sit low.
const FD_SCAN_CAP: RawFd = 1 << 16;

fn listed_fds(dir: &Path) -> io::Result<Vec<RawFd>> {
    let mut fds = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let name = entry?.file_name();
        let fd = name
            .to_str()
            .and_then(|name| name.parse::<RawFd>().ok())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "non-numeric fd entry"))?;
        if fd >= FIRST_INHERITED_FD {
            fds.push(fd);
        }
    }
    Ok(fds)
}

fn fd_scan_limit() -> RawFd {
    let soft = rustix::process::getrlimit(rustix::process::Resource::Nofile).current;
    soft.and_then(|soft| RawFd::try_from(soft).ok())
        .map_or(FD_SCAN_CAP, |soft| soft.min(FD_SCAN_CAP))
}

/// Runs in the forked child. A number that is not open (the listing's own directory handle, an
/// fd closed since) answers EBADF and is skipped.
fn mark_cloexec(fd: RawFd) -> io::Result<()> {
    // SAFETY: the borrow only carries the number into two fcntl(2) calls and never outlives this
    // function; nothing reads or closes through it, and a number that is not open gets EBADF.
    let borrowed = unsafe { BorrowedFd::borrow_raw(fd) };
    let flags = match rustix::io::fcntl_getfd(borrowed) {
        Ok(flags) => flags,
        Err(Errno::BADF) => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if flags.contains(FdFlags::CLOEXEC) {
        return Ok(());
    }
    match rustix::io::fcntl_setfd(borrowed, flags | FdFlags::CLOEXEC) {
        Ok(()) | Err(Errno::BADF) => Ok(()),
        Err(error) => Err(error.into()),
    }
}
