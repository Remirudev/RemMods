//! TekkenModManager's ResolveModsDirectory / ResolveGameDirectory cases, adapted to `resolve_pick`.
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

struct Tree {
    _dir: TempDir,
    root: PathBuf,
    game: PathBuf,
}

fn tree() -> Tree {
    let (dir, root) = support::temp_root();
    let steamapps = root.join("steamapps");
    let game = steamapps.join("common").join("TEKKEN 8");
    make_game(&game);
    fs::write(steamapps.join(MANIFEST), steam_fixture(MANIFEST)).expect("manifest");
    fs::create_dir_all(under(
        &game,
        &["Polaris", "Content", "Paks", "Mods", "m", "s"],
    ))
    .expect("Mods");
    fs::write(
        under(
            &game,
            &["Polaris", "Content", "Paks", "Mods", "m", "s", "file"],
        ),
        b"x",
    )
    .expect("file");
    fs::create_dir_all(under(&game, &["Polaris", "Content", "Paks", "LogicMods"])).expect("Logic");
    Tree {
        _dir: dir,
        root,
        game,
    }
}

fn paks(game: &Path) -> PathBuf {
    under(game, &["Polaris", "Content", "Paks"])
}

fn resolved(pick: &Path) -> Result<PathBuf, FolderProblem> {
    resolve_pick(pick).map(|layout| layout.root)
}

#[test]
fn picks_inside_the_game_resolve_to_its_root() {
    let tree = tree();
    let game = &tree.game;
    let picks = [
        paks(game),
        paks(game).join("Mods"),
        paks(game).join("LogicMods"),
        game.clone(),
        under(game, &["Polaris", "Content"]),
        under(
            game,
            &["Polaris", "Binaries", "Win64", "Polaris-Win64-Shipping.exe"],
        ),
        under(&paks(game), &["Mods", "m", "s", "file"]),
    ];
    for pick in picks {
        assert_eq!(resolved(&pick), Ok(game.clone()), "pick {}", pick.display());
    }
    let mut trailing = paks(game).into_os_string();
    trailing.push(std::path::MAIN_SEPARATOR_STR);
    assert_eq!(resolved(Path::new(&trailing)), Ok(game.clone()));
}

#[test]
fn layout_has_plain_paths_with_on_disk_names() {
    let tree = tree();
    let layout = resolve_pick(&paks(&tree.game)).expect("resolves");
    assert_eq!(
        layout,
        GameLayout {
            root: tree.game.clone(),
            paks: paks(&tree.game),
            exe: under(
                &tree.game,
                &["Polaris", "Binaries", "Win64", "Polaris-Win64-Shipping.exe"]
            ),
        }
    );
}

#[test]
fn lowercase_tree_stores_on_disk_names() {
    let (_dir, root) = support::temp_root();
    let game = root.join("Game");
    make_game_named(
        &game,
        "polaris",
        "content",
        "paks",
        "binaries",
        "win64",
        "polaris-win64-shipping.exe",
    );
    let layout = resolve_pick(&game).expect("resolves");
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
    let tree = tree();
    let root = &tree.root;
    let desktop = root.join("Desktop");
    let elsewhere = under(root, &["Somewhere", "Else", "Paks"]);
    let with_mods = under(root, &["Other", "Mods"]);
    for dir in [&desktop, &elsewhere, &with_mods] {
        fs::create_dir_all(dir).expect("create");
    }
    for pick in [desktop, elsewhere, root.join("Other")] {
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
    let tree = tree();
    let deep = under(&paks(&tree.game), &["a", "b", "c", "d", "e"]);
    fs::create_dir_all(&deep).expect("deep");
    assert_eq!(resolved(&deep), Err(FolderProblem::NotFound));
    let eight = deep.parent().expect("parent");
    assert_eq!(resolved(eight), Ok(tree.game.clone()));
}

#[test]
fn half_a_game_is_not_found() {
    let (_dir, root) = support::temp_root();
    let only_paks = root.join("OnlyPaks");
    fs::create_dir_all(paks(&only_paks)).expect("paks");
    assert_eq!(resolved(&only_paks), Err(FolderProblem::NotFound));
    let only_exe = root.join("OnlyExe");
    let bin = under(&only_exe, &["Polaris", "Binaries", "Win64"]);
    fs::create_dir_all(&bin).expect("bin");
    fs::write(bin.join("Polaris-Win64-Shipping.exe"), b"MZ").expect("exe");
    assert_eq!(resolved(&only_exe), Err(FolderProblem::NotFound));
}

#[test]
fn relative_pick_becomes_absolute() {
    let tree = tree();
    let cwd = std::env::current_dir().expect("cwd");
    let relative = pathdiff(&paks(&tree.game), &cwd);
    assert!(relative.is_relative());
    assert_eq!(resolved(&relative), Ok(tree.game.clone()));
}

#[test]
fn forward_slashes_are_normalised() {
    let tree = tree();
    let text = paks(&tree.game).to_str().expect("UTF-8").replace('\\', "/");
    assert_eq!(resolved(Path::new(&text)), Ok(tree.game.clone()));
}

#[test]
fn manifest_and_game_view() {
    let tree = tree();
    let found = manifest_for_root(&tree.game).expect("manifest found");
    assert_eq!(found.manifest, tree.root.join("steamapps").join(MANIFEST));
    assert_eq!(found.library, tree.root);
    let layout = resolve_pick(&tree.game).expect("resolves");
    assert_eq!(
        game_view_of_paks(&layout, Some(&found.library), OsKind::Linux),
        r"S:\steamapps\common\TEKKEN 8\Polaris\Content\Paks"
    );
    let outside = support::temp_root();
    let game = outside.1.join("Game");
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
