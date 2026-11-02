//! JSON of the self-update DTOs and errors (SPEC-P5 §9.1 I-U).
#![allow(clippy::expect_used)]

use remmods_core::app_update::{InstallKind, UpdateBlock as CoreUpdateBlock};
use remmods_ipc::{
    AppError, AppInstallKind, AppUpdateChannel, AppUpdateCheck, AppUpdateProgress, PlatformFeature,
    UpdateBlock, UpdateStage,
};
use serde_json::{Value, json};

fn to_json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialize")
}

fn round_trip<T>(value: &T, expected: &Value)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    assert_eq!(&to_json(value), expected);
    let parsed: T = serde_json::from_value(expected.clone()).expect("deserialize");
    assert_eq!(&parsed, value);
}

#[test]
fn channel_shape() {
    let channel = AppUpdateChannel {
        install: AppInstallKind::AppImage,
        self_update: true,
        configured: false,
        current: "0.1.0".into(),
    };
    round_trip(
        &channel,
        &json!({"install":"appImage","selfUpdate":true,"configured":false,"current":"0.1.0"}),
    );
}

#[test]
fn every_install_kind_string() {
    let cases = [
        (AppInstallKind::AppImage, "appImage"),
        (AppInstallKind::Nsis, "nsis"),
        (AppInstallKind::Deb, "deb"),
        (AppInstallKind::Rpm, "rpm"),
        (AppInstallKind::Aur, "aur"),
        (AppInstallKind::Unknown, "unknown"),
    ];
    for (kind, text) in cases {
        round_trip(&kind, &json!(text));
    }
}

#[test]
fn install_kinds_convert_one_to_one() {
    let cases = [
        (InstallKind::AppImage, AppInstallKind::AppImage),
        (InstallKind::Nsis, AppInstallKind::Nsis),
        (InstallKind::Deb, AppInstallKind::Deb),
        (InstallKind::Rpm, AppInstallKind::Rpm),
        (InstallKind::Aur, AppInstallKind::Aur),
        (InstallKind::Unknown, AppInstallKind::Unknown),
    ];
    for (core, ipc) in cases {
        assert_eq!(AppInstallKind::from(core), ipc);
    }
    assert_eq!(
        UpdateBlock::from(CoreUpdateBlock::InstallRunning),
        UpdateBlock::InstallRunning
    );
}

#[test]
fn check_variants() {
    round_trip(&AppUpdateCheck::UpToDate, &json!({"kind":"upToDate"}));
    round_trip(
        &AppUpdateCheck::Available {
            version: "1.0.1".into(),
            date: Some("2026-11-02T10:00:00Z".into()),
            notes: Some("Fixes".into()),
        },
        &json!({"kind":"available","version":"1.0.1","date":"2026-11-02T10:00:00Z","notes":"Fixes"}),
    );
    round_trip(
        &AppUpdateCheck::Available {
            version: "1.0.1".into(),
            date: None,
            notes: None,
        },
        &json!({"kind":"available","version":"1.0.1","date":null,"notes":null}),
    );
}

#[test]
fn progress_variants() {
    round_trip(
        &AppUpdateProgress::Downloading {
            downloaded: 10.0,
            total: None,
        },
        &json!({"kind":"downloading","downloaded":10.0,"total":null}),
    );
    round_trip(
        &AppUpdateProgress::Downloading {
            downloaded: 10.0,
            total: Some(20.0),
        },
        &json!({"kind":"downloading","downloaded":10.0,"total":20.0}),
    );
    round_trip(
        &AppUpdateProgress::Installing,
        &json!({"kind":"installing"}),
    );
    round_trip(
        &AppUpdateProgress::Restarting,
        &json!({"kind":"restarting"}),
    );
}

#[test]
fn update_errors_are_tagged() {
    let cases = [
        (
            AppError::UpdateNotConfigured,
            json!({"kind":"updateNotConfigured"}),
        ),
        (
            AppError::UpdateNotAllowed {
                install: AppInstallKind::Deb,
            },
            json!({"kind":"updateNotAllowed","install":"deb"}),
        ),
        (
            AppError::UpdateBlocked {
                reason: UpdateBlock::InstallRunning,
            },
            json!({"kind":"updateBlocked","reason":"installRunning"}),
        ),
        (
            AppError::UpdateBlocked {
                reason: UpdateBlock::Busy,
            },
            json!({"kind":"updateBlocked","reason":"busy"}),
        ),
        (
            AppError::UpdateFailed {
                stage: UpdateStage::Verify,
            },
            json!({"kind":"updateFailed","stage":"verify"}),
        ),
        (
            AppError::Unsupported {
                feature: PlatformFeature::GraphicsFallback,
            },
            json!({"kind":"unsupported","feature":"graphicsFallback"}),
        ),
    ];
    for (error, expected) in cases {
        round_trip(&error, &expected);
    }
}

#[test]
fn every_stage_string() {
    let cases = [
        (UpdateStage::Check, "check"),
        (UpdateStage::Download, "download"),
        (UpdateStage::Verify, "verify"),
        (UpdateStage::Install, "install"),
    ];
    for (stage, text) in cases {
        round_trip(&stage, &json!(text));
    }
}
