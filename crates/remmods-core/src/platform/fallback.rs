//! Unsupported OSes (e.g. macOS): compiles, classifies by `io::ErrorKind` only.

use std::fs::File;
use std::io;
use std::path::Path;
use std::process::Command;

use super::{
    DirTraits, MountId, NoReplace, OsErrorClass, OsKind, ProcessRecord, RegistryHive,
    RegistrySteamValues, RegistryView, RetryPolicy, SteamHints, TopdirPolicy, TrashEnv,
    TrashLocation, TrashUnavailableReason,
};

pub(super) const OS_KIND: OsKind = OsKind::Other;
pub(super) const APP_ORIGIN: &str = "tauri://localhost";

pub(super) fn classify_raw(_error: &io::Error) -> Option<OsErrorClass> {
    None
}

pub(super) fn retry_policy(_class: OsErrorClass) -> RetryPolicy {
    RetryPolicy::ONCE
}

pub(super) fn folder_retry_policy(_class: OsErrorClass) -> RetryPolicy {
    RetryPolicy::ONCE
}

pub(super) fn rename_noreplace(_from: &Path, _to: &Path) -> io::Result<NoReplace> {
    Err(io::ErrorKind::Unsupported.into())
}

pub(super) fn lock_probe(_path: &Path, _is_dir: bool) -> io::Result<()> {
    Ok(())
}

pub(super) fn replace_retry_policy(_class: OsErrorClass) -> RetryPolicy {
    RetryPolicy::ONCE
}

pub(super) fn fatal_message(title: &str, text: &str) {
    eprintln!("{title}: {text}");
}

pub(super) fn processes() -> io::Result<Vec<ProcessRecord>> {
    Err(io::ErrorKind::Unsupported.into())
}

pub(super) fn is_no_such_process(_error: &io::Error) -> bool {
    false
}

pub(super) fn total_memory() -> Option<u64> {
    None
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

pub(super) fn prepare_detached(_command: &mut Command) {}

pub(super) fn rename_noreplace_across(_from: &Path, _to: &Path) -> io::Result<NoReplace> {
    Err(io::ErrorKind::Unsupported.into())
}

pub(super) fn create_private_dir(_path: &Path) -> io::Result<()> {
    Err(io::ErrorKind::Unsupported.into())
}

pub(super) fn create_new_file(_path: &Path) -> io::Result<File> {
    Err(io::ErrorKind::Unsupported.into())
}

pub(super) fn dir_traits(_path: &Path) -> io::Result<DirTraits> {
    Err(io::ErrorKind::Unsupported.into())
}

pub(super) fn current_uid() -> Option<u32> {
    None
}

pub(super) fn trash_env_from_env() -> TrashEnv {
    TrashEnv {
        home_trash: None,
        uid: None,
        topdirs: TopdirPolicy::Deny,
    }
}

pub(super) fn trash_location(
    _env: &TrashEnv,
    _item: &Path,
) -> io::Result<Result<TrashLocation, TrashUnavailableReason>> {
    Ok(Err(TrashUnavailableReason::NotSupported))
}

pub(super) fn mount_identity(_path: &Path) -> io::Result<MountId> {
    Err(io::ErrorKind::Unsupported.into())
}
