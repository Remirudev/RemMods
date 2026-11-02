//! Game state behind one mutex. Detection, manifest reads, process scans and settings writes run
//! without holding it; only the final assignment does.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

use super::detect::{detect, kind_of, same_folder};
use super::layout::{
    GameLayout, game_view_of_paks, plain_absolute, resolve_pick, validate_game_root,
};
use super::running::RunningTracker;
use super::running::scan_verdict;
use super::settle::{Settled, Target, settle};
use super::{
    DetectionNote, GameCandidate, GameError, GameLocation, GameOptions, GameStatus,
    GameStatusChange, LaunchPlan, LocationChoice, LocationSource, PaksLocation, PickResult,
    STEAM_LAUNCH_URL, UpdateNotice,
};
use crate::paths::display_path;
use crate::platform::{self, OsKind};
use crate::settings::{LastSeenBuild, SettingsStore, StoredLocation, StoredSource};

pub struct GameService {
    options: GameOptions,
    initialised: Mutex<bool>,
    state: Mutex<GameState>,
}

impl std::fmt::Debug for GameService {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GameService")
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

struct GameState {
    location: GameLocation,
    source: Option<LocationSource>,
    library: Option<PathBuf>,
    paks_generation: u32,
    detection_generation: u32,
    candidates: Vec<GameCandidate>,
    detection_notes: Vec<DetectionNote>,
    build_note: Option<DetectionNote>,
    update: Option<UpdateNotice>,
    tracker: RunningTracker,
}

impl GameService {
    pub fn new(options: GameOptions) -> Self {
        Self {
            options,
            initialised: Mutex::new(false),
            state: Mutex::new(GameState {
                location: GameLocation::NotSet,
                source: None,
                library: None,
                paks_generation: 0,
                detection_generation: 0,
                candidates: Vec::new(),
                detection_notes: Vec::new(),
                build_note: None,
                update: None,
                tracker: RunningTracker::new(),
            }),
        }
    }

    /// Runs the first resolution once; concurrent callers wait for it.
    pub(crate) fn ensure_initialised(&self, settings: &SettingsStore) {
        let mut initialised = self
            .initialised
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if *initialised {
            return;
        }
        let target = match &self.options.fixed_location {
            Some(fixed) => fixed_target(fixed),
            None => stored_target(settings),
        };
        self.resolve(settings, target);
        *initialised = true;
    }

    pub(crate) fn status(&self) -> GameStatus {
        let state = self.lock();
        let mut notes = state.detection_notes.clone();
        notes.extend(state.build_note.clone());
        GameStatus {
            location: state.location.clone(),
            running: state.tracker.state(),
            session: state.tracker.session(),
            other_manager: state.tracker.other_manager(),
            update: state.update.clone(),
            candidates: state.candidates.clone(),
            notes,
        }
    }

    /// Re-runs detection (when enabled) and re-validates the current location both ways; never
    /// switches a Ready or Missing location to another install.
    pub(crate) fn detect_again(&self, settings: &SettingsStore) {
        self.ensure_initialised(settings);
        let target = {
            let state = self.lock();
            match (&state.location, &self.options.fixed_location) {
                (_, Some(fixed)) => fixed_target(fixed),
                (GameLocation::Ready(install), None) => revalidated(&install.root, state.source),
                (GameLocation::Missing { path }, None) => revalidated(path, state.source),
                (GameLocation::NotSet, None) => Target::NotSet,
            }
        };
        self.resolve(settings, target);
    }

    pub(crate) fn set_location(
        &self,
        settings: &SettingsStore,
        choice: LocationChoice,
    ) -> Result<(), GameError> {
        self.ensure_initialised(settings);
        let (layout, source) = match choice {
            LocationChoice::Candidate { id } => {
                (self.candidate_layout(&id)?, LocationSource::Steam)
            }
            LocationChoice::Path { path } => {
                let picked = self.picked_path(&path)?;
                let layout = resolve_pick(&picked).map_err(GameError::InvalidFolder)?;
                (layout, LocationSource::Manual)
            }
        };
        persist_location(settings, &layout.root, source);
        self.apply(settings, Target::Ready { layout, source });
        Ok(())
    }

