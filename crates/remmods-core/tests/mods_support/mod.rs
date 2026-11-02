//! Fixture builders for the mods tests (P1 spec §8.1, P2 spec §8.2). Every entry is created through
//! `platform::fs_path`, so names Win32 would normalise (`Foo.`, `Foo `) work on Windows too.
#![allow(dead_code, clippy::disallowed_methods, clippy::expect_used)]

pub mod busy;
pub mod manage;
pub mod toggle;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use remmods_core::content::{BaseIndex, ContentCache};
use remmods_core::game::{GameOptions, detect};
use remmods_core::mods::{
    ModRef, ModRow, ScanContent, ScanInput, ScanOutput, paks_identity, read_details, resolve_ref,
    scan_paks, scan_paks_with, scan_paks_with_base, shell_form,
};
use remmods_core::platform::{self, OsKind, RegistrySteamValues, SteamHints};
use remmods_core::{AppPaths, Core, CoreOptions};
use remmods_pak::fixture::TocBuilder;
use tempfile::TempDir;

/// UTF-16 length of `S:\steamapps\common\TEKKEN 8\Polaris\Content\Paks`.
pub const GAME_VIEW_LEN: usize = 49;

pub const DATA: &[u8] = b"data";

/// The mount point of 26 of the 31 real mods (SPEC-P2B §4.4).
pub const MOUNT: &str = "../../../Polaris/Content/";

/// A crafted v5 `.utoc` (flags 0x09) listing `files` below `mount`.
pub fn toc(mount: &str, files: &[&str]) -> Vec<u8> {
    files
        .iter()
        .fold(TocBuilder::new(mount), |builder, file| builder.file(file))
        .build()
}

/// Paks below a game root inside `out` (the §9.2 layout).
pub fn paks_under(out: &Path) -> PathBuf {
    out.join("steamapps")
        .join("common")
        .join("TEKKEN 8")
        .join("Polaris")
        .join("Content")
        .join("Paks")
}

/// Creates `entries` below `root`: `/` separates components, a trailing `/` makes a directory.
pub fn paks_tree(root: &Path, entries: &[(&str, &[u8])]) {
    for (rel, content) in entries {
        let full = rel
            .trim_end_matches('/')
            .split('/')
            .fold(root.to_path_buf(), |path, part| path.join(part));
        let target = platform::fs_path(&full).expect("fs path");
        if rel.ends_with('/') {
            fs::create_dir_all(&target).expect("create fixture dir");
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).expect("create fixture parent");
            }
            fs::write(&target, content).expect("write fixture file");
        }
    }
}

/// A temp dir with an empty `Paks` folder inside (canonical path, see `support::temp_root`).
pub fn fixture() -> (TempDir, PathBuf) {
    let (dir, root) = crate::support::temp_root();
    let paks = root.join("Paks");
    fs::create_dir(&paks).expect("create Paks");
    (dir, paks)
}

/// The row with `key`; panics with the keys present when there is none.
pub fn row<'a>(content: &'a ScanContent, key: &str) -> &'a ModRow {
    content
        .rows
        .iter()
        .find(|row| row.key == key)
        .unwrap_or_else(|| panic!("no row {key}; rows: {:?}", keys(content)))
}

pub fn keys(content: &ScanContent) -> Vec<&str> {
    content.rows.iter().map(|row| row.key.as_str()).collect()
}

pub fn input(paks: &Path) -> ScanInput {
    ScanInput {
        paks: paks.to_path_buf(),
        game_view_len: GAME_VIEW_LEN,
        paks_id: paks_identity(paks, 0),
    }
}

pub fn scan(paks: &Path) -> ScanOutput {
    scan_paks(&input(paks)).expect("scan")
}

/// A scan with the base index the app builds in the background (content conflicts need it).
pub fn scan_based(paks: &Path) -> ScanOutput {
    let base = BaseIndex::load(paks);
    scan_paks_with_base(&input(paks), &ContentCache::new(), Some(&base)).expect("scan")
}

/// A crafted base-game `.utoc` (mount `../../../`) listing `files` such as
/// `Polaris/Content/Ker/Upper.uasset`.
pub const BASE_TOC: &str = "pakchunk0-Windows.utoc";

pub fn base_toc(files: &[&str]) -> Vec<u8> {
    toc("../../../", files)
}

pub fn scan_with(paks: &Path, cache: &ContentCache) -> ScanOutput {
    scan_paks_with(&input(paks), cache).expect("scan")
}

const SMOKE_MODINFO: &[u8] =
    br#"{"Version":"2.0","Category":"Character","Description":"Smoke fixture","Author":"RemMods"}"#;

