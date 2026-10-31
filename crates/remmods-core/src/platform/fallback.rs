//! Unsupported OSes (e.g. macOS): compiles, classifies by `io::ErrorKind` only.

use std::io;

use super::{OsErrorClass, OsKind, RetryPolicy};

pub(super) const OS_KIND: OsKind = OsKind::Other;
pub(super) const APP_ORIGIN: &str = "tauri://localhost";

pub(super) fn classify_raw(_error: &io::Error) -> Option<OsErrorClass> {
    None
}

pub(super) fn retry_policy(_class: OsErrorClass) -> RetryPolicy {
    RetryPolicy::ONCE
}

pub(super) fn replace_retry_policy(_class: OsErrorClass) -> RetryPolicy {
    RetryPolicy::ONCE
}

pub(super) fn fatal_message(title: &str, text: &str) {
    eprintln!("{title}: {text}");
}
