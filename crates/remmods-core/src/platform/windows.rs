use std::io;
use std::ptr;
use std::time::Duration;

use windows_sys::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_DISK_FULL, ERROR_FILE_NOT_FOUND, ERROR_FILENAME_EXCED_RANGE,
    ERROR_HANDLE_DISK_FULL, ERROR_LOCK_VIOLATION, ERROR_NOT_SAME_DEVICE, ERROR_NOT_SUPPORTED,
    ERROR_PATH_NOT_FOUND, ERROR_SHARING_VIOLATION, ERROR_WRITE_PROTECT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MessageBoxW,
};

use super::{OsErrorClass, OsKind, RetryPolicy};

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

fn wide_nul(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
