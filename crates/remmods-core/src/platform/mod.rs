//! The only module with `cfg` and `unsafe`. Every function here has the same signature on every
//! OS; the per-OS files only fill in the details. Raw OS error codes collide across systems
//! (Windows 5/32/33 are EIO/EPIPE/EDOM on Linux), so errors are classified here and nowhere else.

use std::io;
use std::time::Duration;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as imp;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as imp;

#[cfg(not(any(target_os = "linux", windows)))]
mod fallback;
#[cfg(not(any(target_os = "linux", windows)))]
use fallback as imp;

/// OS-independent meaning of an I/O error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OsErrorClass {
    InUse,
    AccessDenied,
    NoSpace,
    ReadOnlyFs,
    CrossDevice,
    NameTooLong,
    NotFound,
    Unsupported,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OsKind {
    Linux,
    Windows,
    Other,
}

/// How often an operation that failed with a given [`OsErrorClass`] may be tried in total.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetryPolicy {
    pub attempts: u32,
    pub total: Duration,
}

impl RetryPolicy {
    pub const ONCE: Self = Self {
        attempts: 1,
        total: Duration::ZERO,
    };

    /// Pause between two attempts, spreading the attempts evenly over `total`.
    pub fn pause(&self) -> Duration {
        match self.attempts {
            0 | 1 => Duration::ZERO,
            attempts => self.total / (attempts - 1),
        }
    }
}

pub fn classify(error: &io::Error) -> OsErrorClass {
    imp::classify_raw(error).unwrap_or_else(|| classify_kind(error.kind()))
}

pub fn retry_policy(class: OsErrorClass) -> RetryPolicy {
    imp::retry_policy(class)
}

/// Policy for replacing an existing file (the rename that ends `write_atomic`). On Windows a
/// target held open without `FILE_SHARE_DELETE` (scanner, indexer, sync tool, editor) makes the
/// replace fail with ACCESS_DENIED, so there that class means "in use" as well.
pub fn replace_retry_policy(class: OsErrorClass) -> RetryPolicy {
    imp::replace_retry_policy(class)
}

pub fn os_kind() -> OsKind {
    imp::OS_KIND
}

/// Origin the bundled frontend is served from (the custom protocol differs per OS).
pub fn app_origin() -> &'static str {
    imp::APP_ORIGIN
}

/// Last-resort error report for failures before the window exists.
pub fn fatal_message(title: &str, text: &str) {
    imp::fatal_message(title, text);
}

fn classify_kind(kind: io::ErrorKind) -> OsErrorClass {
    match kind {
        io::ErrorKind::NotFound => OsErrorClass::NotFound,
        io::ErrorKind::PermissionDenied => OsErrorClass::AccessDenied,
        io::ErrorKind::Unsupported => OsErrorClass::Unsupported,
        _ => OsErrorClass::Other,
    }
}
