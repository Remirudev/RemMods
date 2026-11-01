#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod game_support;
mod support;

use std::fs;
use std::path::{Path, PathBuf};

use game_support::{install_game, steam_text};
use remmods_core::game::{GameLocation, GameOptions, LocationChoice};
use remmods_core::settings::{
    AppearanceUpdate, ColorScheme, GameSettings, LastSeenBuild, SettingsStore, SettingsUpdate,
    StoredLocation, StoredSource,
};
use remmods_core::{AppPaths, Core, CoreOptions};
use serde_json::{Value, json};

fn write_raw(paths: &AppPaths, content: &str) {
    fs::create_dir_all(&paths.config_dir).expect("create config dir");
    fs::write(&paths.settings_file, content).expect("write settings");
}

fn disk(paths: &AppPaths) -> Value {
    serde_json::from_str(&fs::read_to_string(&paths.settings_file).expect("read")).expect("JSON")
}

fn backups(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .expect("list")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("settings.json.backup."))
        })
        .collect()
}

fn light() -> SettingsUpdate {
    SettingsUpdate {
        appearance: Some(AppearanceUpdate {
            scheme: Some(ColorScheme::Light),
            ..AppearanceUpdate::default()
        }),
    }
}

const ROOT: &str = "/home/u/.local/share/Steam/steamapps/common/TEKKEN 8";

#[test]
fn location_round_trips_as_the_documented_json() {
    let (_dir, root) = support::temp_root();
    let paths = AppPaths::under(&root);
    let store = SettingsStore::load(&paths);
    store
        .update_game(|game| {
            game.location = Some(StoredLocation {
                game_root: ROOT.to_owned(),
                source: StoredSource::Steam,
            });
            game.last_seen_build = Some(LastSeenBuild {
                game_root: ROOT.to_owned(),
                build_id: "25221600".to_owned(),
            });
        })
        .expect("update_game");
    let expected = json!({
        "version": 1,
        "appearance": { "scheme": "dark", "uiScale": 1.0, "titleBar": "custom" },
        "game": {
            "location": { "gameRoot": ROOT, "source": "steam" },
            "lastSeenBuild": { "gameRoot": ROOT, "buildId": "25221600" }
        }
    });
    assert_eq!(disk(&paths), expected);
    let reloaded = SettingsStore::load(&paths).snapshot().settings.game;
    assert_eq!(reloaded, store.snapshot().settings.game);
}

#[test]
fn empty_game_section_is_omitted() {
    let (_dir, root) = support::temp_root();
    let paths = AppPaths::under(&root);
    let store = SettingsStore::load(&paths);
    store.update(&light()).expect("update");
    assert!(disk(&paths).get("game").is_none());
    assert!(GameSettings::default().is_empty());
}

#[test]
fn malformed_game_sections_read_as_defaults_and_are_backed_up_once() {
    for raw in [
        r#"{"version":1,"appearance":{"scheme":"light"},"game":5}"#,
        r#"{"version":1,"appearance":{"scheme":"light"},"game":{"location":{"gameRoot":3}}}"#,
    ] {
        let (_dir, root) = support::temp_root();
        let paths = AppPaths::under(&root);
        write_raw(&paths, raw);
        let store = SettingsStore::load(&paths);
        let snapshot = store.snapshot();
        assert_eq!(snapshot.settings.game, GameSettings::default());
        assert_eq!(snapshot.settings.appearance.scheme, ColorScheme::Light);
        assert!(!snapshot.read_only && !snapshot.recovered);
        assert!(backups(&paths.config_dir).is_empty(), "no backup on read");

        store.update(&light()).expect("first update");
        let made = backups(&paths.config_dir);
        assert_eq!(made.len(), 1);
        assert_eq!(fs::read_to_string(&made[0]).expect("backup"), raw);
        store.update(&light()).expect("second update");
        assert_eq!(backups(&paths.config_dir).len(), 1);
    }
}

#[test]
fn update_game_merges_with_another_writer() {
    let (_dir, root) = support::temp_root();
    let paths = AppPaths::under(&root);
    let first = SettingsStore::load(&paths);
    let second = SettingsStore::load(&paths);
    first.update(&light()).expect("appearance");
    second
        .update_game(|game| {
            game.location = Some(StoredLocation {
                game_root: ROOT.to_owned(),
                source: StoredSource::Manual,
            });
        })
        .expect("game");
    let on_disk = disk(&paths);
    assert_eq!(on_disk["appearance"]["scheme"], "light");
    assert_eq!(on_disk["game"]["location"]["source"], "manual");
}

#[test]
fn read_only_settings_keep_the_location_in_memory() {
    let (_dir, root) = support::temp_root();
    let paths = AppPaths::under(&root.join("app"));
    let original = r#"{"version":2,"appearance":{}}"#;
    write_raw(&paths, original);
    let store = SettingsStore::load(&paths);
    assert!(store.update_game(|game| game.location = None).is_err());

    let game = install_game(&root.join("lib"), &steam_text("appmanifest_1778820.acf"));
    let options = CoreOptions {
        game: GameOptions {
            steam_detection: false,
            fixed_location: None,
            steam_hints: None,
            processes: None,
        },
        require_marker: None,
        ..CoreOptions::default()
    };
    let core = Core::open_with(paths.clone(), options).expect("open");
    let status = core
        .game_set_location(LocationChoice::Path { path: game.clone() })
        .expect("set location");
    let GameLocation::Ready(install) = status.location else {
        panic!("expected Ready");
    };
    assert_eq!(install.root, game);
    assert!(core.game_paks().is_some());
    assert_eq!(
        fs::read_to_string(&paths.settings_file).expect("read"),
        original
    );
}
