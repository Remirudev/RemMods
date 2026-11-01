//! Turns a chosen location into the full state: Steam kind, `in_use`, build id and the update
//! notice. Reads the manifest and may store `lastSeenBuild`; holds no lock.

use std::path::{Path, PathBuf};

use super::layout::{GameLayout, ManifestLocation, manifest_for_root};
use super::service::{same_root, steam_kind_for};
use super::steam::{parse_app_manifest, read_steam_file};
use super::{
    DetectionNote, GameCandidate, GameLocation, InstallInfo, LocationSource, NoteCode,
    UpdateDecision, UpdateNotice, UpdateTracking, decide_update,
};
use crate::paths::display_path;
use crate::platform::OsKind;
use crate::settings::{LastSeenBuild, SettingsStore};

pub(crate) enum Target {
    NotSet,
    Ready {
        layout: GameLayout,
        source: LocationSource,
    },
    Missing {
        path: PathBuf,
        source: Option<LocationSource>,
    },
}

pub(crate) struct Settled {
    pub location: GameLocation,
    pub source: Option<LocationSource>,
    pub library: Option<PathBuf>,
    pub candidates: Vec<GameCandidate>,
    pub update: Option<UpdateNotice>,
    pub build_note: Option<DetectionNote>,
}

struct Build {
    id: Option<String>,
    tracking: UpdateTracking,
    note: Option<DetectionNote>,
}

pub(crate) fn settle(
    settings: &SettingsStore,
    target: Target,
    mut candidates: Vec<GameCandidate>,
    home: Option<&Path>,
    os: OsKind,
) -> Settled {
    for candidate in &mut candidates {
        candidate.in_use = false;
    }
    let (layout, source) = match target {
        Target::NotSet => return unsettled(GameLocation::NotSet, None, candidates),
        Target::Missing { path, source } => {
            return unsettled(GameLocation::Missing { path }, source, candidates);
        }
        Target::Ready { layout, source } => (layout, source),
    };
    let manifest = manifest_for_root(&layout.root);
    let (in_use, steam_kind) =
        steam_kind_for(&layout.root, &candidates, manifest.is_some(), home, os);
    if let Some(index) = in_use {
        candidates[index].in_use = true;
    }
    let build = read_build(manifest.as_ref(), &layout.root);
    let update = update_notice(settings, &layout.root, build.id.as_deref());
    Settled {
        location: GameLocation::Ready(InstallInfo {
            root: layout.root,
            paks: layout.paks,
            exe: layout.exe,
            source,
            steam_kind,
            build_id: build.id,
            update_tracking: build.tracking,
        }),
        source: Some(source),
        library: manifest.map(|found| found.library),
        candidates,
        update,
        build_note: build.note,
    }
}

fn unsettled(
    location: GameLocation,
    source: Option<LocationSource>,
    candidates: Vec<GameCandidate>,
) -> Settled {
    Settled {
        location,
        source,
        library: None,
        candidates,
        update: None,
        build_note: None,
    }
}

/// The manifest only counts when its `installdir` names this root: a copied folder in `common`
/// must not borrow another install's build.
fn read_build(manifest: Option<&ManifestLocation>, root: &Path) -> Build {
    let Some(manifest) = manifest else {
        return no_manifest();
    };
    let failed = |line: Option<u32>, detail: &str| Build {
        id: None,
        tracking: UpdateTracking::Unreadable,
        note: Some(DetectionNote {
            code: NoteCode::ManifestError,
            path: manifest.manifest.clone(),
            line,
            detail: Some(detail.to_owned()),
        }),
    };
    let bytes = match read_steam_file(&manifest.manifest) {
        Ok(bytes) => bytes,
        Err(error) => {
            log::warn!("cannot read {}: {error}", manifest.manifest.display());
            return failed(None, "ioError");
        }
    };
    let parsed = match parse_app_manifest(&bytes) {
        Ok(parsed) => parsed,
        Err(error) => {
            log::warn!("{} unusable: {error:?}", manifest.manifest.display());
            return failed(error.line(), error.code());
        }
    };
    let names_root = root.file_name().is_some_and(|name| {
        name.as_encoded_bytes()
            .eq_ignore_ascii_case(parsed.install_dir.as_bytes())
    });
    if !names_root {
        return no_manifest();
    }
    Build {
        id: Some(parsed.build_id),
        tracking: UpdateTracking::Available,
        note: None,
    }
}

fn no_manifest() -> Build {
    Build {
        id: None,
        tracking: UpdateTracking::NoManifest,
        note: None,
    }
}

/// First run for a root stores its build silently; a different build later is a notice.
fn update_notice(
    settings: &SettingsStore,
    root: &Path,
    current: Option<&str>,
) -> Option<UpdateNotice> {
    let stored = settings.snapshot().settings.game.last_seen_build;
    let same = stored
        .as_ref()
        .is_some_and(|stored| same_root(&stored.game_root, root));
    match decide_update(stored.as_ref(), same, current) {
        UpdateDecision::Unavailable | UpdateDecision::Unchanged => None,
        UpdateDecision::Notice { from, to } => Some(UpdateNotice { from, to }),
        UpdateDecision::StoreSilently => {
            let build_id = current.unwrap_or_default().to_owned();
            let game_root = display_path(root);
            if let Err(error) = settings.update_game(|game| {
                game.last_seen_build = Some(LastSeenBuild {
                    game_root,
                    build_id,
                });
            }) {
                log::warn!("cannot store the seen build: {error}");
            }
            None
        }
    }
}
