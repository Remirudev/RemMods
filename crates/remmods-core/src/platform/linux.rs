use std::io;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

use rustix::io::Errno;

mod fs;
mod trash;

pub(super) use fs::{
    create_new_file, create_private_dir, lock_probe, rename_noreplace, rename_noreplace_across,
};
pub(super) use trash::{
    current_uid, dir_traits, mount_identity, trash_env_from_env, trash_location,
};

use super::{
    OsErrorClass, OsKind, ProcessRecord, RegistryHive, RegistrySteamValues, RegistryView,
    RetryPolicy, SteamHints, read_proc_tree,
};

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
/// launched program down with it.
pub(super) fn prepare_detached(command: &mut Command) {
    // SAFETY: the hook runs in the forked child before exec and only calls setsid(2), which is
    // async-signal-safe and touches no memory shared with the parent.
    unsafe {
        command.pre_exec(|| rustix::process::setsid().map(drop).map_err(io::Error::from));
    }
}