/// The §9.2 smoke tree below `out`; returns its Paks folder.
pub fn smoke_tree(out: &Path) -> PathBuf {
    let manifest: &[u8] = include_bytes!("../fixtures/steam/appmanifest_1778820.acf");
    paks_tree(
        out,
        &[
            ("steamapps/appmanifest_1778820.acf", manifest),
            ("steamapps/common/TEKKEN 8/TEKKEN 8.exe", b""),
            (
                "steamapps/common/TEKKEN 8/Polaris/Binaries/Win64/Polaris-Win64-Shipping.exe",
                b"",
            ),
        ],
    );
    let paks = paks_under(out);
    let alpha_toc = toc(MOUNT, &["Smoke/Shared.uasset", "Smoke/Alpha.uasset"]);
    let loose_toc = toc(
        "../../../Polaris/Content/Smoke/",
        &["Shared.uasset", "Shared.ubulk"],
    );
    let beta_toc = toc(MOUNT, &["Smoke/Shared.uasset"]);
    let base = base_toc(&["Polaris/Content/Smoke/Shared.uasset"]);
    paks_tree(
        &paks,
        &[
            ("remmods-smoke-fixture", b"{}"),
            ("pakchunk0-Windows.pak", b""),
            ("pakchunk0-Windows.ucas", b""),
            ("pakchunk0-Windows.utoc", &base),
            ("global.ucas", b""),
            ("global.utoc", b""),
            ("Stray.pak", DATA),
            ("Mods/Alpha/Alpha.pak", DATA),
            ("Mods/Alpha/Alpha.ucas", DATA),
            ("Mods/Alpha/Alpha.utoc", &alpha_toc),
            ("Mods/Alpha/modinfo.json", SMOKE_MODINFO),
            ("Mods/Beta/Beta.pak-x", DATA),
            ("Mods/Beta/Beta.ucas-x", DATA),
            ("Mods/Beta/Beta.utoc-x", &beta_toc),
            ("Mods/Gamma/Content/Paks/Gamma.pak", DATA),
            ("Mods/Gamma/Content/Paks/Gamma_alt.pak-x", DATA),
            ("Mods/Delta/Shared.pak", DATA),
            ("Mods/Epsilon/Sub/shared.PAK", DATA),
            ("Mods/Empty/readme.txt", DATA),
            ("Mods/Loose.pak", DATA),
            ("Mods/Loose.ucas", DATA),
            ("Mods/Loose.utoc", &loose_toc),
            ("Mods/Pack.zip", DATA),
            ("install-smoke.zip", DATA),
            (
                "Mods/_Half_tmp_0123456789abcdef0123456789abcdef/Half.pak",
                DATA,
            ),
            (
                "Mods/_Old_tmp_ABCDEF0123456789ABCDEF0123456789/Old.pak-x",
                DATA,
            ),
            ("~mods/Legacy/Legacy.pak", DATA),
            ("LogicMods/Tool/Tool.pak", DATA),
            ("LogicMods/.disabled_Off/Off.pak-x", DATA),
        ],
    );
    paks
}

/// Every entry below `root`: (rel path with `/`, `d`/`f`/`l`, length, mtime), sorted.
pub fn listing(root: &Path) -> Vec<(String, char, u64, SystemTime)> {
    let mut out = Vec::new();
    let mut stack = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, rel)) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("list fixture dir") {
            let entry = entry.expect("fixture entry");
            let name = entry.file_name().to_string_lossy().into_owned();
            let rel_path = if rel.is_empty() {
                name
            } else {
                format!("{rel}/{name}")
            };
            let metadata = fs::symlink_metadata(entry.path()).expect("fixture metadata");
            let kind = if metadata.is_symlink() {
                'l'
            } else if metadata.is_dir() {
                'd'
            } else {
                'f'
            };
            if kind == 'd' {
                stack.push((entry.path(), rel_path.clone()));
            }
            let modified = metadata.modified().expect("fixture mtime");
            out.push((rel_path, kind, metadata.len(), modified));
        }
    }
    out.sort();
    out
}

pub fn linux_hints(home: &Path) -> SteamHints {
    SteamHints {
        os: OsKind::Linux,
        home: Some(home.to_path_buf()),
        data_dir: Some(home.join(".local").join("share")),
        registry: RegistrySteamValues::default(),
        program_files_x86: None,
        program_files: None,
    }
}

/// Runs every read-only entry point against the smoke tree under `home/.local/share/Steam` and
/// asserts each returns Ok (spec §8.1 `tests/read_only.rs`).
pub fn exercise_read_only(home: &Path, app_root: &Path) {
    let steam = home.join(".local").join("share").join("Steam");
    let scan_input = input(&paks_under(&steam));
    let output = scan_paks(&scan_input).expect("scan_paks");
    let content = &output.content;
    assert!(!content.rows.is_empty(), "the smoke tree has rows");
    let refs: Vec<&ModRef> = content
        .rows
        .iter()
        .map(|row| &row.mod_ref)
        .chain(content.archives.iter().map(|entry| &entry.mod_ref))
        .chain(content.unmanaged.iter().map(|entry| &entry.mod_ref))
        .chain(content.leftovers.iter().map(|entry| &entry.mod_ref))
        .collect();
    for mod_ref in refs {
        resolve_ref(&scan_input, mod_ref).expect("resolve_ref");
        shell_form(&scan_input, mod_ref, platform::os_kind()).expect("shell_form");
    }
    for row in &content.rows {
        read_details(&scan_input, &row.mod_ref).expect("read_details");
    }
    let cache = ContentCache::new();
    scan_paks_with(&scan_input, &cache).expect("scan_paks_with");
    let hints = linux_hints(home);
    let _ = detect(&hints, 0);
    let options = CoreOptions {
        game: GameOptions {
            steam_detection: true,
            fixed_location: None,
            steam_hints: Some(hints),
            processes: None,
        },
        require_marker: None,
        ..CoreOptions::default()
    };
    let core = Core::open_with(AppPaths::under(app_root), options).expect("Core::open_with");
    let _ = core.game_status();
    let snapshot = core.mods_list().expect("mods_list");
    let first = snapshot.content.rows.first().expect("a first row");
    core.mods_details(first.mod_ref.as_str())
        .expect("mods_details");
    for row in &snapshot.content.rows {
        core.mods_contents(row.mod_ref.as_str())
            .expect("mods_contents");
    }
    let _ = core.game_detect();
}
