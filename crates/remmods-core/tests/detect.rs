//! Steam detection on temp homes. Hints say `os: Linux` even on Windows; paths compare plain.
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod game_support;
mod support;

use std::fs;
use std::path::{Path, PathBuf};

use game_support::{
    MANIFEST, install_game, linux_hints, make_steam_root, manifest_with, steam_text, templated,
    text_of, under, vdf_escape, write_library_file,
};
use remmods_core::game::{Detection, DetectionNote, NoteCode, SteamKind, detect};
use remmods_core::platform::{OsKind, RegistryError, RegistrySteamValues, SteamHints};

const DETAIL_CODES: &[&str] = &[
    "native",
    "flatpak",
    "snap",
    "windows",
    "unexpectedEof",
    "unterminatedString",
    "expectedOpenBrace",
    "unexpectedCloseBrace",
    "emptyKey",
    "tooDeep",
    "notUtf8",
    "tooLarge",
    "wrongRoot",
    "ioError",
    "missingKey",
    "wrongAppId",
    "unsafeInstallDir",
    "badBuildId",
    "badNumber",
    "notUnicode",
];

fn local_share(home: &Path) -> PathBuf {
    under(home, &[".local", "share", "Steam"])
}

fn flatpak_root(home: &Path) -> PathBuf {
    under(
        home,
        &[
            ".var",
            "app",
            "com.valvesoftware.Steam",
            ".local",
            "share",
            "Steam",
        ],
    )
}

/// A new-format library list with the given paths as "0", "1", …
fn library_vdf(paths: &[&Path]) -> String {
    let mut text = String::from("\"libraryfolders\"\n{\n");
    for (index, path) in paths.iter().enumerate() {
        text.push_str(&format!(
            "\t\"{index}\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n",
            vdf_escape(&text_of(path))
        ));
    }
    text.push_str("}\n");
    text
}

fn run(hints: &SteamHints) -> Detection {
    let detection = detect(hints, 1);
    check_notes(&detection.notes);
    for (index, candidate) in detection.candidates.iter().enumerate() {
        assert_eq!(candidate.id, format!("c1-{index}"));
        assert!(!candidate.in_use);
    }
    detection
}

fn check_notes(notes: &[DetectionNote]) {
    for note in notes {
        if let Some(detail) = &note.detail {
            assert!(
                DETAIL_CODES.contains(&detail.as_str()),
                "unknown detail {detail}"
            );
        }
    }
}

fn notes_with(detection: &Detection, code: NoteCode) -> Vec<&DetectionNote> {
    detection
        .notes
        .iter()
        .filter(|note| note.code == code)
        .collect()
}

#[test]
fn native_root_finds_the_game_in_a_second_library() {
    let (_dir, home) = support::temp_root();
    let (_lib_dir, lib1) = support::temp_root();
    let root = local_share(&home);
    let vdf = templated("libraryfolders.new.vdf", &text_of(&root), &text_of(&lib1));
    write_library_file(&root, &vdf);
    let game = install_game(&lib1, &steam_text(MANIFEST));
    let detection = run(&linux_hints(&home));
    assert_eq!(detection.generation, 1);
    assert_eq!(detection.candidates.len(), 1);
    let candidate = &detection.candidates[0];
    assert_eq!(candidate.root, game);
    assert_eq!(candidate.library, lib1);
    assert_eq!(candidate.build_id.as_deref(), Some("25221600"));
    assert_eq!(candidate.steam_kind, SteamKind::Native);
    assert!(candidate.fully_installed);
    assert_eq!(candidate.problem, None);
    let roots = notes_with(&detection, NoteCode::SteamRoot);
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].path, root);
    assert_eq!(roots[0].detail.as_deref(), Some("native"));
}

#[test]
fn old_format_library_list() {
    let (_dir, home) = support::temp_root();
    let (_lib_dir, lib1) = support::temp_root();
    let root = local_share(&home);
    let vdf = format!(
        "\"LibraryFolders\"\n{{\n\t\"TimeNextStatsReport\"\t\t\"1561832478\"\n\t\"1\"\t\t\"{}\"\n}}\n",
        vdf_escape(&text_of(&lib1))
    );
    write_library_file(&root, &vdf);
    let game = install_game(&lib1, &steam_text(MANIFEST));
    let detection = run(&linux_hints(&home));
    assert_eq!(detection.candidates.len(), 1);
    assert_eq!(detection.candidates[0].root, game);
}

