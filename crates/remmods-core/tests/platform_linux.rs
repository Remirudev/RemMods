#![cfg(target_os = "linux")]

use std::io;

use remmods_core::platform::{
    OsErrorClass, OsKind, RetryPolicy, app_origin, classify, os_kind, replace_retry_policy,
    retry_policy,
};
use rustix::io::Errno;

fn raw(code: i32) -> io::Error {
    io::Error::from_raw_os_error(code)
}

#[test]
fn classify_maps_the_errno_table() {
    let table = [
        (Errno::BUSY, OsErrorClass::InUse),
        (Errno::TXTBSY, OsErrorClass::InUse),
        (Errno::ACCESS, OsErrorClass::AccessDenied),
        (Errno::PERM, OsErrorClass::AccessDenied),
        (Errno::NOSPC, OsErrorClass::NoSpace),
        (Errno::DQUOT, OsErrorClass::NoSpace),
        (Errno::ROFS, OsErrorClass::ReadOnlyFs),
        (Errno::XDEV, OsErrorClass::CrossDevice),
        (Errno::NAMETOOLONG, OsErrorClass::NameTooLong),
        (Errno::NOENT, OsErrorClass::NotFound),
        (Errno::NOSYS, OsErrorClass::Unsupported),
        (Errno::OPNOTSUPP, OsErrorClass::Unsupported),
        (Errno::IO, OsErrorClass::Other),
        (Errno::EXIST, OsErrorClass::Other),
    ];
    for (errno, expected) in table {
        assert_eq!(classify(&raw(errno.raw_os_error())), expected, "{errno:?}");
    }
}

#[test]
fn windows_raw_codes_mean_something_else_here() {
    assert_eq!(
        classify(&raw(Errno::IO.raw_os_error())),
        OsErrorClass::Other
    );
    assert_eq!(
        classify(&raw(Errno::PIPE.raw_os_error())),
        OsErrorClass::Other
    );
    assert_eq!(
        classify(&raw(Errno::DOM.raw_os_error())),
        OsErrorClass::Other
    );
}

#[test]
fn errors_without_an_os_code_fall_back_to_their_kind() {
    assert_eq!(
        classify(&io::Error::from(io::ErrorKind::NotFound)),
        OsErrorClass::NotFound
    );
    assert_eq!(
        classify(&io::Error::from(io::ErrorKind::PermissionDenied)),
        OsErrorClass::AccessDenied
    );
    assert_eq!(classify(&io::Error::other("custom")), OsErrorClass::Other);
}

#[test]
fn linux_tries_every_operation_once() {
    let classes = [
        OsErrorClass::InUse,
        OsErrorClass::AccessDenied,
        OsErrorClass::NoSpace,
        OsErrorClass::Other,
    ];
    for class in classes {
        assert_eq!(retry_policy(class), RetryPolicy::ONCE, "{class:?}");
        assert_eq!(replace_retry_policy(class), RetryPolicy::ONCE, "{class:?}");
    }
}

#[test]
fn reports_linux_and_the_tauri_scheme_origin() {
    assert_eq!(os_kind(), OsKind::Linux);
    assert_eq!(app_origin(), "tauri://localhost");
}