    pub(crate) fn choose_path(
        &self,
        settings: &SettingsStore,
        picked: &Path,
    ) -> Result<PickResult, GameError> {
        self.ensure_initialised(settings);
        let picked = self.picked_path(picked)?;
        match resolve_pick(&picked) {
            Ok(layout) => {
                persist_location(settings, &layout.root, LocationSource::Manual);
                self.apply(
                    settings,
                    Target::Ready {
                        layout,
                        source: LocationSource::Manual,
                    },
                );
                Ok(PickResult::Selected(self.status()))
            }
            Err(problem) => Ok(PickResult::Invalid {
                problem,
                path: picked,
            }),
        }
    }

    pub(crate) fn ack_update(&self, settings: &SettingsStore) {
        self.ensure_initialised(settings);
        let acknowledged = {
            let state = self.lock();
            match (&state.location, &state.update) {
                (GameLocation::Ready(install), Some(notice)) => {
                    Some((display_path(&install.root), notice.to.clone()))
                }
                _ => None,
            }
        };
        let Some((game_root, build_id)) = acknowledged else {
            return;
        };
        if let Err(error) = settings.update_game(|game| {
            game.last_seen_build = Some(LastSeenBuild {
                game_root,
                build_id,
            });
        }) {
            log::warn!("cannot store the acknowledged build: {error}");
        }
        self.lock().update = None;
    }

    pub(crate) fn launch_plan(&self) -> LaunchPlan {
        let state = self.lock();
        let steam_kind = match &state.location {
            GameLocation::Ready(install) => install.steam_kind,
            _ => None,
        };
        LaunchPlan {
            url: STEAM_LAUNCH_URL.to_owned(),
            steam_kind,
        }
    }

    pub(crate) fn paks(&self) -> Option<PaksLocation> {
        let state = self.lock();
        let GameLocation::Ready(install) = &state.location else {
            return None;
        };
        let layout = GameLayout {
            root: install.root.clone(),
            paks: install.paks.clone(),
            exe: install.exe.clone(),
        };
        Some(PaksLocation {
            paks: install.paks.clone(),
            game_view: game_view_of_paks(&layout, state.library.as_deref(), platform::os_kind()),
            generation: state.paks_generation,
        })
    }

    pub(crate) fn reveal_target(&self) -> Result<PathBuf, GameError> {
        match &self.lock().location {
            GameLocation::Ready(install) => Ok(dunce::simplified(&install.paks).to_path_buf()),
            _ => Err(GameError::NotSet),
        }
    }

    pub(crate) fn poll_running(&self) -> Option<GameStatusChange> {
        let scan = scan_verdict(self.options.processes.as_ref())
            .map_err(|error| log::debug!("process scan failed: {error}"));
        let mut state = self.lock();
        let changed = state.tracker.observe(scan, Instant::now());
        changed.then(|| change_of(&state))
    }

    /// One fresh process scan for a mutation; a failed scan counts as not running.
    pub(crate) fn running_now(&self) -> bool {
        let scan = scan_verdict(self.options.processes.as_ref())
            .map_err(|error| log::warn!("process scan failed, treated as not running: {error}"));
        let running = scan.as_ref().is_ok_and(|verdict| verdict.game.is_some());
        self.lock().tracker.observe(scan, Instant::now());
        running
    }

