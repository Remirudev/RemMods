#![cfg(target_os = "linux")]
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod game_support;
mod support;

use std::ffi::OsString;
use std::fs;
use std::io;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::symlink;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use game_support::{MANIFEST, install_game, linux_hints, make_game_named, steam_text, under};
use remmods_core::AppPaths;
use remmods_core::game::{
    FolderProblem, NoteCode, SteamKind, detect, game_matches, validate_game_root,
};
use remmods_core::graphics::{self, GraphicsPaths};
use remmods_core::platform::{host_command, host_env_plan, processes, spawn_detached};
use remmods_core::safe_fs::{self, FsError};

#[test]
fn own_process_is_listed_with_its_argv() {
    let records = processes().expect("read /proc");
    let own = records
        .iter()
        .find(|record| record.pid == std::process::id())
        .expect("own pid listed");
    assert_eq!(own.argv.first(), std::env::args().next().as_ref());
    assert!(records.windows(2).all(|pair| pair[0].pid < pair[1].pid));
}

#[test]
fn a_process_named_like_the_game_is_found() {
    let mut child = Command::new("sleep")
        .arg0(r"S:\steamapps\common\TEKKEN 8\TEKKEN 8.exe")
        .arg("30")
        .spawn()
        .expect("spawn sleep");
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut found = false;
    while Instant::now() < deadline && !found {
        let records = processes().expect("read /proc");
        found = game_matches(&records)
            .iter()
            .any(|matched| matched.pid == child.id());
        if !found {
            thread::sleep(Duration::from_millis(50));
        }
    }
    child.kill().expect("kill sleep");
    child.wait().expect("reap sleep");
    assert!(found, "the fake game process was not matched");
}

#[test]
fn host_command_runs_in_home() {
    let command = host_command("true");
    assert_eq!(command.get_current_dir(), dirs::home_dir().as_deref());
}

#[test]
fn spawn_detached_reports_early_failures() {
    spawn_detached(host_command("true")).expect("true succeeds");
    assert!(spawn_detached(host_command("false")).is_err());
    let missing =
        spawn_detached(host_command("remmods-no-such-program")).expect_err("missing program");
    assert_eq!(missing.kind(), io::ErrorKind::NotFound);
}

/// Spec P5 H10: non-UTF-8 entries outside the mount survive byte-exact.
#[test]
fn host_env_keeps_non_utf8_entries_byte_exact() {
    let path = OsString::from_vec(b"/tmp/.mount_x/usr/bin:/opt/\xff\xfe/bin:/usr/bin".to_vec());
    let lib = OsString::from_vec(b"/tmp/.mount_x/lib:/home/u/\xc3(lib".to_vec());
    let env = vec![
        (OsString::from("APPDIR"), OsString::from("/tmp/.mount_x")),
        (OsString::from("PATH"), path),
        (OsString::from("LD_LIBRARY_PATH"), lib),
    ];
    let plan = host_env_plan(&env, &[]);
    let value = |name: &str| {
        plan.set
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_bytes().to_vec())
    };
    assert_eq!(value("PATH"), Some(b"/opt/\xff\xfe/bin:/usr/bin".to_vec()));
    assert_eq!(value("LD_LIBRARY_PATH"), Some(b"/home/u/\xc3(lib".to_vec()));
}

/// Spec P5 G19: a symlink in an app folder is never removed (nor its target).
#[test]
fn remove_app_file_refuses_a_symlink() {
    let (_dir, root) = support::temp_root();
    let app = AppPaths::under(&root);
    fs::create_dir_all(&app.config_dir).expect("config dir");
    let target = root.join("target.txt");
    fs::write(&target, b"keep").expect("target");
    let link = app.config_dir.join("graphics-pending");
    symlink(&target, &link).expect("symlink");
    let error = safe_fs::remove_app_file(&link, &[&app.config_dir]).expect_err("a link");
    assert!(matches!(error, FsError::Io { .. }));
    let paths = GraphicsPaths::from_app(&app);
    assert!(graphics::clear_marker(&paths).is_err());
    assert!(
        fs::symlink_metadata(&link)
            .expect("link stays")
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read(&target).expect("target stays"), b"keep");
}

#[test]
fn symlinked_steam_root_dedupes_to_one() {
    let (_dir, home) = support::temp_root();
    let real = under(&home, &[".local", "share", "Steam"]);
    install_game(&real, &steam_text(MANIFEST));
    fs::create_dir_all(home.join(".steam")).expect(".steam");
    symlink(&real, under(&home, &[".steam", "steam"])).expect("symlink");
    let detection = detect(&linux_hints(&home), 1);
    let roots: Vec<_> = detection
        .notes
        .iter()
        .filter(|note| note.code == NoteCode::SteamRoot)
        .collect();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].path, under(&home, &[".steam", "steam"]));
    assert_eq!(detection.candidates.len(), 1);
}

#[test]
fn symlink_into_the_flatpak_has_the_flatpak_kind() {
    let (_dir, home) = support::temp_root();
    let flatpak = under(
        &home,
        &[
            ".var",
            "app",
            "com.valvesoftware.Steam",
            ".local",
            "share",
            "Steam",
        ],
    );
    install_game(&flatpak, &steam_text(MANIFEST));
    fs::create_dir_all(home.join(".steam")).expect(".steam");
    symlink(&flatpak, under(&home, &[".steam", "steam"])).expect("symlink");
    let detection = detect(&linux_hints(&home), 1);
    assert_eq!(detection.candidates.len(), 1);
    assert_eq!(detection.candidates[0].steam_kind, SteamKind::Flatpak);
}

#[test]
fn library_folder_spelled_steam_apps_is_found() {
    let (_dir, home) = support::temp_root();
    let root = under(&home, &[".local", "share", "Steam"]);
    let steamapps = root.join("SteamApps");
    let game = steamapps.join("common").join("TEKKEN 8");
    make_game_named(
        &game,
        "Polaris",
        "Content",
        "Paks",
        "Binaries",
        "Win64",
        "Polaris-Win64-Shipping.exe",
    );
    fs::write(steamapps.join(MANIFEST), steam_text(MANIFEST)).expect("manifest");
    let detection = detect(&linux_hints(&home), 1);
    assert_eq!(detection.candidates.len(), 1);
    assert_eq!(detection.candidates[0].root, game);
}

#[test]
fn exact_case_wins_and_two_folded_matches_are_ambiguous() {
    let (_dir, root) = support::temp_root();
    let exact = root.join("Exact");
    make_game_named(
        &exact,
        "Polaris",
        "Content",
        "Paks",
        "Binaries",
        "Win64",
        "Polaris-Win64-Shipping.exe",
    );
    fs::create_dir_all(exact.join("polaris")).expect("decoy");
    let layout = validate_game_root(&exact).expect("exact wins");
    assert_eq!(layout.paks, under(&exact, &["Polaris", "Content", "Paks"]));

    let ambiguous = root.join("Ambiguous");
    make_game_named(
        &ambiguous,
        "polaris",
        "Content",
        "Paks",
        "Binaries",
        "Win64",
        "Polaris-Win64-Shipping.exe",
    );
    fs::create_dir_all(ambiguous.join("POLARIS")).expect("decoy");
    assert_eq!(
        validate_game_root(&ambiguous),
        Err(FolderProblem::Ambiguous)
    );
}
