#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod game_support;
mod support;

use std::fs;
use std::path::{Path, PathBuf};

use game_support::{
    MANIFEST, install_game, linux_hints, make_game, make_steam_root, manifest_with, steam_text,
    text_of, under,
};
use remmods_core::game::{
    FolderProblem, GameError, GameLocation, GameOptions, GameStatus, InstallInfo, LocationChoice,
    LocationSource, PickResult, STEAM_LAUNCH_URL, SteamKind, UpdateNotice, UpdateTracking,
};
use remmods_core::settings::{LastSeenBuild, StoredLocation, StoredSource};
use remmods_core::{AppPaths, Core, CoreError, CoreOptions};
use tempfile::TempDir;

struct Fixture {
    _dir: TempDir,
    root: PathBuf,
    home: PathBuf,
    paths: AppPaths,
}

fn fixture() -> Fixture {
    let (dir, root) = support::temp_root();
    let home = root.join("home");
    fs::create_dir_all(&home).expect("home");
    Fixture {
        paths: AppPaths::under(&root.join("app")),
        home,
        root,
        _dir: dir,
    }
}

impl Fixture {
    fn steam_root(&self) -> PathBuf {
        under(&self.home, &[".local", "share", "Steam"])
    }

    fn core(&self, detection: bool) -> Core {
        let options = CoreOptions {
            game: GameOptions {
                steam_detection: detection,
                fixed_location: None,
                steam_hints: Some(linux_hints(&self.home)),
            },
        };
        Core::open_with(self.paths.clone(), options).expect("open core")
    }

    fn store(&self, location: Option<StoredLocation>, build: Option<LastSeenBuild>) {
        let store = remmods_core::settings::SettingsStore::load(&self.paths);
        store
            .update_game(|game| {
                game.location = location;
                game.last_seen_build = build;
            })
            .expect("store game settings");
    }

    fn stored(&self) -> remmods_core::settings::GameSettings {
        remmods_core::settings::SettingsStore::load(&self.paths)
            .snapshot()
            .settings
            .game
    }

    /// A second Steam root (`~/Steam`) with its own install.
    fn second_install(&self) -> PathBuf {
        let other = self.home.join("Steam");
        make_steam_root(&other);
        install_game(&other, &steam_text(MANIFEST))
    }
}

fn ready(status: &GameStatus) -> &InstallInfo {
    match &status.location {
        GameLocation::Ready(install) => install,
        other => panic!("expected Ready, got {other:?}"),
    }
}

fn manual(path: &Path) -> Option<StoredLocation> {
    Some(StoredLocation {
        game_root: text_of(path),
        source: StoredSource::Manual,
    })
}

#[test]
fn one_valid_candidate_is_selected_and_stored() {
    let fx = fixture();
    let game = install_game(&fx.steam_root(), &steam_text(MANIFEST));
    let core = fx.core(true);
    let status = core.game_status();
    let install = ready(&status);
    assert_eq!(install.root, game);
    assert_eq!(install.source, LocationSource::Steam);
    assert_eq!(install.steam_kind, Some(SteamKind::Native));
    assert_eq!(install.build_id.as_deref(), Some("25221600"));
    assert_eq!(install.update_tracking, UpdateTracking::Available);
    assert_eq!(status.candidates.len(), 1);
    assert!(status.candidates[0].in_use);
    assert_eq!(
        fx.stored().location,
        Some(StoredLocation {
            game_root: text_of(&game),
            source: StoredSource::Steam
        })
    );
    assert_eq!(core.game_launch_plan().url, STEAM_LAUNCH_URL);
    assert_eq!(core.game_launch_plan().steam_kind, Some(SteamKind::Native));
    let paks = core.game_paks().expect("paks");
    assert_eq!(paks.paks, install.paks);
    assert_eq!(core.game_reveal_target().expect("reveal"), install.paks);
}

