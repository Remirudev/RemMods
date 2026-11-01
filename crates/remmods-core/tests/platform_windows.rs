#![cfg(windows)]

use std::io;
use std::time::Duration;

use remmods_core::platform::{
    OsErrorClass, OsKind, RetryPolicy, app_origin, classify, folder_retry_policy, os_kind,
    replace_retry_policy, retry_policy,
};

fn raw(code: i32) -> io::Error {
    io::Error::from_raw_os_error(code)
}

#[test]
fn classify_maps_the_win32_code_table() {
    let table = [
        (32, OsErrorClass::InUse),
        (33, OsErrorClass::InUse),
        (5, OsErrorClass::AccessDenied),
        (39, OsErrorClass::NoSpace),
        (112, OsErrorClass::NoSpace),
        (19, OsErrorClass::ReadOnlyFs),
        (17, OsErrorClass::CrossDevice),
        (206, OsErrorClass::NameTooLong),
        (2, OsErrorClass::NotFound),
        (3, OsErrorClass::NotFound),
        (50, OsErrorClass::Unsupported),
        (80, OsErrorClass::AlreadyExists),
        (183, OsErrorClass::AlreadyExists),
        (87, OsErrorClass::Other),
    ];
    for (code, expected) in table {
        assert_eq!(classify(&raw(code)), expected, "code {code}");
    }
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
    assert_eq!(
        classify(&io::Error::from(io::ErrorKind::AlreadyExists)),
        OsErrorClass::AlreadyExists
    );
    assert_eq!(classify(&io::Error::other("custom")), OsErrorClass::Other);
}

#[test]
fn in_use_is_retried_for_up_to_1_6_seconds() {
    let policy = retry_policy(OsErrorClass::InUse);

    assert!(policy.attempts > 1);
    assert_eq!(policy.total, Duration::from_millis(1600));
    assert!(policy.pause() > Duration::ZERO);
}

#[test]
fn other_classes_are_tried_once() {
    let classes = [
        OsErrorClass::AccessDenied,
        OsErrorClass::NoSpace,
        OsErrorClass::NotFound,
        OsErrorClass::Other,
    ];
    for class in classes {
        assert_eq!(retry_policy(class), RetryPolicy::ONCE, "{class:?}");
    }
}

#[test]
fn replacing_a_file_also_retries_access_denied() {
    let in_use = retry_policy(OsErrorClass::InUse);

    assert_eq!(replace_retry_policy(OsErrorClass::InUse), in_use);
    assert_eq!(replace_retry_policy(OsErrorClass::AccessDenied), in_use);
    assert_eq!(
        replace_retry_policy(OsErrorClass::NoSpace),
        RetryPolicy::ONCE
    );
    assert_eq!(
        replace_retry_policy(OsErrorClass::NotFound),
        RetryPolicy::ONCE
    );
}

#[test]
fn reports_windows_and_the_http_origin() {
    assert_eq!(os_kind(), OsKind::Windows);
    assert_eq!(app_origin(), "http://tauri.localhost");
}

#[test]
fn folder_renames_retry_in_use_and_access_denied_for_30_seconds() {
    for class in [OsErrorClass::InUse, OsErrorClass::AccessDenied] {
        let policy = folder_retry_policy(class);
        assert_eq!(policy.attempts, 61, "{class:?}");
        assert_eq!(policy.total, Duration::from_secs(30), "{class:?}");
        assert_eq!(policy.pause(), Duration::from_millis(500), "{class:?}");
    }
    assert_eq!(
        folder_retry_policy(OsErrorClass::NotFound),
        RetryPolicy::ONCE
    );
}
