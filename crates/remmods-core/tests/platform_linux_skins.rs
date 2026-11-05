//! Linux only: the saves in the Proton prefix, and a link in place of a save.
#![cfg(target_os = "linux")]
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod mods_support;
mod support;

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use mods_support::toggle::{Extras, MARKER, ToggleFixture};
use remmods_core::safe_fs::{FsError, SKIN_SAVE_FILES, Scope};

const ACCOUNT: &str = "76561198000000001";

fn proton_saves(game_root: &Path) -> PathBuf {
    let steamapps = game_root
        .parent()
        .and_then(Path::parent)
        .expect("steamapps");
    steamapps
        .join("compatdata")
        .join("1778820")
        .join("pfx")
        .join("drive_c")
        .join("users")
        .join("steamuser")
        .join("AppData")
        .join("Local")
        .join("TEKKEN 8")
        .join("Saved")
        .join("SaveGames")
        .join(ACCOUNT)
}

fn write_saves(folder: &Path) {
    fs::create_dir_all(folder).expect("folder");
    for name in SKIN_SAVE_FILES {
        fs::write(folder.join(name), name).expect("save");
    }
}

#[test]
fn the_saves_are_found_in_the_proton_prefix_of_the_library() {
    let fixture = ToggleFixture::new(&[]);
    write_saves(&proton_saves(&fixture.game_root));

    let view = fixture.core.skins_view();

    assert!(view.available);
    assert_eq!(view.account.as_deref(), Some(ACCOUNT));
}

#[test]
fn smoke_runs_never_see_real_saves() {
    let fixture = ToggleFixture::build(&[(MARKER, b"")], |_| Extras {
        marker: Some(MARKER.to_owned()),
        ..Extras::default()
    });
    write_saves(&proton_saves(&fixture.game_root));

    assert!(!fixture.core.skins_view().available);
}

#[test]
fn a_link_in_place_of_a_save_is_refused() {
    let fixture = ToggleFixture::new(&[]);
    let folder = fixture.root.join("Saved").join("SaveGames").join(ACCOUNT);
    write_saves(&folder);
    let elsewhere = fixture.root.join("elsewhere.sav");
    fs::write(&elsewhere, b"keep").expect("file");
    fs::remove_file(folder.join(SKIN_SAVE_FILES[1])).expect("remove");
    symlink(&elsewhere, folder.join(SKIN_SAVE_FILES[1])).expect("link");
    let paths = &fixture.paths;
    let scope = Scope::from_roots(&[], std::slice::from_ref(&paths.data_dir)).expect("scope");
    let guard = fixture.core.ops.write(scope).expect("guard");
    let files: [Vec<u8>; 5] = Default::default();

    let refused = guard.replace_skin_saves(&folder, &files, None);

    assert!(matches!(refused, Err(FsError::OutOfScope { .. })));
    assert_eq!(fs::read(&elsewhere).expect("read"), b"keep");
}
