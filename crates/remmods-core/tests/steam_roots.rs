#![allow(clippy::expect_used)]

mod game_support;
mod support;

use std::path::{Path, PathBuf};

use game_support::under;
use remmods_core::game::{
    SteamKind, flatpak_library_path, linux_root_candidates, steam_kind_of, windows_root_candidates,
};
use remmods_core::platform::{OsKind, RegistrySteamValues};

fn flatpak(home: &Path) -> PathBuf {
    under(home, &[".var", "app", "com.valvesoftware.Steam"])
}

#[test]
fn linux_candidates_in_order() {
    let (_dir, home) = support::temp_root();
    let data = home.join("data");
    let expected = vec![
        under(&home, &[".steam", "steam"]),
        under(&home, &[".steam", "root"]),
        data.join("Steam"),
        under(&home, &[".local", "share", "Steam"]),
        under(&home, &[".steam", "debian-installation"]),
        home.join("Steam"),
        under(&flatpak(&home), &[".local", "share", "Steam"]),
        under(&flatpak(&home), &["data", "Steam"]),
        under(
            &home,
            &["snap", "steam", "common", ".local", "share", "Steam"],
        ),
        under(
            &home,
            &[
                ".snap", "data", "steam", "common", ".local", "share", "Steam",
            ],
        ),
    ];
    assert_eq!(linux_root_candidates(&home, Some(&data)), expected);
}

#[test]
fn linux_fourth_candidate_dropped_when_equal_to_third() {
    let (_dir, home) = support::temp_root();
    let data = under(&home, &[".local", "share"]);
    let roots = linux_root_candidates(&home, Some(&data));
    assert_eq!(roots.len(), 9);
    assert_eq!(roots[2], under(&home, &[".local", "share", "Steam"]));
    assert_eq!(roots[3], under(&home, &[".steam", "debian-installation"]));
    assert_eq!(linux_root_candidates(&home, None).len(), 9);
}

#[test]
fn windows_candidates_in_order_without_duplicates() {
    let registry = RegistrySteamValues {
        steam_path: Some("c:/program files (x86)/steam".to_owned()),
        install_path_32: Some("C:/Program Files (x86)/Steam".to_owned()),
        install_path_64: Some("D:\\Steam".to_owned()),
        errors: Vec::new(),
    };
    let roots = windows_root_candidates(
        &registry,
        Some(Path::new("E:\\PF86")),
        Some(Path::new("E:\\PF")),
    );
    assert_eq!(
        roots,
        [
            PathBuf::from("c:/program files (x86)/steam"),
            PathBuf::from("D:\\Steam"),
            Path::new("E:\\PF86").join("Steam"),
            Path::new("E:\\PF").join("Steam"),
        ]
    );
    let empty = RegistrySteamValues {
        steam_path: Some(String::new()),
        ..RegistrySteamValues::default()
    };
    assert!(windows_root_candidates(&empty, None, None).is_empty());
}

#[test]
fn steam_kind_by_prefix() {
    let (_dir, home) = support::temp_root();
    let kind = |root: PathBuf| steam_kind_of(&root, Some(&home), OsKind::Linux);
    assert_eq!(
        kind(under(&flatpak(&home), &[".local", "share", "Steam"])),
        SteamKind::Flatpak
    );
    assert_eq!(
        kind(under(
            &home,
            &["snap", "steam", "common", ".local", "share", "Steam"]
        )),
        SteamKind::Snap
    );
    assert_eq!(
        kind(under(
            &home,
            &[
                ".snap", "data", "steam", "common", ".local", "share", "Steam"
            ]
        )),
        SteamKind::Snap
    );
    assert_eq!(
        kind(under(&home, &[".local", "share", "Steam"])),
        SteamKind::Native
    );
    assert_eq!(kind(home.join("Steam")), SteamKind::Native);
    assert_eq!(
        steam_kind_of(&home.join("Steam"), Some(&home), OsKind::Windows),
        SteamKind::Windows
    );
    assert_eq!(
        steam_kind_of(&home.join("Steam"), None, OsKind::Linux),
        SteamKind::Native
    );
}

#[test]
fn flatpak_fixup_rules() {
    let (_dir, home) = support::temp_root();
    let inside = under(&flatpak(&home), &[".local", "share", "Steam"]);
    let none = |_: &Path| false;
    // (b) the sandbox's own views of the root
    for view in [
        under(&home, &[".local", "share", "Steam"]),
        under(&home, &[".steam", "steam"]),
        under(&home, &[".steam", "root"]),
    ] {
        assert_eq!(
            flatpak_library_path(&view, &home, &none),
            Some(inside.clone())
        );
    }
    // (c) another path under home: itself, else inside the sandbox, else missing
    let games = home.join("Games");
    let games_inside = flatpak(&home).join("Games");
    let only_games = |path: &Path| path == games;
    assert_eq!(
        flatpak_library_path(&games, &home, &only_games),
        Some(games.clone())
    );
    let only_inside = |path: &Path| path == games_inside;
    assert_eq!(
        flatpak_library_path(&games, &home, &only_inside),
        Some(games_inside.clone())
    );
    assert_eq!(flatpak_library_path(&games, &home, &none), None);
    // (d) outside home or under .var/app: as is
    let outside = Path::new("/mnt/library").to_path_buf();
    assert_eq!(
        flatpak_library_path(&outside, &home, &none),
        Some(outside.clone())
    );
    let var_app = under(&flatpak(&home), &["data", "Steam"]);
    assert_eq!(
        flatpak_library_path(&var_app, &home, &none),
        Some(var_app.clone())
    );
}
