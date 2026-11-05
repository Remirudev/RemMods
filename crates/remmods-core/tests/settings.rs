#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use remmods_core::AppPaths;
use remmods_core::safe_fs::{AppLock, FsError};
use remmods_core::settings::{
    AppearanceUpdate, ColorScheme, Settings, SettingsError, SettingsStore, SettingsUpdate,
    TitleBarMode, normalize_ui_scale,
};

fn write_raw(paths: &AppPaths, content: &str) {
    fs::create_dir_all(&paths.config_dir).expect("create config dir");
    fs::write(&paths.settings_file, content).expect("write settings fixture");
}

fn appearance(update: AppearanceUpdate) -> SettingsUpdate {
    SettingsUpdate {
        appearance: Some(update),
    }
}

fn scale_update(ui_scale: f64) -> SettingsUpdate {
    appearance(AppearanceUpdate {
        ui_scale: Some(ui_scale),
        ..AppearanceUpdate::default()
    })
}

fn backups_in(dir: &Path) -> Vec<PathBuf> {
    let mut backups: Vec<PathBuf> = fs::read_dir(dir)
        .expect("list config dir")
        .map(|entry| entry.expect("dir entry").path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("settings.json.backup."))
        })
        .collect();
    backups.sort();
    backups
}

#[test]
fn missing_file_gives_defaults_without_writing() {
    let (_dir, root) = support::temp_root();
    let paths = AppPaths::under(&root);

    let snapshot = SettingsStore::load(&paths).snapshot();

    assert_eq!(snapshot.settings, Settings::default());
    assert_eq!(snapshot.settings.version, 1);
    assert_eq!(snapshot.settings.appearance.scheme, ColorScheme::Dark);
    assert_eq!(snapshot.settings.appearance.ui_scale, 1.0);
    assert_eq!(snapshot.settings.appearance.title_bar, TitleBarMode::Custom);
    assert!(!snapshot.read_only);
    assert!(!snapshot.recovered);
    assert!(!paths.settings_file.exists());
}

#[test]
fn update_round_trips_through_disk_as_pretty_json() {
    let (_dir, root) = support::temp_root();
    let paths = AppPaths::under(&root);
    let store = SettingsStore::load(&paths);

    let updated = store
        .update(&appearance(AppearanceUpdate {
            scheme: Some(ColorScheme::Light),
            ui_scale: Some(1.25),
            title_bar: Some(TitleBarMode::System),
            hide_pictures: None,
        }))
        .expect("update settings");

    let reloaded = SettingsStore::load(&paths).snapshot();
    assert_eq!(reloaded, updated);
    assert_eq!(reloaded.settings.appearance.scheme, ColorScheme::Light);
    let text = fs::read_to_string(&paths.settings_file).expect("read settings");
    let expected = "{\n  \"version\": 1,\n  \"appearance\": {\n    \"scheme\": \"light\",\n    \
                    \"uiScale\": 1.25,\n    \"titleBar\": \"system\"\n  }\n}\n";
    assert_eq!(text, expected);
}

#[test]
fn partial_update_keeps_the_other_fields() {
    let (_dir, root) = support::temp_root();
    let paths = AppPaths::under(&root);
    let store = SettingsStore::load(&paths);
    store
        .update(&appearance(AppearanceUpdate {
            title_bar: Some(TitleBarMode::System),
            ..AppearanceUpdate::default()
        }))
        .expect("first update");

    let snapshot = store.update(&scale_update(0.9)).expect("second update");

    assert_eq!(snapshot.settings.appearance.title_bar, TitleBarMode::System);
    assert!(!snapshot.settings.appearance.hide_pictures);
    assert_eq!(snapshot.settings.appearance.scheme, ColorScheme::Dark);
    assert_eq!(snapshot.settings.appearance.ui_scale, 0.9);
}

