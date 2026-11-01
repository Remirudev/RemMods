//! `mod-meta.json` and its store (spec §5.4).
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use remmods_core::AppPaths;
use remmods_core::mods::{DisableRecord, MOD_META_FILE, ModMeta, ModMetaStore};
use remmods_core::safe_fs::{OpGate, Scope};
use tempfile::TempDir;

struct Store {
    _dir: TempDir,
    paths: AppPaths,
    paks: PathBuf,
    gate: OpGate,
    store: ModMetaStore,
}

impl Store {
    fn new() -> Self {
        let (dir, root) = support::temp_root();
        let paks = root.join("Paks");
        fs::create_dir_all(paks.join("Mods")).expect("create Mods");
        let paths = AppPaths::under(&root.join("app"));
        fs::create_dir_all(&paths.data_dir).expect("create data dir");
        let gate = OpGate::open(&paths).expect("open gate");
        let store = ModMetaStore::new(paths.data_dir.join(MOD_META_FILE));
        Self {
            _dir: dir,
            paths,
            paks,
            gate,
            store,
        }
    }

    fn update(&self, change: impl FnOnce(&mut ModMeta)) {
        let app_dirs = [self.paths.data_dir.as_path()];
        let scope = Scope::for_paks(&self.paks, &app_dirs).expect("scope");
        let guard = self.gate.write(scope).expect("write guard");
        self.store.update(&guard, change).expect("update");
    }

    fn file(&self) -> &Path {
        self.store.path()
    }

    fn json(&self) -> serde_json::Value {
        let text = fs::read_to_string(self.file()).expect("read mod-meta.json");
        serde_json::from_str(&text).expect("parse mod-meta.json")
    }
}

fn record(file: &str) -> DisableRecord {
    DisableRecord {
        files: vec![file.to_owned(), "Sub/Cool_P.ucas".to_owned()],
        at: "2026-11-01T10:00:00.000Z".to_owned(),
    }
}

#[test]
fn records_round_trip() {
    let store = Store::new();
    let loaded = store.store.load();
    assert!(!loaded.read_only);
    assert_eq!(loaded.meta, ModMeta::fresh());
    store.update(|meta| meta.set_record("Standard:Cool", Some(record("Cool.pak"))));
    let loaded = store.store.load();
    assert_eq!(
        loaded.meta.record("Standard:Cool"),
        Some(&record("Cool.pak"))
    );
    let json = store.json();
    assert_eq!(json["version"], 1);
    assert_eq!(
        json["mods"]["Standard:Cool"]["disabled"]["files"][1],
        "Sub/Cool_P.ucas"
    );
}

#[test]
fn unknown_fields_survive_at_both_levels() {
    let store = Store::new();
    let text = r#"{"version":1,"revision":3,"future":{"a":1},
        "mods":{"Standard:Cool":{"added":"2026-10-01","disabled":{"files":["Cool.pak"],"at":"x"}}}}"#;
    fs::write(store.file(), text).expect("write");
    store.update(|meta| meta.set_record("Standard:Cool", None));
    let json = store.json();
    assert_eq!(json["future"]["a"], 1);
    assert_eq!(json["mods"]["Standard:Cool"]["added"], "2026-10-01");
    assert!(json["mods"]["Standard:Cool"].get("disabled").is_none());
    assert_eq!(json["revision"], 4);
}

#[test]
fn a_corrupt_file_is_backed_up() {
    let store = Store::new();
    fs::write(store.file(), b"{ not json").expect("write");
    let loaded = store.store.load();
    assert!(!loaded.read_only);
    assert_eq!(loaded.meta, ModMeta::fresh());
    let backups = fs::read_dir(&store.paths.data_dir)
        .expect("list data dir")
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.starts_with(MOD_META_FILE) && name != MOD_META_FILE
        })
        .count();
    assert_eq!(backups, 1);
}

#[test]
fn a_newer_version_is_read_only() {
    let store = Store::new();
    let text = r#"{"version":2,"revision":9,"mods":{"Standard:Cool":{"disabled":{"files":["Cool.pak"],"at":"x"}}}}"#;
    fs::write(store.file(), text).expect("write");
    let loaded = store.store.load();
    assert!(loaded.read_only);
    assert_eq!(loaded.meta.record("Standard:Cool"), None);
    store.update(|meta| meta.set_record("Standard:Other", Some(record("Other.pak"))));
    assert_eq!(fs::read_to_string(store.file()).expect("read"), text);
}

#[test]
fn every_write_increments_the_revision() {
    let store = Store::new();
    store.update(|meta| meta.set_record("Standard:A", Some(record("a.pak"))));
    assert_eq!(store.json()["revision"], 1);
    store.update(|meta| meta.set_record("Standard:B", Some(record("b.pak"))));
    assert_eq!(store.json()["revision"], 2);
    assert_eq!(store.store.load().meta.revision, 2);
}

#[test]
fn entries_without_fields_are_removed() {
    let store = Store::new();
    fs::write(
        store.file(),
        r#"{"version":1,"revision":1,"mods":{"Standard:Empty":{}}}"#,
    )
    .expect("write");
    store.update(|meta| {
        meta.set_record("Standard:A", Some(record("a.pak")));
        meta.set_record("Standard:A", None);
    });
    let json = store.json();
    assert_eq!(json["mods"], serde_json::json!({}));
}