#[test]
fn two_candidates_leave_it_unset_until_one_is_chosen() {
    let fx = fixture();
    install_game(&fx.steam_root(), &steam_text(MANIFEST));
    let second = fx.second_install();
    let core = fx.core(true);
    let status = core.game_status();
    assert_eq!(status.location, GameLocation::NotSet);
    assert_eq!(status.candidates.len(), 2);
    assert!(core.game_paks().is_none());
    assert!(matches!(
        core.game_reveal_target(),
        Err(CoreError::Game(GameError::NotSet))
    ));
    let id = status
        .candidates
        .iter()
        .find(|candidate| candidate.root == second)
        .expect("second candidate")
        .id
        .clone();
    let chosen = core
        .game_set_location(LocationChoice::Candidate { id })
        .expect("set candidate");
    assert_eq!(ready(&chosen).root, second);
    assert_eq!(ready(&chosen).source, LocationSource::Steam);
    let in_use: Vec<bool> = chosen.candidates.iter().map(|c| c.in_use).collect();
    assert_eq!(in_use.iter().filter(|used| **used).count(), 1);
    for candidate in &chosen.candidates {
        assert_eq!(candidate.in_use, candidate.root == second);
    }
}

#[test]
fn stale_ids_and_bad_paths_are_errors() {
    let fx = fixture();
    install_game(&fx.steam_root(), &steam_text(MANIFEST));
    fx.second_install();
    let core = fx.core(true);
    let old_id = core.game_status().candidates[0].id.clone();
    core.game_detect();
    let stale = core.game_set_location(LocationChoice::Candidate { id: old_id });
    assert!(matches!(
        stale,
        Err(CoreError::Game(GameError::StaleCandidate))
    ));
    let relative = core.game_set_location(LocationChoice::Path {
        path: PathBuf::from("relative/TEKKEN 8"),
    });
    assert!(matches!(
        relative,
        Err(CoreError::Game(GameError::InvalidPath))
    ));
    let empty = core.game_set_location(LocationChoice::Path {
        path: PathBuf::from("  "),
    });
    assert!(matches!(
        empty,
        Err(CoreError::Game(GameError::InvalidPath))
    ));
    let not_game = core.game_set_location(LocationChoice::Path {
        path: fx.home.clone(),
    });
    assert!(matches!(
        not_game,
        Err(CoreError::Game(GameError::InvalidFolder(
            FolderProblem::NotFound
        )))
    ));
}

#[test]
fn incomplete_install_is_not_selected() {
    let fx = fixture();
    install_game(&fx.steam_root(), &manifest_with("25221600", "1026"));
    let status = fx.core(true).game_status();
    assert_eq!(status.location, GameLocation::NotSet);
    assert_eq!(status.candidates.len(), 1);
}

#[test]
fn stored_location_wins_and_a_gone_one_is_missing() {
    let fx = fixture();
    install_game(&fx.steam_root(), &steam_text(MANIFEST));
    let own = fx.root.join("Own").join("TEKKEN 8");
    make_game(&own);
    fx.store(manual(&own), None);
    let status = fx.core(true).game_status();
    assert_eq!(ready(&status).root, own);
    assert_eq!(ready(&status).source, LocationSource::Manual);
    assert_eq!(ready(&status).steam_kind, None);
    assert_eq!(ready(&status).update_tracking, UpdateTracking::NoManifest);

    let fx = fixture();
    let gone = fx.root.join("Gone");
    fx.store(manual(&gone), None);
    let status = fx.core(true).game_status();
    assert_eq!(status.location, GameLocation::Missing { path: gone });
}

#[test]
fn choose_path_accepts_a_file_inside_paks() {
    let fx = fixture();
    let game = install_game(&fx.steam_root(), &steam_text(MANIFEST));
    let file = under(&game, &["Polaris", "Content", "Paks", "Mods"]);
    fs::create_dir_all(&file).expect("Mods");
    let file = file.join("x.pak");
    fs::write(&file, b"pak").expect("pak");
    let core = fx.core(false);
    let PickResult::Selected(status) = core.game_choose_path(&file).expect("choose") else {
        panic!("expected Selected");
    };
    assert_eq!(ready(&status).root, game);
    assert_eq!(ready(&status).source, LocationSource::Manual);
    let invalid = core.game_choose_path(&fx.home).expect("choose");
    assert_eq!(
        invalid,
        PickResult::Invalid {
            problem: FolderProblem::NotFound,
            path: fx.home.clone()
        }
    );
}

