#![allow(clippy::expect_used)]

use std::path::Path;

use remmods_core::AppPaths;
use remmods_core::platform;
use remmods_core::settings as core;
use remmods_ipc::{
    AppPathsDto, Appearance, AppearanceUpdate, ColorScheme, OsKind, PlatformInfo, Settings,
    SettingsUpdate, SmokeCheck, SmokeReport, TitleBarMode,
};
use serde_json::{Value, json};

fn to_json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialize")
}

fn from_json<T: serde::de::DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).expect("deserialize")
}

fn sample_paths() -> AppPathsDto {
    AppPathsDto {
        config: "/c".into(),
        data: "/d".into(),
        logs: "/d/logs".into(),
    }
}

#[test]
fn os_kind_is_lowercase() {
    assert_eq!(to_json(&OsKind::Linux), json!("linux"));
    assert_eq!(to_json(&OsKind::Windows), json!("windows"));
    assert_eq!(to_json(&OsKind::Other), json!("other"));
}

#[test]
fn platform_info_shape() {
    let info = PlatformInfo {
        os: OsKind::Linux,
        arch: "x86_64".into(),
        app_version: "0.1.0".into(),
        webview_version: None,
        smoke: true,
        paths: sample_paths(),
    };

    let expected = json!({
        "os": "linux",
        "arch": "x86_64",
        "appVersion": "0.1.0",
        "webviewVersion": null,
        "smoke": true,
        "paths": { "config": "/c", "data": "/d", "logs": "/d/logs" }
    });
    assert_eq!(to_json(&info), expected);
    assert_eq!(from_json::<PlatformInfo>(expected), info);
}

#[test]
fn scheme_and_title_bar_are_lowercase_strings() {
    assert_eq!(to_json(&ColorScheme::Dark), json!("dark"));
    assert_eq!(to_json(&ColorScheme::Light), json!("light"));
    assert_eq!(to_json(&ColorScheme::System), json!("system"));
    assert_eq!(to_json(&TitleBarMode::Custom), json!("custom"));
    assert_eq!(to_json(&TitleBarMode::System), json!("system"));
}

#[test]
fn settings_shape() {
    let settings = Settings {
        version: 1,
        appearance: Appearance {
            scheme: ColorScheme::Dark,
            ui_scale: 1.25,
            title_bar: TitleBarMode::Custom,
        },
        read_only: false,
        recovered: true,
    };

    let expected = json!({
        "version": 1,
        "appearance": { "scheme": "dark", "uiScale": 1.25, "titleBar": "custom" },
        "readOnly": false,
        "recovered": true
    });
    assert_eq!(to_json(&settings), expected);
    assert_eq!(from_json::<Settings>(expected), settings);
}

#[test]
fn settings_update_fields_may_be_omitted_or_null() {
    let empty: SettingsUpdate = from_json(json!({}));
    let partial: SettingsUpdate = from_json(json!({ "appearance": { "uiScale": 1.1 } }));
    let explicit: SettingsUpdate = from_json(json!({
        "appearance": { "scheme": "light", "uiScale": null, "titleBar": "system" }
    }));

    assert_eq!(empty, SettingsUpdate::default());
    assert_eq!(
        partial.appearance,
        Some(AppearanceUpdate {
            ui_scale: Some(1.1),
            ..AppearanceUpdate::default()
        })
    );
    assert_eq!(
        explicit.appearance,
        Some(AppearanceUpdate {
            scheme: Some(ColorScheme::Light),
            ui_scale: None,
            title_bar: Some(TitleBarMode::System),
        })
    );
    assert_eq!(
        to_json(&AppearanceUpdate::default()),
        json!({ "scheme": null, "uiScale": null, "titleBar": null })
    );
}

#[test]
fn smoke_report_shape() {
    let report = SmokeReport {
        ok: false,
        checks: vec![
            SmokeCheck {
                name: "rail".into(),
                ok: true,
                detail: None,
            },
            SmokeCheck {
                name: "navigation".into(),
                ok: false,
                detail: Some("origin changed".into()),
            },
        ],
    };

    let expected = json!({
        "ok": false,
        "checks": [
            { "name": "rail", "ok": true, "detail": null },
            { "name": "navigation", "ok": false, "detail": "origin changed" }
        ]
    });
    assert_eq!(to_json(&report), expected);
    assert_eq!(from_json::<SmokeReport>(expected), report);
    let without_detail: SmokeCheck = from_json(json!({ "name": "rail", "ok": true }));
    assert_eq!(without_detail.detail, None);
}

#[test]
fn smoke_report_passes_only_with_all_checks_green() {
    let check = |ok| SmokeCheck {
        name: "c".into(),
        ok,
        detail: None,
    };
    let report = |ok, checks| SmokeReport { ok, checks };

    assert!(report(true, vec![check(true), check(true)]).passed());
    assert!(!report(true, vec![check(true), check(false)]).passed());
    assert!(!report(false, vec![check(true)]).passed());
    assert!(!report(true, vec![]).passed());
}

#[test]
fn core_types_convert_into_dtos() {
    let snapshot = core::SettingsSnapshot {
        settings: core::Settings {
            version: 1,
            appearance: core::Appearance {
                scheme: core::ColorScheme::System,
                ui_scale: 0.9,
                title_bar: core::TitleBarMode::System,
            },
            game: Default::default(),
        },
        read_only: true,
        recovered: false,
    };

    let settings = Settings::from(snapshot);

    assert_eq!(settings.version, 1);
    assert_eq!(settings.appearance.scheme, ColorScheme::System);
    assert_eq!(settings.appearance.ui_scale, 0.9);
    assert_eq!(settings.appearance.title_bar, TitleBarMode::System);
    assert!(settings.read_only);
    assert!(!settings.recovered);
    assert_eq!(OsKind::from(platform::OsKind::Linux), OsKind::Linux);
    assert_eq!(OsKind::from(platform::OsKind::Windows), OsKind::Windows);
    assert_eq!(OsKind::from(platform::OsKind::Other), OsKind::Other);
}

#[test]
fn app_paths_become_display_strings() {
    let root = Path::new("root");

    let dto = AppPathsDto::from(&AppPaths::under(root));

    assert_eq!(dto.config, root.join("config").display().to_string());
    assert_eq!(dto.data, root.join("data").display().to_string());
    assert_eq!(
        dto.logs,
        root.join("data").join("logs").display().to_string()
    );
}

#[test]
fn settings_update_converts_into_the_core_update() {
    let update = SettingsUpdate {
        appearance: Some(AppearanceUpdate {
            scheme: Some(ColorScheme::Light),
            ui_scale: Some(1.4),
            title_bar: Some(TitleBarMode::Custom),
        }),
    };

    let core_update = core::SettingsUpdate::from(update);

    assert_eq!(
        core_update.appearance,
        Some(core::AppearanceUpdate {
            scheme: Some(core::ColorScheme::Light),
            ui_scale: Some(1.4),
            title_bar: Some(core::TitleBarMode::Custom),
        })
    );
    assert_eq!(
        core::SettingsUpdate::from(SettingsUpdate::default()),
        core::SettingsUpdate::default()
    );
}
