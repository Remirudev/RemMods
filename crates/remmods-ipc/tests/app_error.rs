#![allow(clippy::expect_used)]

use std::io;
use std::path::Path;

use remmods_core::CoreError;
use remmods_core::platform;
use remmods_core::safe_fs::FsError;
use remmods_core::settings::SettingsError;
use remmods_ipc::{AppError, OsErrorClass};
use serde_json::{Value, json};

fn to_json(error: &AppError) -> Value {
    serde_json::to_value(error).expect("serialize")
}

fn io_error(kind: io::ErrorKind) -> FsError {
    FsError::io("write", Path::new("settings.json"), io::Error::from(kind))
}

#[test]
fn every_variant_is_tagged_by_kind() {
    let cases = [
        (AppError::Busy, json!({ "kind": "busy" })),
        (
            AppError::Io {
                op: "write".into(),
                class: OsErrorClass::InUse,
            },
            json!({ "kind": "io", "op": "write", "class": "inUse" }),
        ),
        (
            AppError::InvalidInput {
                field: "smoke".into(),
            },
            json!({ "kind": "invalidInput", "field": "smoke" }),
        ),
        (AppError::ReadOnly, json!({ "kind": "readOnly" })),
        (AppError::Internal, json!({ "kind": "internal" })),
    ];
    for (error, expected) in cases {
        assert_eq!(to_json(&error), expected);
        let parsed: AppError = serde_json::from_value(expected).expect("deserialize");
        assert_eq!(parsed, error);
    }
}

#[test]
fn os_error_classes_are_camel_case() {
    let cases = [
        (OsErrorClass::InUse, "inUse"),
        (OsErrorClass::AccessDenied, "accessDenied"),
        (OsErrorClass::NoSpace, "noSpace"),
        (OsErrorClass::ReadOnlyFs, "readOnlyFs"),
        (OsErrorClass::CrossDevice, "crossDevice"),
        (OsErrorClass::NameTooLong, "nameTooLong"),
        (OsErrorClass::NotFound, "notFound"),
        (OsErrorClass::Unsupported, "unsupported"),
        (OsErrorClass::Other, "other"),
    ];
    for (class, expected) in cases {
        assert_eq!(
            serde_json::to_value(class).expect("serialize"),
            json!(expected)
        );
    }
}

#[test]
fn core_classes_map_one_to_one() {
    let cases = [
        (platform::OsErrorClass::InUse, OsErrorClass::InUse),
        (
            platform::OsErrorClass::AccessDenied,
            OsErrorClass::AccessDenied,
        ),
        (platform::OsErrorClass::NoSpace, OsErrorClass::NoSpace),
        (platform::OsErrorClass::ReadOnlyFs, OsErrorClass::ReadOnlyFs),
        (
            platform::OsErrorClass::CrossDevice,
            OsErrorClass::CrossDevice,
        ),
        (
            platform::OsErrorClass::NameTooLong,
            OsErrorClass::NameTooLong,
        ),
        (platform::OsErrorClass::NotFound, OsErrorClass::NotFound),
        (
            platform::OsErrorClass::Unsupported,
            OsErrorClass::Unsupported,
        ),
        (platform::OsErrorClass::Other, OsErrorClass::Other),
    ];
    for (core_class, expected) in cases {
        assert_eq!(OsErrorClass::from(core_class), expected);
    }
}

#[test]
fn fs_errors_map_to_busy_and_io() {
    assert_eq!(AppError::from(CoreError::Fs(FsError::Busy)), AppError::Busy);
    assert_eq!(
        AppError::from(CoreError::Fs(io_error(io::ErrorKind::NotFound))),
        AppError::Io {
            op: "write".into(),
            class: OsErrorClass::NotFound,
        }
    );
}

#[test]
fn settings_errors_map_to_their_kinds() {
    let encode_error = serde_json::from_str::<u8>("x").expect_err("invalid json");
    let cases = [
        (SettingsError::ReadOnly, AppError::ReadOnly),
        (
            SettingsError::InvalidValue {
                field: "appearance.uiScale",
            },
            AppError::InvalidInput {
                field: "appearance.uiScale".into(),
            },
        ),
        (SettingsError::Encode(encode_error), AppError::Internal),
        (SettingsError::Fs(FsError::Busy), AppError::Busy),
        (
            SettingsError::Fs(io_error(io::ErrorKind::PermissionDenied)),
            AppError::Io {
                op: "write".into(),
                class: OsErrorClass::AccessDenied,
            },
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(AppError::from(CoreError::Settings(error)), expected);
    }
}

#[test]
fn missing_home_dir_is_internal() {
    assert_eq!(
        AppError::from(CoreError::MissingHomeDir),
        AppError::Internal
    );
}

#[test]
fn serialized_errors_carry_no_os_details() {
    let error = AppError::from(CoreError::Fs(io_error(io::ErrorKind::NotFound)));

    let text = serde_json::to_string(&error).expect("serialize");

    assert!(!text.contains("settings.json"));
    assert_eq!(text, r#"{"kind":"io","op":"write","class":"notFound"}"#);
}
