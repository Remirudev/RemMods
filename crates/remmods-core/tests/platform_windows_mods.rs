//! Windows-only scan cases: names Win32 rewrites, long CJK names, unpaired surrogates, junctions
//! and file ids (spec §8.1).
#![cfg(windows)]
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod mods_support;
mod support;

use std::ffi::OsString;
use std::fs;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;
use std::process::Command;

use mods_support::{DATA, fixture, input, paks_tree, row, scan, smoke_tree};
use remmods_core::game::GameOptions;
use remmods_core::mods::{ModIssue, ModState, ModsError, NameProblem, UnmanagedKind, shell_form};
use remmods_core::platform::{self, OsKind};
use remmods_core::{AppPaths, Core, CoreError, CoreOptions};

fn junction(link: &Path, target: &Path) {
    let status = Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .status()
        .expect("run mklink");
    assert!(status.success(), "mklink /J failed");
}

#[test]
fn names_win32_rewrites_stay_apart() {
    let (_dir, paks) = fixture();
    paks_tree(
        &paks,
        &[
            ("Mods/Foo/a.pak", DATA),
            ("Mods/Foo./b.pak", DATA),
            ("Mods/Foo /c.pak", DATA),
        ],
    );
    let content = scan(&paks).content;
    assert_eq!(content.rows.len(), 3);
    for key in ["Standard:Foo", "Standard:Foo.", "Standard:Foo "] {
        assert_eq!(row(&content, key).payload.live_pak, 1, "{key}");
    }
    let trailing = ModIssue::InvalidName {
        problem: NameProblem::TrailingDotOrSpace,
    };
    assert!(row(&content, "Standard:Foo.").issues.contains(&trailing));
    assert!(row(&content, "Standard:Foo ").issues.contains(&trailing));
    assert!(!row(&content, "Standard:Foo").issues.contains(&trailing));
}

#[test]
fn reveal_refuses_a_trailing_dot() {
    let (_dir, root) = support::temp_root();
    let out = root.join("fixture");
    let paks = smoke_tree(&out);
    paks_tree(&paks, &[("Mods/Foo./a.pak", DATA)]);
    let game_root = out.join("steamapps").join("common").join("TEKKEN 8");
    let options = CoreOptions {
        game: GameOptions {
            steam_detection: false,
            fixed_location: Some(game_root),
            steam_hints: None,
            processes: None,
        },
        require_marker: None,
    };
    let core = Core::open_with(AppPaths::under(&root.join("app")), options).expect("core");
    let snapshot = core.mods_list().expect("mods_list");
    let dotted = row(&snapshot.content, "Standard:Foo.");
    assert!(matches!(
        core.mods_reveal_target(dotted.mod_ref.as_str()),
        Err(CoreError::Mods(ModsError::NotShellSafe))
    ));
}

#[test]
fn long_cjk_names_are_valid_but_not_portable() {
    let (_dir, paks) = fixture();
    let name = "\u{672C}".repeat(100);
    let path = format!("Mods/{name}/a.pak");
    paks_tree(&paks, &[(path.as_str(), DATA)]);
    let content = scan(&paks).content;
    let long = row(&content, &format!("Standard:{name}"));
    assert!(
        !long
            .issues
            .iter()
            .any(|issue| matches!(issue, ModIssue::InvalidName { .. }))
    );
    assert!(long.issues.contains(&ModIssue::NameNotPortable {
        problem: NameProblem::TooLong
    }));
    shell_form(&input(&paks), &long.mod_ref, OsKind::Windows).expect("revealable");
}

#[test]
fn unpaired_surrogates_are_escaped() {
    let (_dir, paks) = fixture();
    let name = OsString::from_wide(&[0x62, 0x61, 0x64, 0xD800]);
    let folder = platform::fs_path(&paks.join("Mods").join(name)).expect("fs path");
    fs::create_dir_all(&folder).expect("create");
    fs::write(folder.join("a.pak"), DATA).expect("pak");
    let content = scan(&paks).content;
    let bad = row(&content, r"Standard:bad\u{D800}");
    assert!(bad.issues.contains(&ModIssue::NonUnicodeName));
}

#[test]
fn junctions_are_links() {
    let (_dir, paks) = fixture();
    let elsewhere = paks.parent().expect("parent").join("elsewhere");
    paks_tree(
        &elsewhere,
        &[("Target/a.pak-x", DATA), ("Extra/x.pak", DATA)],
    );
    paks_tree(&paks, &[("Mods/Mixed/b.pak", DATA)]);
    junction(&paks.join("Mods").join("Linked"), &elsewhere.join("Target"));
    junction(
        &paks.join("Mods").join("Mixed").join("Extra"),
        &elsewhere.join("Extra"),
    );
    let content = scan(&paks).content;
    let linked = row(&content, "Standard:Linked");
    assert!(linked.issues.contains(&ModIssue::LinkedMod));
    assert_eq!(linked.state, ModState::Disabled);
    let mixed = row(&content, "Standard:Mixed");
    assert!(mixed.issues.contains(&ModIssue::ContainsLink));
    assert_eq!(mixed.payload.live_pak, 1);
    assert!(
        content
            .unmanaged
            .iter()
            .any(|e| e.kind == UnmanagedKind::LinkedFolder && e.rel_path == "Mods/Mixed/Extra")
    );
}

#[test]
fn file_ids_are_stable_and_distinct() {
    let (_dir, paks) = fixture();
    paks_tree(&paks, &[("Mods/A/", b""), ("Mods/B/", b"")]);
    let a = platform::fs_path(&paks.join("Mods").join("A")).expect("fs path");
    let b = platform::fs_path(&paks.join("Mods").join("B")).expect("fs path");
    let first = platform::file_id(&a).expect("id a");
    assert_eq!(platform::file_id(&a).expect("id a again"), first);
    assert_ne!(platform::file_id(&b).expect("id b"), first);
}

#[test]
fn fs_path_keeps_trailing_dots_and_spaces() {
    let kept = remmods_core::platform::fs_path(Path::new(r"C:\Games\Mods\Foo.\Bar \a.pak"));
    assert_eq!(
        kept.expect("fs path"),
        Path::new(r"\\?\C:\Games\Mods\Foo.\Bar \a.pak")
    );
    let resolved = remmods_core::platform::fs_path(Path::new(r"C:\Games\x\..\y\.\z"));
    assert_eq!(resolved.expect("fs path"), Path::new(r"\\?\C:\Games\y\z"));
    let unc = remmods_core::platform::fs_path(Path::new(r"\\server\share\Mods\Foo."));
    assert_eq!(
        unc.expect("fs path"),
        Path::new(r"\\?\UNC\server\share\Mods\Foo.")
    );
}