#[test]
fn broken_library_list_is_noted_and_the_root_still_scanned() {
    let (_dir, home) = support::temp_root();
    let root = local_share(&home);
    write_library_file(&root, &steam_text("libraryfolders.broken.vdf"));
    let game = install_game(&root, &steam_text(MANIFEST));
    let detection = run(&linux_hints(&home));
    let broken = notes_with(&detection, NoteCode::VdfError);
    assert_eq!(broken.len(), 1);
    assert_eq!(broken[0].line, Some(14));
    assert_eq!(broken[0].detail.as_deref(), Some("unterminatedString"));
    assert_eq!(
        broken[0].path,
        under(&root, &["steamapps", "libraryfolders.vdf"])
    );
    assert_eq!(detection.candidates.len(), 1);
    assert_eq!(detection.candidates[0].root, game);
}

#[test]
fn library_file_in_config_is_the_fallback() {
    let (_dir, home) = support::temp_root();
    let (_lib_dir, lib1) = support::temp_root();
    let root = local_share(&home);
    make_steam_root(&root);
    fs::create_dir_all(root.join("config")).expect("config");
    fs::write(
        under(&root, &["config", "libraryfolders.vdf"]),
        library_vdf(&[&root, &lib1]),
    )
    .expect("vdf");
    install_game(&lib1, &steam_text(MANIFEST));
    assert_eq!(run(&linux_hints(&home)).candidates.len(), 1);
}

#[test]
fn missing_and_relative_libraries_are_noted() {
    let (_dir, home) = support::temp_root();
    let root = local_share(&home);
    let gone = home.join("gone");
    let vdf = library_vdf(&[&root, &gone, Path::new("relative/lib")]);
    write_library_file(&root, &vdf);
    let detection = run(&linux_hints(&home));
    let missing = notes_with(&detection, NoteCode::LibraryMissing);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].path, gone);
    let relative = notes_with(&detection, NoteCode::LibraryNotAbsolute);
    assert_eq!(relative.len(), 1);
    assert!(detection.candidates.is_empty());
}

#[test]
fn broken_manifest_is_noted_with_a_code() {
    let (_dir, home) = support::temp_root();
    let root = local_share(&home);
    make_steam_root(&root);
    let broken = steam_text(MANIFEST).replace("\"25221600\"", "\"25x\"");
    fs::write(under(&root, &["steamapps", MANIFEST]), broken).expect("manifest");
    let detection = run(&linux_hints(&home));
    let notes = notes_with(&detection, NoteCode::ManifestError);
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].detail.as_deref(), Some("badBuildId"));
    assert!(detection.candidates.is_empty());
}

#[test]
fn flatpak_sandbox_view_and_native_decoy_stay_apart() {
    let (_dir, home) = support::temp_root();
    let flatpak = flatpak_root(&home);
    write_library_file(&flatpak, &library_vdf(&[&local_share(&home)]));
    let flatpak_game = install_game(&flatpak, &manifest_with("222", "4"));
    let native = home.join("Steam");
    make_steam_root(&native);
    let native_game = install_game(&native, &manifest_with("111", "4"));
    let detection = run(&linux_hints(&home));
    assert_eq!(detection.candidates.len(), 2);
    let by_kind = |kind| {
        detection
            .candidates
            .iter()
            .find(|candidate| candidate.steam_kind == kind)
            .expect("candidate of kind")
    };
    let flatpak_candidate = by_kind(SteamKind::Flatpak);
    assert_eq!(flatpak_candidate.root, flatpak_game);
    assert_eq!(flatpak_candidate.build_id.as_deref(), Some("222"));
    let native_candidate = by_kind(SteamKind::Native);
    assert_eq!(native_candidate.root, native_game);
    assert_eq!(native_candidate.build_id.as_deref(), Some("111"));
    assert!(notes_with(&detection, NoteCode::LibraryMissing).is_empty());
}