    /// Re-reads the build of the Ready location; `Some` when the update notice changed.
    pub(crate) fn refresh_build(&self, settings: &SettingsStore) -> Option<GameStatusChange> {
        self.ensure_initialised(settings);
        let (before, target) = {
            let state = self.lock();
            let GameLocation::Ready(install) = &state.location else {
                return None;
            };
            let layout = GameLayout {
                root: install.root.clone(),
                paks: install.paks.clone(),
                exe: install.exe.clone(),
            };
            (
                state.update.clone(),
                Target::Ready {
                    layout,
                    source: install.source,
                },
            )
        };
        self.apply(settings, target);
        let state = self.lock();
        (state.update != before).then(|| change_of(&state))
    }

    fn resolve(&self, settings: &SettingsStore, target: Target) {
        let detection = self.detection_enabled().then(|| {
            let generation = self.lock().detection_generation.wrapping_add(1);
            detect(&self.hints(), generation)
        });
        let mut target = target;
        if let Some(detection) = detection {
            if matches!(target, Target::NotSet) {
                target = auto_selected(&detection.candidates, settings);
            }
            let mut state = self.lock();
            state.detection_generation = detection.generation;
            state.candidates = detection.candidates;
            state.detection_notes = detection.notes;
        }
        self.apply(settings, target);
    }

    fn apply(&self, settings: &SettingsStore, target: Target) {
        let candidates = self.lock().candidates.clone();
        let hints = self.options.steam_hints.as_ref();
        let home = hints
            .and_then(|hints| hints.home.clone())
            .or_else(dirs::home_dir);
        let os = hints.map_or_else(platform::os_kind, |hints| hints.os);
        let settled = settle(settings, target, candidates, home.as_deref(), os);
        self.commit(settled);
    }

    fn commit(&self, settled: Settled) {
        let mut state = self.lock();
        if location_key(&state.location) != location_key(&settled.location) {
            state.paks_generation = state.paks_generation.wrapping_add(1);
        }
        state.location = settled.location;
        state.source = settled.source;
        state.library = settled.library;
        state.candidates = settled.candidates;
        state.update = settled.update;
        state.build_note = settled.build_note;
    }

    fn candidate_layout(&self, id: &str) -> Result<GameLayout, GameError> {
        let candidate = self
            .lock()
            .candidates
            .iter()
            .find(|candidate| candidate.id == id)
            .cloned()
            .ok_or(GameError::StaleCandidate)?;
        if let Some(problem) = candidate.problem {
            return Err(GameError::InvalidFolder(problem));
        }
        validate_game_root(&candidate.root).map_err(GameError::InvalidFolder)
    }

    /// Trimmed; `~` and `~/…` expand to the home folder; must then be absolute.
    fn picked_path(&self, path: &Path) -> Result<PathBuf, GameError> {
        let path = match path.to_str() {
            Some(text) => {
                let text = text.trim();
                let rest = text
                    .strip_prefix("~/")
                    .or_else(|| (text == "~").then_some(""));
                match (rest, self.home()) {
                    (Some(rest), Some(home)) => home.join(rest),
                    _ => PathBuf::from(text),
                }
            }
            None => path.to_path_buf(),
        };
        if path.as_os_str().is_empty() || !path.is_absolute() {
            return Err(GameError::InvalidPath);
        }
        Ok(path)
    }

    fn detection_enabled(&self) -> bool {
        self.options.steam_detection && self.options.fixed_location.is_none()
    }

    fn hints(&self) -> platform::SteamHints {
        self.options
            .steam_hints
            .clone()
            .unwrap_or_else(platform::steam_hints)
    }

    fn home(&self) -> Option<PathBuf> {
        match &self.options.steam_hints {
            Some(hints) => hints.home.clone(),
            None => dirs::home_dir(),
        }
    }

