//! Steam detection: roots → libraries → TEKKEN 8 manifests → game folders. Reads only; every
//! problem becomes a `DetectionNote` with a code the UI translates.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use same_file::Handle;

use super::layout::{
    child_dir, lookup_child, plain_absolute, validate_game_root, with_disk_casing,
};
use super::steam::{
    AppManifest, flatpak_library_path, linux_root_candidates, parse_app_manifest,
    parse_library_folders, read_steam_file, steam_kind_of, windows_root_candidates,
};
use super::{DetectionNote, FolderProblem, GameCandidate, MANIFEST_FILE, NoteCode, SteamKind};
use crate::platform::{self, OsKind, SteamHints};

const STEAMAPPS: &str = "steamapps";
const COMMON: &str = "common";
const CONFIG: &str = "config";
const LIBRARY_FILE: &str = "libraryfolders.vdf";
const IO_ERROR: &str = "ioError";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Detection {
    pub generation: u32,
    pub candidates: Vec<GameCandidate>,
    pub notes: Vec<DetectionNote>,
}

struct SteamRoot {
    path: PathBuf,
    kind: SteamKind,
}

/// One full detection run. Candidate ids are `c{generation}-{index}`.
pub fn detect(hints: &SteamHints, generation: u32) -> Detection {
    let mut notes = registry_notes(hints);
    let roots = steam_roots(hints, &mut notes);
    let mut found: Vec<GameCandidate> = Vec::new();
    for root in &roots {
        for library in libraries_of(root, hints, &mut notes) {
            if let Some(candidate) = candidate_in(&library, root.kind, &mut notes) {
                found.push(candidate);
            }
        }
    }
    let mut candidates = dedupe_by_identity(found, |candidate| candidate.root.as_path());
    for (index, candidate) in candidates.iter_mut().enumerate() {
        candidate.id = format!("c{generation}-{index}");
    }
    log::info!(
        "Steam detection {generation}: {} root(s), {} candidate(s), {} note(s)",
        roots.len(),
        candidates.len(),
        notes.len()
    );
    Detection {
        generation,
        candidates,
        notes,
    }
}

/// `dunce::canonicalize` of each side (absolute path on error): a symlinked root gets the kind
/// of its target, and both sides stay plain on Windows.
pub(crate) fn kind_of(root: &Path, home: Option<&Path>, os: OsKind) -> SteamKind {
    let canonical = |path: &Path| {
        dunce::canonicalize(path)
            .unwrap_or_else(|_| std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf()))
    };
    let home = home.map(canonical);
    steam_kind_of(&canonical(root), home.as_deref(), os)
}

pub(crate) fn same_folder(left: &Path, right: &Path) -> bool {
    match (platform::fs_path(left), platform::fs_path(right)) {
        (Ok(left), Ok(right)) => same_file::is_same_file(left, right).unwrap_or(false),
        _ => false,
    }
}

fn registry_notes(hints: &SteamHints) -> Vec<DetectionNote> {
    hints
        .registry
        .errors
        .iter()
        .map(|error| {
            log::warn!("Steam registry value unreadable: {}", error.value);
            note(
                NoteCode::RegistryUnreadable,
                PathBuf::from(&error.value),
                None,
                error.not_unicode.then_some("notUnicode"),
            )
        })
        .collect()
}

fn steam_roots(hints: &SteamHints, notes: &mut Vec<DetectionNote>) -> Vec<SteamRoot> {
    let listed = match hints.os {
        OsKind::Linux => hints
            .home
            .as_deref()
            .map(|home| linux_root_candidates(home, hints.data_dir.as_deref()))
            .unwrap_or_default(),
        OsKind::Windows => windows_root_candidates(
            &hints.registry,
            hints.program_files_x86.as_deref(),
            hints.program_files.as_deref(),
        ),
        OsKind::Other => Vec::new(),
    };
    let kept: Vec<PathBuf> = listed
        .iter()
        .filter_map(|root| normalised(root))
        .filter(|root| child_dir(root, STEAMAPPS).is_ok())
        .collect();
    let roots: Vec<SteamRoot> = dedupe_by_identity(kept, PathBuf::as_path)
        .into_iter()
        .map(|path| SteamRoot {
            kind: kind_of(&path, hints.home.as_deref(), hints.os),
            path,
        })
        .collect();
    for root in &roots {
        notes.push(note(
            NoteCode::SteamRoot,
            root.path.clone(),
            None,
            Some(kind_code(root.kind)),
        ));
    }
    if roots.is_empty() {
        log::info!("no Steam root found");
        notes.push(note(NoteCode::NoSteam, PathBuf::new(), None, None));
    }
    roots
}

fn libraries_of(
    root: &SteamRoot,
    hints: &SteamHints,
    notes: &mut Vec<DetectionNote>,
) -> Vec<PathBuf> {
    let mut libraries = vec![root.path.clone()];
    for listed in listed_libraries(&root.path, notes) {
        let listed = PathBuf::from(listed);
        if !listed.is_absolute() {
            notes.push(note(NoteCode::LibraryNotAbsolute, listed, None, None));
            continue;
        }
        let seen_from_host = match (root.kind, hints.home.as_deref()) {
            (SteamKind::Flatpak, Some(home)) => flatpak_library_path(&listed, home, &exists),
            _ => Some(listed.clone()),
        };
        let Some(library) = seen_from_host.and_then(|path| normalised(&path)) else {
            notes.push(note(NoteCode::LibraryMissing, listed, None, None));
            continue;
        };
        if child_dir(&library, STEAMAPPS).is_err() {
            notes.push(note(NoteCode::LibraryMissing, library, None, None));
            continue;
        }
        libraries.push(library);
    }
    dedupe_by_identity(libraries, PathBuf::as_path)
}

