//! Paks watcher core: notify events filtered into signals, plus the debounce the shell's watcher
//! thread runs. RemMods' own renames are not suppressed: the rescan after a batch publishes an
//! unchanged snapshot and emits nothing.

use std::path::{Component, Path};
use std::time::{Duration, Instant};

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use super::names::is_install_temp_name;
use crate::platform;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatchSignal {
    Changed,
    Rescan,
    LimitReached,
}

#[derive(Debug, thiserror::Error)]
pub enum WatchError {
    #[error("the Paks folder does not exist")]
    Missing,
    #[error("the system limit for watched folders is reached")]
    LimitReached,
    #[error("{0}")]
    Other(String),
}

/// Watches Paks recursively until dropped; symlinks are not followed.
pub struct PaksWatcher {
    _watcher: RecommendedWatcher,
}

impl std::fmt::Debug for PaksWatcher {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("PaksWatcher")
    }
}

impl PaksWatcher {
    pub fn start(
        paks: &Path,
        on_signal: impl Fn(WatchSignal) + Send + 'static,
    ) -> Result<PaksWatcher, WatchError> {
        let raw = platform::fs_path(paks).map_err(|error| WatchError::Other(error.to_string()))?;
        if !raw.is_dir() {
            return Err(WatchError::Missing);
        }
        let handler = move |event: notify::Result<Event>| {
            if let Some(signal) = signal_for(&event) {
                on_signal(signal);
            }
        };
        let config = Config::default().with_follow_symlinks(false);
        let mut watcher = RecommendedWatcher::new(handler, config).map_err(watch_error)?;
        watcher
            .watch(&raw, RecursiveMode::Recursive)
            .map_err(watch_error)?;
        Ok(PaksWatcher { _watcher: watcher })
    }
}

/// Access events (the game opens paks) and events that only touch install temp folders are
/// ignored; a full queue or an unknown error asks for a full rescan.
pub fn signal_for(event: &notify::Result<Event>) -> Option<WatchSignal> {
    let event = match event {
        Err(error) if matches!(error.kind, notify::ErrorKind::MaxFilesWatch) => {
            return Some(WatchSignal::LimitReached);
        }
        Err(_) => return Some(WatchSignal::Rescan),
        Ok(event) => event,
    };
    if event.need_rescan() {
        return Some(WatchSignal::Rescan);
    }
    if matches!(event.kind, EventKind::Access(_)) {
        return None;
    }
    let only_temp = !event.paths.is_empty() && event.paths.iter().all(|path| touches_temp(path));
    (!only_temp).then_some(WatchSignal::Changed)
}

fn touches_temp(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Normal(name) => is_install_temp_name(name),
        _ => false,
    })
}

fn watch_error(error: notify::Error) -> WatchError {
    match error.kind {
        notify::ErrorKind::MaxFilesWatch => WatchError::LimitReached,
        notify::ErrorKind::PathNotFound => WatchError::Missing,
        notify::ErrorKind::Io(ref io) if io.kind() == std::io::ErrorKind::NotFound => {
            WatchError::Missing
        }
        _ => WatchError::Other(error.to_string()),
    }
}

/// Trailing debounce with a cap: due at `last + quiet`, but never later than `first + max`.
#[derive(Clone, Debug)]
pub struct Debounce {
    quiet: Duration,
    max: Duration,
    burst: Option<(Instant, Instant)>,
}

impl Debounce {
    pub const QUIET: Duration = Duration::from_millis(400);
    pub const MAX: Duration = Duration::from_secs(2);

    pub fn new(quiet: Duration, max: Duration) -> Self {
        Self {
            quiet,
            max,
            burst: None,
        }
    }

    pub fn standard() -> Self {
        Self::new(Self::QUIET, Self::MAX)
    }

    pub fn push(&mut self, now: Instant) {
        let first = self.burst.map_or(now, |(first, _)| first);
        self.burst = Some((first, now));
    }

    /// min(last + quiet, first + max); None when idle.
    pub fn deadline(&self) -> Option<Instant> {
        self.burst
            .map(|(first, last)| (last + self.quiet).min(first + self.max))
    }

    /// True (and idle again) when `now` has reached the deadline.
    pub fn take_if_due(&mut self, now: Instant) -> bool {
        match self.deadline() {
            Some(deadline) if now >= deadline => {
                self.burst = None;
                true
            }
            _ => false,
        }
    }
}