#[test]
fn update_merges_changes_made_by_another_writer() {
    let (_dir, root) = support::temp_root();
    let paths = AppPaths::under(&root);
    let store = SettingsStore::load(&paths);
    write_raw(&paths, r#"{"version":1,"appearance":{"scheme":"system"}}"#);

    let snapshot = store.update(&scale_update(1.1)).expect("update");

    assert_eq!(snapshot.settings.appearance.scheme, ColorScheme::System);
    assert_eq!(snapshot.settings.appearance.ui_scale, 1.1);
}

#[test]
fn corrupt_file_is_backed_up_and_flagged_recovered() {
    let (_dir, root) = support::temp_root();
    let paths = AppPaths::under(&root);
    write_raw(&paths, "{ not json");

    let snapshot = SettingsStore::load(&paths).snapshot();

    assert!(snapshot.recovered);
    assert!(!snapshot.read_only);
    assert_eq!(snapshot.settings, Settings::default());
    let backups = backups_in(&paths.config_dir);
    assert_eq!(backups.len(), 1);
    assert_eq!(fs::read(&backups[0]).expect("read backup"), b"{ not json");
    let name = backups[0]
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let stamp = name.trim_start_matches("settings.json.backup.");
    assert_eq!(stamp.len(), 15, "stamp {stamp} is yyyyMMdd_HHmmss");
    assert!(stamp.chars().enumerate().all(|(i, c)| if i == 8 {
        c == '_'
    } else {
        c.is_ascii_digit()
    }));
    let reloaded = SettingsStore::load(&paths).snapshot();
    assert!(
        !reloaded.recovered,
        "defaults were written over the corrupt file"
    );
    assert_eq!(reloaded.settings, Settings::default());
}

#[test]
fn a_corrupt_file_is_backed_up_once_across_restarts() {
    let (_dir, root) = support::temp_root();
    let paths = AppPaths::under(&root);
    write_raw(&paths, "{ not json");

    let first = SettingsStore::load(&paths).snapshot();
    let second = SettingsStore::load(&paths).snapshot();
    let third = SettingsStore::load(&paths).snapshot();

    assert!(first.recovered);
    assert!(!second.recovered);
    assert!(!third.recovered);
    assert_eq!(backups_in(&paths.config_dir).len(), 1);
}

#[test]
fn corrupt_file_stays_while_another_process_holds_the_app_lock() {
    let (_dir, root) = support::temp_root();
    let paths = AppPaths::under(&root);
    write_raw(&paths, "{ not json");
    let held = AppLock::try_exclusive(&paths.lock_file).expect("take lock");

    let snapshot = SettingsStore::load(&paths).snapshot();

    assert!(snapshot.recovered);
    assert_eq!(backups_in(&paths.config_dir).len(), 1);
    assert_eq!(
        fs::read(&paths.settings_file).expect("read original"),
        b"{ not json"
    );
    drop(held);
}

#[test]
fn update_after_recovery_replaces_the_file_without_a_second_backup() {
    let (_dir, root) = support::temp_root();
    let paths = AppPaths::under(&root);
    write_raw(&paths, "[1, 2,");
    let store = SettingsStore::load(&paths);

    let snapshot = store.update(&scale_update(1.2)).expect("update");

    assert!(snapshot.recovered);
    assert_eq!(snapshot.settings.appearance.ui_scale, 1.2);
    assert_eq!(backups_in(&paths.config_dir).len(), 1);
    let reloaded = SettingsStore::load(&paths).snapshot();
    assert!(!reloaded.recovered);
    assert_eq!(reloaded.settings.appearance.ui_scale, 1.2);
}

#[test]
fn wrong_value_types_count_as_corrupt() {
    let (_dir, root) = support::temp_root();
    let paths = AppPaths::under(&root);
    write_raw(&paths, r#"{"version":1,"appearance":{"uiScale":"big"}}"#);

    let snapshot = SettingsStore::load(&paths).snapshot();

    assert!(snapshot.recovered);
    assert_eq!(backups_in(&paths.config_dir).len(), 1);
}

#[test]
fn newer_version_is_read_only_and_the_file_stays_untouched() {
    let (_dir, root) = support::temp_root();
    let paths = AppPaths::under(&root);
    let original = r#"{"version":2,"appearance":{"scheme":"neon","layout":[1,2]}}"#;
    write_raw(&paths, original);
    let store = SettingsStore::load(&paths);

    let snapshot = store.snapshot();
    let result = store.update(&scale_update(1.3));

    assert!(snapshot.read_only);
    assert!(!snapshot.recovered);
    assert_eq!(snapshot.settings, Settings::default());
    assert!(matches!(result, Err(SettingsError::ReadOnly)));
    assert_eq!(
        fs::read_to_string(&paths.settings_file).expect("read"),
        original
    );
    assert!(backups_in(&paths.config_dir).is_empty());
}

#[test]
fn ui_scale_is_clamped_and_rounded_to_five_percent_steps() {
    let cases = [
        (3.0, 1.5),
        (0.1, 0.8),
        (-2.0, 0.8),
        (1.13, 1.15),
        (1.12, 1.1),
        (0.87, 0.85),
        (1.0, 1.0),
        (1.5, 1.5),
    ];
    for (input, expected) in cases {
        assert_eq!(normalize_ui_scale(input), Some(expected), "input {input}");
    }
    assert_eq!(normalize_ui_scale(f64::NAN), None);
    assert_eq!(normalize_ui_scale(f64::INFINITY), None);
}

#[test]
fn update_clamps_scale_and_rejects_non_finite_values() {
    let (_dir, root) = support::temp_root();
    let paths = AppPaths::under(&root);
    let store = SettingsStore::load(&paths);

    let clamped = store.update(&scale_update(7.0)).expect("update");
    let rejected = store.update(&scale_update(f64::NAN));

    assert_eq!(clamped.settings.appearance.ui_scale, 1.5);
    assert!(matches!(
        rejected,
        Err(SettingsError::InvalidValue {
            field: "appearance.uiScale"
        })
    ));
    assert_eq!(store.snapshot().settings.appearance.ui_scale, 1.5);
}

#[test]
fn out_of_range_scale_on_disk_is_clamped_on_load() {
    let (_dir, root) = support::temp_root();
    let paths = AppPaths::under(&root);
    write_raw(&paths, r#"{"version":1,"appearance":{"uiScale":9}}"#);

    let snapshot = SettingsStore::load(&paths).snapshot();

    assert_eq!(snapshot.settings.appearance.ui_scale, 1.5);
    assert!(!snapshot.recovered);
}

#[test]
fn unknown_fields_are_ignored_and_missing_fields_default() {
    let (_dir, root) = support::temp_root();
    let paths = AppPaths::under(&root);
    write_raw(
        &paths,
        r#"{"version":1,"futureFlag":true,"appearance":{"scheme":"light","sparkles":3}}"#,
    );

    let snapshot = SettingsStore::load(&paths).snapshot();

    assert!(!snapshot.recovered);
    assert!(!snapshot.read_only);
    assert_eq!(snapshot.settings.appearance.scheme, ColorScheme::Light);
    assert_eq!(snapshot.settings.appearance.ui_scale, 1.0);
    assert_eq!(snapshot.settings.appearance.title_bar, TitleBarMode::Custom);
}

#[test]
fn update_is_busy_while_another_process_holds_the_app_lock() {
    let (_dir, root) = support::temp_root();
    let paths = AppPaths::under(&root);
    let store = SettingsStore::load(&paths);
    let _held = AppLock::try_exclusive(&paths.lock_file).expect("take lock");

    let result = store.update(&scale_update(1.1));

    assert!(matches!(result, Err(SettingsError::Fs(FsError::Busy))));
    assert!(!paths.settings_file.exists());
}

#[test]
fn hide_pictures_is_saved_only_while_on() {
    let (_dir, root) = support::temp_root();
    let paths = AppPaths::under(&root);
    let store = SettingsStore::load(&paths);
    let update = |hide_pictures| {
        appearance(AppearanceUpdate {
            hide_pictures: Some(hide_pictures),
            ..AppearanceUpdate::default()
        })
    };

    let on = store.update(&update(true)).expect("turn on");
    assert!(on.settings.appearance.hide_pictures);
    let text = fs::read_to_string(&paths.settings_file).expect("read");
    assert!(text.contains("\"hidePictures\": true"), "{text}");
    let reloaded = SettingsStore::load(&paths).snapshot();
    assert!(reloaded.settings.appearance.hide_pictures);

    store.update(&update(false)).expect("turn off");
    let text = fs::read_to_string(&paths.settings_file).expect("read");
    assert!(!text.contains("hidePictures"), "{text}");
}