/// The listed library paths; `[]` (root only) when no file exists or it cannot be used.
fn listed_libraries(root: &Path, notes: &mut Vec<DetectionNote>) -> Vec<String> {
    let in_steamapps = child_dir(root, STEAMAPPS)
        .ok()
        .and_then(|steamapps| child_file(&steamapps, LIBRARY_FILE));
    let Some(file) = in_steamapps.or_else(|| {
        child_dir(root, CONFIG)
            .ok()
            .and_then(|config| child_file(&config, LIBRARY_FILE))
    }) else {
        log::debug!("no {LIBRARY_FILE} under {}", root.display());
        return Vec::new();
    };
    let bytes = match read_steam_file(&file) {
        Ok(bytes) => bytes,
        Err(error) => {
            log::warn!("cannot read {}: {error}", file.display());
            notes.push(note(NoteCode::VdfError, file, None, Some(IO_ERROR)));
            return Vec::new();
        }
    };
    match parse_library_folders(&bytes) {
        Ok(paths) => paths,
        Err(error) => {
            log::warn!("{} is not a library list: {error:?}", file.display());
            notes.push(note(
                NoteCode::VdfError,
                file,
                error.line(),
                Some(error.code()),
            ));
            Vec::new()
        }
    }
}

fn candidate_in(
    library: &Path,
    kind: SteamKind,
    notes: &mut Vec<DetectionNote>,
) -> Option<GameCandidate> {
    let steamapps = child_dir(library, STEAMAPPS).ok()?;
    let manifest_path = child_file(&steamapps, MANIFEST_FILE)?;
    let manifest = read_manifest(&manifest_path, notes)?;
    let game_dir = game_dir_of(&steamapps, &manifest.install_dir);
    let (root, problem) = match validate_game_root(&game_dir) {
        Ok(layout) => (layout.root, None),
        Err(problem) => (game_dir, Some(problem)),
    };
    match problem {
        Some(FolderProblem::NotUnicode) => {
            notes.push(note(NoteCode::PathNotUnicode, root.clone(), None, None));
        }
        Some(_) => notes.push(note(NoteCode::GameFolderInvalid, root.clone(), None, None)),
        None => {}
    }
    if !manifest.fully_installed() {
        notes.push(note(NoteCode::InstallIncomplete, root.clone(), None, None));
    }
    Some(GameCandidate {
        id: String::new(),
        root,
        library: library.to_path_buf(),
        steam_kind: kind,
        build_id: Some(manifest.build_id.clone()),
        fully_installed: manifest.fully_installed(),
        problem,
        in_use: false,
    })
}

fn read_manifest(path: &Path, notes: &mut Vec<DetectionNote>) -> Option<AppManifest> {
    let bytes = match read_steam_file(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            log::warn!("cannot read {}: {error}", path.display());
            notes.push(note(
                NoteCode::ManifestError,
                path.to_path_buf(),
                None,
                Some(IO_ERROR),
            ));
            return None;
        }
    };
    match parse_app_manifest(&bytes) {
        Ok(manifest) => Some(manifest),
        Err(error) => {
            log::warn!("{} is not a usable manifest: {error:?}", path.display());
            notes.push(note(
                NoteCode::ManifestError,
                path.to_path_buf(),
                error.line(),
                Some(error.code()),
            ));
            None
        }
    }
}

/// `<steamapps>/common/<installdir>`, each component in its on-disk spelling when it exists.
fn game_dir_of(steamapps: &Path, install_dir: &str) -> PathBuf {
    let common = on_disk_child(steamapps, COMMON);
    on_disk_child(&common, install_dir)
}

fn on_disk_child(dir: &Path, name: &str) -> PathBuf {
    let on_disk = lookup_child(dir, OsStr::new(name)).ok().flatten();
    dir.join(on_disk.as_deref().unwrap_or(OsStr::new(name)))
}

fn child_file(dir: &Path, name: &str) -> Option<PathBuf> {
    let on_disk = lookup_child(dir, OsStr::new(name)).ok().flatten()?;
    Some(dir.join(on_disk))
}

fn normalised(path: &Path) -> Option<PathBuf> {
    plain_absolute(path)
        .ok()
        .map(|path| with_disk_casing(&path))
}

fn exists(path: &Path) -> bool {
    platform::fs_path(path).is_ok_and(|target| fs::metadata(target).is_ok())
}

/// Keeps the first of several entries that name the same folder; entries whose identity cannot
/// be read are kept.
fn dedupe_by_identity<T>(items: Vec<T>, path_of: impl Fn(&T) -> &Path) -> Vec<T> {
    let mut handles: Vec<Handle> = Vec::new();
    let mut kept = Vec::new();
    for item in items {
        let handle = platform::fs_path(path_of(&item)).and_then(Handle::from_path);
        match handle {
            Ok(handle) if handles.contains(&handle) => {}
            Ok(handle) => {
                handles.push(handle);
                kept.push(item);
            }
            Err(_) => kept.push(item),
        }
    }
    kept
}

fn note(code: NoteCode, path: PathBuf, line: Option<u32>, detail: Option<&str>) -> DetectionNote {
    DetectionNote {
        code,
        path,
        line,
        detail: detail.map(str::to_owned),
    }
}

pub(crate) fn kind_code(kind: SteamKind) -> &'static str {
    match kind {
        SteamKind::Native => "native",
        SteamKind::Flatpak => "flatpak",
        SteamKind::Snap => "snap",
        SteamKind::Windows => "windows",
    }
}