#[test]
fn flatpak_list_with_the_literal_flatpak_path() {
    let (_dir, home) = support::temp_root();
    let flatpak = flatpak_root(&home);
    write_library_file(&flatpak, &library_vdf(&[&flatpak]));
    let game = install_game(&flatpak, &steam_text(MANIFEST));
    let detection = run(&linux_hints(&home));
    assert_eq!(detection.candidates.len(), 1);
    assert_eq!(detection.candidates[0].root, game);
    assert_eq!(detection.candidates[0].steam_kind, SteamKind::Flatpak);
}

#[test]
fn snap_debian_and_home_steam_roots() {
    let cases = [
        (
            vec!["snap", "steam", "common", ".local", "share", "Steam"],
            SteamKind::Snap,
        ),
        (vec![".steam", "debian-installation"], SteamKind::Native),
        (vec!["Steam"], SteamKind::Native),
    ];
    for (parts, kind) in cases {
        let (_dir, home) = support::temp_root();
        let root = under(&home, &parts);
        make_steam_root(&root);
        let game = install_game(&root, &steam_text(MANIFEST));
        let detection = run(&linux_hints(&home));
        assert_eq!(detection.candidates.len(), 1, "{parts:?}");
        assert_eq!(detection.candidates[0].root, game);
        assert_eq!(detection.candidates[0].steam_kind, kind, "{parts:?}");
    }
}

#[test]
fn incomplete_install_is_listed_and_noted() {
    let (_dir, home) = support::temp_root();
    let root = local_share(&home);
    make_steam_root(&root);
    install_game(&root, &manifest_with("25221600", "1026"));
    let detection = run(&linux_hints(&home));
    assert_eq!(detection.candidates.len(), 1);
    assert!(!detection.candidates[0].fully_installed);
    assert_eq!(notes_with(&detection, NoteCode::InstallIncomplete).len(), 1);
}

#[test]
fn missing_game_folder_is_a_candidate_with_a_problem() {
    let (_dir, home) = support::temp_root();
    let root = local_share(&home);
    make_steam_root(&root);
    fs::write(under(&root, &["steamapps", MANIFEST]), steam_text(MANIFEST)).expect("manifest");
    let detection = run(&linux_hints(&home));
    assert_eq!(detection.candidates.len(), 1);
    assert!(detection.candidates[0].problem.is_some());
    assert_eq!(notes_with(&detection, NoteCode::GameFolderInvalid).len(), 1);
}

#[test]
fn no_steam_is_noted() {
    let (_dir, home) = support::temp_root();
    let detection = run(&linux_hints(&home));
    assert!(detection.candidates.is_empty());
    assert_eq!(notes_with(&detection, NoteCode::NoSteam).len(), 1);
}

#[test]
fn windows_registry_values_find_temp_roots() {
    let (_dir, base) = support::temp_root();
    let root = base.join("SteamWin");
    make_steam_root(&root);
    let game = install_game(&root, &steam_text(MANIFEST));
    let hints = SteamHints {
        os: OsKind::Windows,
        home: None,
        data_dir: None,
        registry: RegistrySteamValues {
            steam_path: Some(text_of(&root)),
            install_path_32: None,
            install_path_64: Some(text_of(&root)),
            errors: vec![RegistryError {
                value: r"HKLM\SOFTWARE\Valve\Steam\InstallPath (32-bit)".to_owned(),
                not_unicode: true,
            }],
        },
        program_files_x86: None,
        program_files: None,
    };
    let detection = run(&hints);
    assert_eq!(detection.candidates.len(), 1);
    assert_eq!(detection.candidates[0].root, game);
    assert_eq!(detection.candidates[0].steam_kind, SteamKind::Windows);
    let registry = notes_with(&detection, NoteCode::RegistryUnreadable);
    assert_eq!(registry.len(), 1);
    assert_eq!(registry[0].detail.as_deref(), Some("notUnicode"));
}

#[test]
fn generation_numbers_the_candidate_ids() {
    let (_dir, home) = support::temp_root();
    let root = local_share(&home);
    make_steam_root(&root);
    install_game(&root, &steam_text(MANIFEST));
    let detection = detect(&linux_hints(&home), 7);
    assert_eq!(detection.candidates[0].id, "c7-0");
}
