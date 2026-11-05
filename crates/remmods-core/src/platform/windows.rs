use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::time::Duration;

use windows_sys::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, ERROR_DISK_FULL, ERROR_FILE_EXISTS,
    ERROR_FILE_NOT_FOUND, ERROR_FILENAME_EXCED_RANGE, ERROR_HANDLE_DISK_FULL, ERROR_LOCK_VIOLATION,
    ERROR_NOT_SAME_DEVICE, ERROR_NOT_SUPPORTED, ERROR_PATH_NOT_FOUND, ERROR_SHARING_VIOLATION,
    ERROR_WRITE_PROTECT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MessageBoxW,
};

use super::{OsErrorClass, OsKind, RetryPolicy};

mod fs;
mod memory;
mod process;
mod recycle;
mod registry;
mod trash;

pub(super) use fs::{
    create_new_file, create_private_dir, lock_probe, rename_noreplace, rename_noreplace_across,
};
pub(super) use memory::total_memory;
pub(super) use process::processes;
pub(super) use recycle::{enter_mta_for_tests, recycle};
pub(super) use registry::{registry_string, steam_hints};
pub(super) use trash::{
    current_uid, dir_traits, mount_identity, trash_env_from_env, trash_location,
};

pub(super) const OS_KIND: OsKind = OsKind::Windows;
pub(super) const APP_ORIGIN: &str = "http://tauri.localhost";

/// Antivirus scanners and the indexer open fresh files briefly; sharing violations clear quickly.
const IN_USE_RETRY: RetryPolicy = RetryPolicy {
    attempts: 9,
    total: Duration::from_millis(1600),
};

pub(super) fn classify_raw(error: &io::Error) -> Option<OsErrorClass> {
    let code = u32::try_from(error.raw_os_error()?).ok()?;
    let class = match code {
        ERROR_SHARING_VIOLATION | ERROR_LOCK_VIOLATION => OsErrorClass::InUse,
        ERROR_ACCESS_DENIED => OsErrorClass::AccessDenied,
        ERROR_HANDLE_DISK_FULL | ERROR_DISK_FULL => OsErrorClass::NoSpace,
        ERROR_WRITE_PROTECT => OsErrorClass::ReadOnlyFs,
        ERROR_NOT_SAME_DEVICE => OsErrorClass::CrossDevice,
        ERROR_FILENAME_EXCED_RANGE => OsErrorClass::NameTooLong,
        ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => OsErrorClass::NotFound,
        ERROR_FILE_EXISTS | ERROR_ALREADY_EXISTS => OsErrorClass::AlreadyExists,
        ERROR_NOT_SUPPORTED => OsErrorClass::Unsupported,
        _ => return None,
    };
    Some(class)
}

pub(super) fn retry_policy(class: OsErrorClass) -> RetryPolicy {
    match class {
        OsErrorClass::InUse => IN_USE_RETRY,
        _ => RetryPolicy::ONCE,
    }
}

/// Explorer, the indexer and scanners hold folder handles without FILE_SHARE_DELETE for a while.
const FOLDER_RETRY: RetryPolicy = RetryPolicy {
    attempts: 61,
    total: Duration::from_secs(30),
};

pub(super) fn folder_retry_policy(class: OsErrorClass) -> RetryPolicy {
    match class {
        OsErrorClass::InUse | OsErrorClass::AccessDenied => FOLDER_RETRY,
        _ => RetryPolicy::ONCE,
    }
}

pub(super) fn save_games_root(_game_root: &Path, _app_id: u32) -> Option<PathBuf> {
    let local = dirs::data_local_dir()?;
    Some(local.join("TEKKEN 8").join("Saved").join("SaveGames"))
}

/// MoveFileExW(REPLACE_EXISTING) reports ACCESS_DENIED, not a sharing violation, when the target
/// is open without FILE_SHARE_DELETE; std's POSIX-semantics fallback fails the same way.
pub(super) fn replace_retry_policy(class: OsErrorClass) -> RetryPolicy {
    match class {
        OsErrorClass::InUse | OsErrorClass::AccessDenied => IN_USE_RETRY,
        _ => RetryPolicy::ONCE,
    }
}

pub(super) fn fatal_message(title: &str, text: &str) {
    let title = wide_nul(title);
    let text = wide_nul(text);
    // SAFETY: both buffers are NUL-terminated UTF-16 that outlive the call; a null owner window is allowed.
    unsafe {
        MessageBoxW(
            ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR | MB_SETFOREGROUND,
        );
    }
}

/// Procfs-shaped trees only exist in tests here; a vanished entry is plain NotFound.
pub(super) fn is_no_such_process(_error: &io::Error) -> bool {
    false
}

/// A GUI process has no console or session to leave.
pub(super) fn prepare_detached(_command: &mut Command) {}

fn wide_nul(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