#[test]
fn fixed_location_is_ready_and_not_stored() {
    let fx = fixture();
    let game = fx.root.join("Fixture").join("TEKKEN 8");
    make_game(&game);
    let options = CoreOptions {
        game: GameOptions {
            steam_detection: true,
            fixed_location: Some(under(&game, &["Polaris", "Content", "Paks"])),
            steam_hints: Some(linux_hints(&fx.home)),
        },
    };
    let core = Core::open_with(fx.paths.clone(), options).expect("open");
    let status = core.game_status();
    assert_eq!(ready(&status).root, game);
    assert_eq!(ready(&status).source, LocationSource::Fixture);
    assert!(status.candidates.is_empty());
    assert_eq!(fx.stored().location, None);
}

#[test]
fn detection_off_reads_no_steam() {
    let fx = fixture();
    install_game(&fx.steam_root(), &steam_text(MANIFEST));
    let core = fx.core(false);
    assert_eq!(core.game_status().location, GameLocation::NotSet);
    let status = core.game_detect();
    assert!(status.candidates.is_empty());
    assert!(status.notes.is_empty());
}

#[test]
fn stored_location_in_a_flatpak_library() {
    let fx = fixture();
    let flatpak = under(
        &fx.home,
        &[
            ".var",
            "app",
            "com.valvesoftware.Steam",
            ".local",
            "share",
            "Steam",
        ],
    );
    let game = install_game(&flatpak, &steam_text(MANIFEST));
    fx.store(manual(&game), None);
    let core = fx.core(false);
    assert_eq!(
        ready(&core.game_status()).steam_kind,
        Some(SteamKind::Flatpak)
    );
    assert_eq!(core.game_launch_plan().steam_kind, Some(SteamKind::Flatpak));
}

#[test]
fn renamed_game_folder_goes_missing_and_comes_back() {
    let fx = fixture();
    let game = install_game(&fx.steam_root(), &steam_text(MANIFEST));
    let core = fx.core(true);
    assert_eq!(ready(&core.game_status()).root, game);
    let generation = core.game_paks().expect("paks").generation;
    let moved = game.with_file_name("TEKKEN 8 moved");
    fs::rename(&game, &moved).expect("rename away");
    let status = core.game_detect();
    assert_eq!(
        status.location,
        GameLocation::Missing { path: game.clone() }
    );
    fs::rename(&moved, &game).expect("rename back");
    let status = core.game_detect();
    assert_eq!(ready(&status).root, game);
    assert!(core.game_paks().expect("paks").generation > generation);
}

#[test]
fn first_run_stores_the_build_silently_then_updates_notify() {
    let fx = fixture();
    let game = install_game(&fx.steam_root(), &steam_text(MANIFEST));
    let core = fx.core(true);
    assert_eq!(core.game_status().update, None);
    assert_eq!(
        fx.stored().last_seen_build,
        Some(LastSeenBuild {
            game_root: text_of(&game),
            build_id: "25221600".to_owned()
        })
    );
    assert_eq!(core.game_refresh_build(), None);
    let manifest = fx.steam_root().join("steamapps").join(MANIFEST);
    fs::write(&manifest, manifest_with("26410000", "4")).expect("new build");
    let change = core.game_refresh_build().expect("update change");
    let notice = UpdateNotice {
        from: "25221600".to_owned(),
        to: "26410000".to_owned(),
    };
    assert_eq!(change.update, Some(notice.clone()));
    assert_eq!(core.game_status().update, Some(notice));
    let acked = core.game_ack_update().expect("ack");
    assert_eq!(acked.update, None);
    assert_eq!(
        fx.stored().last_seen_build.map(|build| build.build_id),
        Some("26410000".to_owned())
    );
    assert_eq!(core.game_refresh_build(), None);
}

#[test]
fn another_spelling_of_the_same_root_is_not_a_first_run() {
    let fx = fixture();
    let game = install_game(&fx.steam_root(), &steam_text(MANIFEST));
    let spelled = game.join("..").join("TEKKEN 8");
    fx.store(
        Some(StoredLocation {
            game_root: text_of(&game),
            source: StoredSource::Steam,
        }),
        Some(LastSeenBuild {
            game_root: text_of(&spelled),
            build_id: "100".to_owned(),
        }),
    );
    let status = fx.core(true).game_status();
    assert_eq!(
        status.update,
        Some(UpdateNotice {
            from: "100".to_owned(),
            to: "25221600".to_owned()
        })
    );
}
