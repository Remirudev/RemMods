#![cfg(windows)]
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod game_support;
mod support;

use game_support::{MANIFEST, install_game, make_steam_root, steam_text, text_of};
use remmods_core::game::{SteamKind, detect};
use remmods_core::platform::{
    OsKind, RegistryHive, RegistrySteamValues, RegistryView, SteamHints, processes,
    registry_string, save_games_root, steam_hints,
};

#[test]
fn saves_live_in_local_app_data_wherever_the_game_is() {
    let root = save_games_root(std::path::Path::new(r"D:\Games\TEKKEN 8"), 1_778_820)
        .expect("local app data");
    assert!(
        root.ends_with(r"TEKKEN 8\Saved\SaveGames"),
        "{}",
        root.display()
    );
    assert!(!root.starts_with(r"D:\Games"), "{}", root.display());
}

#[test]
fn own_process_is_listed_by_exe_name() {
    let exe = std::env::current_exe().expect("current exe");
    let name = exe
        .file_name()
        .and_then(|name| name.to_str())
        .expect("exe name");
    let records = processes().expect("Toolhelp32 snapshot");
    let own = records
        .iter()
        .find(|record| record.pid == std::process::id())
        .expect("own pid listed");
    assert!(
        own.name.eq_ignore_ascii_case(name),
        "{} vs {name}",
        own.name
    );
    assert!(own.argv.is_empty());
}

#[test]
fn registry_reads_strings_and_misses_cleanly() {
    let build = registry_string(
        RegistryHive::LocalMachine,
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        "CurrentBuild",
        RegistryView::Default,
    )
    .expect("registry read");
    assert!(build.is_some_and(|value| !value.is_empty()));
    let missing = registry_string(
        RegistryHive::CurrentUser,
        r"Software\RemMods\NoSuchKey",
        "Nothing",
        RegistryView::Default,
    )
    .expect("missing key is not an error");
    assert_eq!(missing, None);
    let _ = steam_hints();
}

#[test]
fn lowercase_registry_path_gets_the_on_disk_spelling() {
    let (_dir, base) = support::temp_root();
    let root = base.join("MixedCase Steam");
    make_steam_root(&root);
    let game = install_game(&root, &steam_text(MANIFEST));
    let registry_value = text_of(&root).to_lowercase().replace('\\', "/");
    let hints = SteamHints {
        os: OsKind::Windows,
        home: None,
        data_dir: None,
        registry: RegistrySteamValues {
            steam_path: Some(registry_value),
            ..RegistrySteamValues::default()
        },
        program_files_x86: None,
        program_files: None,
    };
    let detection = detect(&hints, 1);
    assert_eq!(detection.candidates.len(), 1);
    assert_eq!(detection.candidates[0].root, game);
    assert_eq!(detection.candidates[0].steam_kind, SteamKind::Windows);
}
