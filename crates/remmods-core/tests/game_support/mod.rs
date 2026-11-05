#![allow(dead_code, clippy::disallowed_methods, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};

use remmods_core::platform::{OsKind, RegistrySteamValues, SteamHints};
use serde_json::Value;

pub const MANIFEST: &str = "appmanifest_1778820.acf";

pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

pub fn steam_fixture(name: &str) -> Vec<u8> {
    fs::read(fixtures_dir().join("steam").join(name)).expect("read steam fixture")
}

pub fn steam_text(name: &str) -> String {
    String::from_utf8(steam_fixture(name)).expect("fixture is UTF-8")
}

/// VDF string escaping for the `@STEAM_ROOT@` / `@LIB1@` placeholders.
pub fn vdf_escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

pub fn templated(name: &str, steam_root: &str, lib1: &str) -> String {
    steam_text(name)
        .replace("@STEAM_ROOT@", &vdf_escape(steam_root))
        .replace("@LIB1@", &vdf_escape(lib1))
}

pub fn text_of(path: &Path) -> String {
    path.to_str().expect("test paths are UTF-8").to_owned()
}

/// The manifest fixture with another build id and StateFlags.
pub fn manifest_with(build: &str, flags: &str) -> String {
    steam_text(MANIFEST)
        .replace(
            "\"buildid\"\t\t\"25221600\"",
            &format!("\"buildid\"\t\t\"{build}\""),
        )
        .replace(
            "\"StateFlags\"\t\t\"4\"",
            &format!("\"StateFlags\"\t\t\"{flags}\""),
        )
}

/// `Polaris/Content/Paks` and the shipping exe below `root`.
pub fn make_game(root: &Path) {
    make_game_named(
        root,
        "Polaris",
        "Content",
        "Paks",
        "Binaries",
        "Win64",
        "Polaris-Win64-Shipping.exe",
    );
}

pub fn make_game_named(
    root: &Path,
    polaris: &str,
    content: &str,
    paks: &str,
    binaries: &str,
    win64: &str,
    exe: &str,
) {
    fs::create_dir_all(root.join(polaris).join(content).join(paks)).expect("create Paks");
    let bin = root.join(polaris).join(binaries).join(win64);
    fs::create_dir_all(&bin).expect("create Win64");
    fs::write(bin.join(exe), b"MZ").expect("write exe");
}

pub fn make_steam_root(root: &Path) {
    fs::create_dir_all(root.join("steamapps")).expect("create steamapps");
}

pub fn write_library_file(root: &Path, text: &str) {
    make_steam_root(root);
    fs::write(root.join("steamapps").join("libraryfolders.vdf"), text).expect("write vdf");
}

/// `library/steamapps/common/TEKKEN 8` with the game tree and a manifest; returns the game root.
pub fn install_game(library: &Path, manifest: &str) -> PathBuf {
    let steamapps = library.join("steamapps");
    let root = steamapps.join("common").join("TEKKEN 8");
    make_game(&root);
    fs::write(steamapps.join(MANIFEST), manifest).expect("write manifest");
    root
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

pub fn under(base: &Path, parts: &[&str]) -> PathBuf {
    let mut path = base.to_path_buf();
    path.extend(parts);
    path
}

pub struct ProcFixture {
    pub name: String,
    pub running: bool,
    pub signal: Option<String>,
    pub pid: Option<u32>,
    pub other_manager: Option<bool>,
    pub processes: Vec<Value>,
}

pub fn proc_fixtures() -> Vec<ProcFixture> {
    let dir = fixtures_dir().join("proc");
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("list proc fixtures")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    paths.sort();
    paths.iter().map(|path| parse_proc_fixture(path)).collect()
}

fn parse_proc_fixture(path: &Path) -> ProcFixture {
    let text = fs::read_to_string(path).expect("read proc fixture");
    let value: Value = serde_json::from_str(&text).expect("proc fixture is JSON");
    assert_eq!(value["version"], 1, "{}", path.display());
    let expect = &value["expect"];
    ProcFixture {
        name: path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .into_owned(),
        running: expect["running"].as_bool().expect("expect.running"),
        signal: expect["signal"].as_str().map(str::to_owned),
        pid: expect["pid"]
            .as_u64()
            .map(|pid| u32::try_from(pid).expect("pid fits")),
        other_manager: expect["otherManager"].as_bool(),
        processes: value["processes"].as_array().expect("processes").clone(),
    }
}

/// A procfs-shaped tree: `<pid>/comm`, `<pid>/cmdline`, plus non-numeric entries.
pub fn build_proc_tree(dir: &Path, processes: &[Value]) {
    fs::create_dir_all(dir.join("self")).expect("self");
    fs::create_dir_all(dir.join("sys")).expect("sys");
    fs::write(dir.join("version"), b"Linux version test\n").expect("version");
    for process in processes {
        let pid = process["pid"].as_u64().expect("pid");
        let process_dir = dir.join(pid.to_string());
        fs::create_dir_all(&process_dir).expect("process dir");
        if let Some(comm) = process["comm"].as_str() {
            fs::write(process_dir.join("comm"), format!("{comm}\n")).expect("comm");
        }
        if let Some(bytes) = cmdline_bytes(process) {
            fs::write(process_dir.join("cmdline"), bytes).expect("cmdline");
        }
    }
}

fn cmdline_bytes(process: &Value) -> Option<Vec<u8>> {
    if let Some(hex) = process["cmdlineHex"].as_str() {
        return Some(decode_hex(hex));
    }
    let args = process["cmdline"].as_array()?;
    if args.is_empty() {
        return Some(Vec::new());
    }
    let mut bytes = Vec::new();
    for (index, arg) in args.iter().enumerate() {
        if index > 0 {
            bytes.push(0);
        }
        bytes.extend_from_slice(arg.as_str().expect("arg").as_bytes());
    }
    bytes.push(0);
    let padding = process["nulPadding"].as_u64().unwrap_or(0);
    bytes.extend(std::iter::repeat_n(
        0u8,
        usize::try_from(padding).expect("padding"),
    ));
    Some(bytes)
}

fn decode_hex(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&hex[at..at + 2], 16).expect("hex"))
        .collect()
}
