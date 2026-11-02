//! Folder picks for `resolve_pick`: game root, Paks, mod roots and unrelated folders.
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod game_support;
mod support;

use std::fs;
use std::path::{Path, PathBuf};

use game_support::{MANIFEST, make_game, make_game_named, steam_fixture, under};
use remmods_core::game::{
    FolderProblem, GameLayout, game_view_of_paks, manifest_for_root, resolve_pick,
};
use remmods_core::platform::OsKind;
use tempfile::TempDir;

const SHIPPING_EXE: [&str; 4] = ["Polaris", "Binaries", "Win64", "Polaris-Win64-Shipping.exe"];

/// A Steam library with the game, its manifest and a few mod folders.
struct Library {
    _dir: TempDir,
    root: PathBuf,
    game: PathBuf,
}

impl Library {
    fn new() -> Self {
        let (dir, root) = support::temp_root();
        let steamapps = root.join("steamapps");
        let game = steamapps.join("common").join("TEKKEN 8");
        make_game(&game);
        fs::write(steamapps.join(MANIFEST), steam_fixture(MANIFEST)).expect("manifest");
        let extra = under(&paks_of(&game), &["Mods", "Kazuya", "extra"]);
        fs::create_dir_all(&extra).expect("mod folder");
        fs::write(extra.join("notes.txt"), b"kazuya notes").expect("mod file");
        fs::create_dir_all(paks_of(&game).join("LogicMods")).expect("LogicMods");
        Self {
            _dir: dir,
            root,
            game,
        }
    }

    fn paks(&self) -> PathBuf {
        paks_of(&self.game)
    }
}

fn paks_of(game: &Path) -> PathBuf {
    under(game, &["Polaris", "Content", "Paks"])
}

fn resolved(pick: &Path) -> Result<PathBuf, FolderProblem> {
    resolve_pick(pick).map(|layout| layout.root)
}

#[test]
fn picks_inside_the_game_resolve_to_its_root() {
    let library = Library::new();
    let game = &library.game;
    let paks = library.paks();
    let picks = [
        game.clone(),
        under(game, &["Polaris", "Content"]),
        under(game, &SHIPPING_EXE),
        paks.clone(),
        paks.join("Mods"),
        paks.join("LogicMods"),
        under(&paks, &["Mods", "Kazuya", "extra", "notes.txt"]),
    ];
    for pick in picks {
        assert_eq!(resolved(&pick), Ok(game.clone()), "pick {}", pick.display());
    }
    let mut with_separator = paks.into_os_string();
    with_separator.push(std::path::MAIN_SEPARATOR_STR);
    assert_eq!(resolved(Path::new(&with_separator)), Ok(game.clone()));
}

#[test]
fn layout_has_plain_paths_with_on_disk_names() {
    let library = Library::new();
    let layout = resolve_pick(&library.paks().join("Mods")).expect("resolves");
    assert_eq!(
        layout,
        GameLayout {
            root: library.game.clone(),
            paks: library.paks(),
            exe: under(&library.game, &SHIPPING_EXE),
        }
    );
}

#[test]
fn lowercase_tree_stores_on_disk_names() {
    let (_dir, root) = support::temp_root();
    let game = root.join("tekken");
    make_game_named(
        &game,
        "polaris",
        "content",
        "paks",
        "binaries",
        "win64",
        "polaris-win64-shipping.exe",
    );
    let layout = resolve_pick(&game.join("polaris")).expect("resolves");
    assert_eq!(layout.root, game);
    assert_eq!(layout.paks, under(&game, &["polaris", "content", "paks"]));
    assert_eq!(
        layout.exe,
        under(
            &game,
            &["polaris", "binaries", "win64", "polaris-win64-shipping.exe"]
        )
    );
}

#[test]
fn unrelated_picks_are_not_found() {
    let library = Library::new();
    let root = &library.root;
    let downloads = root.join("Downloads");
    let old_paks = under(root, &["Backups", "Old", "Paks"]);
    let loose_mods = under(root, &["Backups", "Mods"]);
    for dir in [&downloads, &old_paks, &loose_mods] {
        fs::create_dir_all(dir).expect("create");
    }
    for pick in [downloads, old_paks, loose_mods, root.join("Backups")] {
        assert_eq!(
            resolved(&pick),
            Err(FolderProblem::NotFound),
            "{}",
            pick.display()
        );
    }
}

#[test]
fn nine_levels_deep_is_not_found() {
    let library = Library::new();
    let ninth = under(
        &library.paks(),
        &["Backups", "2026", "spring", "week1", "day3"],
    );
    fs::create_dir_all(&ninth).expect("deep folders");
    assert_eq!(resolved(&ninth), Err(FolderProblem::NotFound));
    let eighth = ninth.parent().expect("parent");
    assert_eq!(resolved(eighth), Ok(library.game.clone()));
}

#[test]
fn half_a_game_is_not_found() {
    let (_dir, root) = support::temp_root();
    let paks_only = root.join("PaksOnly");
    fs::create_dir_all(paks_of(&paks_only)).expect("paks");
    assert_eq!(resolved(&paks_only), Err(FolderProblem::NotFound));
    let exe_only = root.join("ExeOnly");
    let exe = under(&exe_only, &SHIPPING_EXE);
    fs::create_dir_all(exe.parent().expect("exe folder")).expect("exe folder");
    fs::write(&exe, b"MZ").expect("exe");
    assert_eq!(resolved(&exe_only), Err(FolderProblem::NotFound));
}

#[test]
fn relative_pick_becomes_absolute() {
    let library = Library::new();
    let cwd = std::env::current_dir().expect("cwd");
    let relative = pathdiff(&library.paks(), &cwd);
    // Windows runners keep %TEMP% and the checkout on different drives, where no relative path
    // exists; the Linux run covers this case.
    if relative.is_absolute() {
        return;
    }
    assert_eq!(resolved(&relative), Ok(library.game.clone()));
}

#[test]
fn forward_slashes_are_normalised() {
    let library = Library::new();
    let text = library
        .paks()
        .join("Mods")
        .to_str()
        .expect("UTF-8")
        .replace('\\', "/");
    assert_eq!(resolved(Path::new(&text)), Ok(library.game.clone()));
}

#[test]
fn manifest_and_game_view() {
    let library = Library::new();
    let found = manifest_for_root(&library.game).expect("manifest found");
    assert_eq!(
        found.manifest,
        library.root.join("steamapps").join(MANIFEST)
    );
    assert_eq!(found.library, library.root);
    let layout = resolve_pick(&library.game).expect("resolves");
    assert_eq!(
        game_view_of_paks(&layout, Some(&found.library), OsKind::Linux),
        r"S:\steamapps\common\TEKKEN 8\Polaris\Content\Paks"
    );
    let (_other, outside) = support::temp_root();
    let game = outside.join("Standalone");
    make_game(&game);
    assert!(manifest_for_root(&game).is_none());
}

/// `target` relative to `base` with `..` steps (both absolute, same drive).
fn pathdiff(target: &Path, base: &Path) -> PathBuf {
    let target: Vec<_> = target.components().collect();
    let base: Vec<_> = base.components().collect();
    let common = target
        .iter()
        .zip(&base)
        .take_while(|(left, right)| left == right)
        .count();
    let mut relative = PathBuf::new();
    for _ in common..base.len() {
        relative.push("..");
    }
    for part in &target[common..] {
        relative.push(part);
    }
    relative
}