    fn lock(&self) -> MutexGuard<'_, GameState> {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }
}

fn fixed_target(fixed: &Path) -> Target {
    match resolve_pick(fixed) {
        Ok(layout) => Target::Ready {
            layout,
            source: LocationSource::Fixture,
        },
        Err(problem) => {
            log::warn!(
                "fixed game location {} unusable: {problem:?}",
                fixed.display()
            );
            Target::Missing {
                path: plain_absolute(fixed).unwrap_or_else(|_| fixed.to_path_buf()),
                source: Some(LocationSource::Fixture),
            }
        }
    }
}

fn stored_target(settings: &SettingsStore) -> Target {
    let Some(stored) = settings.snapshot().settings.game.location else {
        return Target::NotSet;
    };
    let source = match stored.source {
        StoredSource::Steam => LocationSource::Steam,
        StoredSource::Manual => LocationSource::Manual,
    };
    revalidated(Path::new(&stored.game_root), Some(source))
}

fn revalidated(root: &Path, source: Option<LocationSource>) -> Target {
    if source == Some(LocationSource::Fixture) {
        return fixed_target(root);
    }
    match validate_game_root(root) {
        Ok(layout) => Target::Ready {
            layout,
            source: source.unwrap_or(LocationSource::Manual),
        },
        Err(problem) => {
            log::info!("game folder {} not usable: {problem:?}", root.display());
            Target::Missing {
                path: root.to_path_buf(),
                source,
            }
        }
    }
}

/// Exactly one usable, fully installed candidate becomes the location (and is stored).
fn auto_selected(candidates: &[GameCandidate], settings: &SettingsStore) -> Target {
    let mut usable = candidates
        .iter()
        .filter(|candidate| candidate.problem.is_none() && candidate.fully_installed);
    let (Some(only), None) = (usable.next(), usable.next()) else {
        return Target::NotSet;
    };
    match validate_game_root(&only.root) {
        Ok(layout) => {
            log::info!("auto-selected the game folder {}", layout.root.display());
            persist_location(settings, &layout.root, LocationSource::Steam);
            Target::Ready {
                layout,
                source: LocationSource::Steam,
            }
        }
        Err(problem) => {
            log::warn!("the only candidate became unusable: {problem:?}");
            Target::NotSet
        }
    }
}

fn persist_location(settings: &SettingsStore, root: &Path, source: LocationSource) {
    let source = match source {
        LocationSource::Steam => StoredSource::Steam,
        LocationSource::Manual => StoredSource::Manual,
        LocationSource::Fixture => return,
    };
    let game_root = display_path(root);
    if let Err(error) = settings.update_game(|game| {
        game.location = Some(StoredLocation { game_root, source });
    }) {
        log::warn!("the game location applies for this run only: {error}");
    }
}

fn location_key(location: &GameLocation) -> (u8, Option<&Path>) {
    match location {
        GameLocation::NotSet => (0, None),
        GameLocation::Ready(install) => (1, Some(&install.root)),
        GameLocation::Missing { path } => (2, Some(path)),
    }
}

fn change_of(state: &GameState) -> GameStatusChange {
    GameStatusChange {
        running: state.tracker.state(),
        session: state.tracker.session(),
        other_manager: state.tracker.other_manager(),
        update: state.update.clone(),
    }
}

/// Kept for `settle`: whether a stored root names the current one.
pub(crate) fn same_root(stored: &str, root: &Path) -> bool {
    if stored == display_path(root) {
        return true;
    }
    let stored_path = Path::new(stored);
    match (platform::fs_path(stored_path), platform::fs_path(root)) {
        (Ok(left), Ok(right)) => match same_file::is_same_file(left, right) {
            Ok(same) => same,
            Err(_) => {
                platform::os_kind() == OsKind::Windows
                    && stored.eq_ignore_ascii_case(&display_path(root))
            }
        },
        _ => false,
    }
}

pub(crate) fn steam_kind_for(
    root: &Path,
    candidates: &[GameCandidate],
    has_manifest: bool,
    home: Option<&Path>,
    os: OsKind,
) -> (Option<usize>, Option<super::SteamKind>) {
    if let Some(index) = candidates
        .iter()
        .position(|candidate| same_folder(&candidate.root, root))
    {
        return (Some(index), Some(candidates[index].steam_kind));
    }
    (None, has_manifest.then(|| kind_of(root, home, os)))
}
