//! Install sources the user picked or dropped. Their paths stay in Rust: the webview only gets an
//! opaque `src-…` id and a display name, so it can never ask core to inspect an arbitrary path.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use remmods_core::Core;
use remmods_core::game::GameLocation;
use remmods_core::install::{self, SourceKind};
use remmods_ipc::{PickedSource, RejectedSource, SourceReject};

/// Oldest ids are forgotten first; a forgotten id makes `install_inspect` answer `Stale`.
const MAX_SOURCES: usize = 64;

#[derive(Default)]
pub struct SourceRegistry {
    entries: VecDeque<(String, PathBuf)>,
}

impl SourceRegistry {
    pub fn register(&mut self, path: PathBuf, kind: SourceKind) -> PickedSource {
        let id = format!("src-{}", uuid::Uuid::new_v4().simple());
        let picked = PickedSource {
            id: id.clone(),
            name: display_name(&path),
            kind: kind.into(),
        };
        if self.entries.len() >= MAX_SOURCES {
            self.entries.pop_front();
        }
        self.entries.push_back((id, path));
        picked
    }

    pub fn resolve(&self, id: &str) -> Option<PathBuf> {
        self.entries
            .iter()
            .find(|(known, _)| known == id)
            .map(|(_, path)| path.clone())
    }
}

/// A path checked by core: what it is, or why it cannot be installed from.
pub struct Classified {
    pub path: PathBuf,
    pub result: Result<SourceKind, SourceReject>,
}

/// Classifies each path against the current Paks folder (blocking: reads metadata).
pub fn classify_all(core: &Core, paths: Vec<PathBuf>) -> Vec<Classified> {
    let paks = current_paks(core);
    paths
        .into_iter()
        .map(|path| {
            let result =
                install::classify_source(&path, paks.as_deref()).map_err(SourceReject::from);
            Classified { path, result }
        })
        .collect()
}

/// Registers the accepted paths; the refused ones come back for the UI (names only).
pub fn register_all(
    registry: &mut SourceRegistry,
    classified: Vec<Classified>,
) -> (Vec<PickedSource>, Vec<RejectedSource>) {
    let mut sources = Vec::new();
    let mut rejected = Vec::new();
    for Classified { path, result } in classified {
        match result {
            Ok(kind) => sources.push(registry.register(path, kind)),
            Err(reason) => rejected.push(RejectedSource {
                name: display_name(&path),
                reason,
            }),
        }
    }
    (sources, rejected)
}

pub fn display_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

fn current_paks(core: &Core) -> Option<PathBuf> {
    match core.game_status().location {
        GameLocation::Ready(install) => Some(install.paks),
        GameLocation::NotSet | GameLocation::Missing { .. } => None,
    }
}
