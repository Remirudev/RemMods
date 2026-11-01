//! Where Steam installs itself, per OS and package format. Pure: no I/O, no normalisation;
//! detection absolutises, re-spells and dedupes what these functions list.

use std::path::{Path, PathBuf};

use crate::game::SteamKind;
use crate::platform::{OsKind, RegistrySteamValues};

const FLATPAK_APP: [&str; 3] = [".var", "app", "com.valvesoftware.Steam"];
const SNAP_ROOTS: [&[&str]; 2] = [&["snap", "steam"], &[".snap", "data", "steam"]];

/// The ten Linux candidates in order (H = home, D = data dir); the fourth is left out when it
/// equals the third.
pub fn linux_root_candidates(home: &Path, data_dir: Option<&Path>) -> Vec<PathBuf> {
    let flatpak = flatpak_dir(home);
    let mut roots = vec![
        under(home, &[".steam", "steam"]),
        under(home, &[".steam", "root"]),
    ];
    let data_steam = data_dir.map(|dir| dir.join("Steam"));
    let local_share = under(home, &[".local", "share", "Steam"]);
    if let Some(data_steam) = &data_steam {
        roots.push(data_steam.clone());
    }
    if data_steam.as_ref() != Some(&local_share) {
        roots.push(local_share);
    }
    roots.extend([
        under(home, &[".steam", "debian-installation"]),
        home.join("Steam"),
        under(&flatpak, &[".local", "share", "Steam"]),
        under(&flatpak, &["data", "Steam"]),
        under(
            home,
            &["snap", "steam", "common", ".local", "share", "Steam"],
        ),
        under(
            home,
            &[
                ".snap", "data", "steam", "common", ".local", "share", "Steam",
            ],
        ),
    ]);
    roots
}

/// HKCU SteamPath, HKLM InstallPath (32-bit, then 64-bit view), `ProgramFiles(x86)\Steam`,
/// `ProgramFiles\Steam`; empty values and ASCII-case-insensitive duplicates dropped.
pub fn windows_root_candidates(
    registry: &RegistrySteamValues,
    program_files_x86: Option<&Path>,
    program_files: Option<&Path>,
) -> Vec<PathBuf> {
    let listed = [
        registry.steam_path.as_deref().map(PathBuf::from),
        registry.install_path_32.as_deref().map(PathBuf::from),
        registry.install_path_64.as_deref().map(PathBuf::from),
        program_files_x86.map(|dir| dir.join("Steam")),
        program_files.map(|dir| dir.join("Steam")),
    ];
    let mut roots: Vec<PathBuf> = Vec::new();
    for root in listed.into_iter().flatten() {
        let text = root.as_os_str().as_encoded_bytes();
        let duplicate = roots.iter().any(|kept| {
            kept.as_os_str()
                .as_encoded_bytes()
                .eq_ignore_ascii_case(text)
        });
        if !text.is_empty() && !duplicate {
            roots.push(root);
        }
    }
    roots
}

/// Pure prefix test on plain absolute paths: Windows → Windows; under the Flatpak app dir →
/// Flatpak; under `H/snap/steam` or `H/.snap/data/steam` → Snap; else Native.
pub fn steam_kind_of(root: &Path, home: Option<&Path>, os: OsKind) -> SteamKind {
    if os == OsKind::Windows {
        return SteamKind::Windows;
    }
    let Some(home) = home else {
        return SteamKind::Native;
    };
    if root.starts_with(flatpak_dir(home)) {
        SteamKind::Flatpak
    } else if SNAP_ROOTS
        .iter()
        .any(|parts| root.starts_with(under(home, parts)))
    {
        SteamKind::Snap
    } else {
        SteamKind::Native
    }
}

/// A library listed by a Flatpak Steam, seen from the host (F = the Flatpak app dir):
/// the sandbox's own Steam paths are F/.local/share/Steam; another path under the home (not
/// under `.var/app`) is itself if it exists, else F/<path below home> if that exists, else
/// missing (`None`); everything else is kept as it is.
pub fn flatpak_library_path(
    listed: &Path,
    home: &Path,
    exists: &dyn Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let flatpak = flatpak_dir(home);
    let sandbox_views = [
        under(home, &[".local", "share", "Steam"]),
        under(home, &[".steam", "steam"]),
        under(home, &[".steam", "root"]),
    ];
    if sandbox_views.iter().any(|view| listed == view) {
        return Some(under(&flatpak, &[".local", "share", "Steam"]));
    }
    let Ok(below_home) = listed.strip_prefix(home) else {
        return Some(listed.to_path_buf());
    };
    if listed.starts_with(under(home, &[".var", "app"])) {
        return Some(listed.to_path_buf());
    }
    if exists(listed) {
        return Some(listed.to_path_buf());
    }
    let inside = flatpak.join(below_home);
    exists(&inside).then_some(inside)
}

fn flatpak_dir(home: &Path) -> PathBuf {
    under(home, &FLATPAK_APP)
}

fn under(base: &Path, parts: &[&str]) -> PathBuf {
    let mut path = base.to_path_buf();
    path.extend(parts);
    path
}
