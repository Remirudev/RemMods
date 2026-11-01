//! A `Core` on a TempDir game with an injected process list, for the toggle tests (spec §8.2).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use remmods_core::game::{GameOptions, ProcessSource};
use remmods_core::mods::{ModRow, ScanSnapshot, ToggleOutcome, ToggleRequest, ToggleTarget};
use remmods_core::platform::{ProcessRecord, TrashEnv};
use remmods_core::{AppPaths, Core, CoreError, CoreOptions};
use tempfile::TempDir;

use super::{listing, paks_tree, paks_under};

pub const MARKER: &str = "remmods-smoke-fixture";

pub struct ToggleFixture {
    _dir: TempDir,
    pub root: PathBuf,
    pub game_root: PathBuf,
    pub paks: PathBuf,
    pub paths: AppPaths,
    /// Flip to make the injected process list show the game.
    pub running: Arc<AtomicBool>,
    pub core: Core,
    /// Options every `Core` of this fixture gets (trash env, original app dir).
    pub extras: Extras,
}

/// The P3 options of the fixture's cores; `Default` = `CoreOptions::default()`'s.
#[derive(Clone, Debug, Default)]
pub struct Extras {
    pub marker: Option<String>,
    pub trash: Option<TrashEnv>,
    pub tmm_dir: Option<PathBuf>,
}

impl ToggleFixture {
    pub fn new(entries: &[(&str, &[u8])]) -> Self {
        Self::build(entries, |_| Extras::default())
    }

    /// Mutations need `<Paks>/remmods-smoke-fixture` (A9).
    pub fn requiring_marker(entries: &[(&str, &[u8])]) -> Self {
        Self::build(entries, |_| Extras {
            marker: Some(MARKER.to_owned()),
            ..Extras::default()
        })
    }

    /// `extras` gets the fixture's root (a canonical TempDir) to place a trash or folders in.
    pub fn build(entries: &[(&str, &[u8])], extras: impl FnOnce(&Path) -> Extras) -> Self {
        let (dir, root) = crate::support::temp_root();
        let extras = extras(&root);
        let out = root.join("game");
        paks_tree(
            &out,
            &[
                ("steamapps/common/TEKKEN 8/TEKKEN 8.exe", b""),
                (
                    "steamapps/common/TEKKEN 8/Polaris/Binaries/Win64/Polaris-Win64-Shipping.exe",
                    b"",
                ),
                ("steamapps/common/TEKKEN 8/Polaris/Content/Paks/", b""),
            ],
        );
        let paks = paks_under(&out);
        paks_tree(&paks, entries);
        let game_root = out.join("steamapps").join("common").join("TEKKEN 8");
        let paths = AppPaths::under(&root.join("app"));
        let running = Arc::new(AtomicBool::new(false));
        let core = open_core(&paths, &game_root, &running, &extras);
        Self {
            _dir: dir,
            root,
            game_root,
            paks,
            paths,
            running,
            core,
            extras,
        }
    }

    /// A second `Core` on the same app folders and game (another process in the real app).
    pub fn second_core(&self) -> Core {
        let extras = Extras {
            marker: None,
            ..self.extras.clone()
        };
        open_core(&self.paths, &self.game_root, &self.running, &extras)
    }

    pub fn snapshot(&self) -> Arc<ScanSnapshot> {
        self.core.mods_refresh().expect("mods_refresh").snapshot
    }

    pub fn row(&self, key: &str) -> ModRow {
        let snapshot = self.snapshot();
        snapshot
            .content
            .rows
            .iter()
            .find(|row| row.key == key)
            .cloned()
            .unwrap_or_else(|| panic!("no row {key}"))
    }

    pub fn target(&self, key: &str) -> ToggleTarget {
        let row = self.row(key);
        ToggleTarget {
            mod_ref: row.mod_ref,
            key: row.key,
        }
    }

    pub fn toggle(&self, keys: &[&str], enable: bool) -> Result<ToggleOutcome, CoreError> {
        let targets = keys.iter().map(|key| self.target(key)).collect();
        self.core.mods_toggle(&ToggleRequest {
            targets,
            enable,
            apply_while_running: false,
        })
    }

    /// Every entry below Paks as (rel path, kind, length); renames keep all three for files.
    pub fn names(&self) -> Vec<(String, char, u64)> {
        names_below(&self.paks)
    }

    pub fn exists(&self, rel: &str) -> bool {
        let path = rel
            .split('/')
            .fold(self.paks.clone(), |path, part| path.join(part));
        remmods_core::platform::fs_path(&path)
            .and_then(std::fs::symlink_metadata)
            .is_ok()
    }
}

pub fn names_below(root: &Path) -> Vec<(String, char, u64)> {
    listing(root)
        .into_iter()
        .map(|(rel, kind, len, _)| (rel, kind, if kind == 'd' { 0 } else { len }))
        .collect()
}

fn open_core(
    paths: &AppPaths,
    game_root: &Path,
    running: &Arc<AtomicBool>,
    extras: &Extras,
) -> Core {
    let flag = Arc::clone(running);
    let processes = ProcessSource(Arc::new(move || {
        Ok(if flag.load(Ordering::SeqCst) {
            vec![ProcessRecord {
                pid: 4242,
                name: "TEKKEN 8.exe".to_owned(),
                argv: Vec::new(),
            }]
        } else {
            Vec::new()
        })
    }));
    let options = CoreOptions {
        game: GameOptions {
            steam_detection: false,
            fixed_location: Some(game_root.to_path_buf()),
            steam_hints: None,
            processes: Some(processes),
        },
        require_marker: extras.marker.clone(),
        trash: extras
            .trash
            .clone()
            .unwrap_or_else(|| CoreOptions::default().trash),
        tmm_dir: extras.tmm_dir.clone(),
    };
    Core::open_with(paths.clone(), options).expect("open core")
}
