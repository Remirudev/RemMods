use std::io;

use rustix::io::Errno;

use super::{OsErrorClass, OsKind, RetryPolicy};

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
        Errno::NOSYS | Errno::OPNOTSUPP => OsErrorClass::Unsupported,
        _ => return None,
    };
    Some(class)
}

/// Linux never retries: nothing holds files the way Windows sharing modes do.
pub(super) fn retry_policy(_class: OsErrorClass) -> RetryPolicy {
    RetryPolicy::ONCE
}

/// rename(2) replaces a target that others hold open, so a failed replace is not transient.
pub(super) fn replace_retry_policy(_class: OsErrorClass) -> RetryPolicy {
    RetryPolicy::ONCE
}

pub(super) fn fatal_message(title: &str, text: &str) {
    eprintln!("{title}: {text}");
}
