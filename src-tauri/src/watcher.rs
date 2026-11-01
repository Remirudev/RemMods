//! The "paks-watcher" thread: watches the Paks folder, debounces bursts and rescans, so changes
//! made outside RemMods show up without a focus change. It never calls window getters.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use remmods_core::game::GameError;
use remmods_core::mods::ModsError;
use remmods_core::safe_fs::{Debounce, FsError, PaksWatcher, WatchError, WatchSignal};
use remmods_core::{Core, CoreError};
use remmods_ipc::WatchState;
use tauri::AppHandle;

use crate::events::{self, ModsChanged, WatchStatusChanged};

/// Longest sleep between wakes: a location change made without `rearm` is noticed this late.
const MAX_WAIT: Duration = Duration::from_secs(5);

enum WatcherInput {
    Fs(WatchSignal),
    Rearm,
    Stop,
}

pub struct WatcherHandle {
    sender: Sender<WatcherInput>,
    state: Arc<Mutex<WatchState>>,
}

impl WatcherHandle {
    /// Drops the current watch and watches the (possibly new) Paks folder.
    pub fn rearm(&self) {
        self.send(WatcherInput::Rearm);
    }

    /// Asks the thread to finish; it is not joined, so exit never waits on a scan.
    pub fn stop(&self) {
        self.send(WatcherInput::Stop);
    }

    pub fn status(&self) -> WatchState {
        *self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn send(&self, input: WatcherInput) {
        if self.sender.send(input).is_err() {
            log::debug!("the Paks watcher has already stopped");
        }
    }
}

pub fn start(app: AppHandle, core: Arc<Core>) -> std::io::Result<WatcherHandle> {
    let (sender, receiver) = mpsc::channel();
    let state = Arc::new(Mutex::new(WatchState::Unavailable));
    let watcher = Watcher {
        app,
        core,
        sender: sender.clone(),
        state: Arc::clone(&state),
    };
    thread::Builder::new()
        .name("paks-watcher".into())
        .spawn(move || watcher.run(&receiver))?;
    Ok(WatcherHandle { sender, state })
}

struct Watcher {
    app: AppHandle,
    core: Arc<Core>,
    /// Handed to each `PaksWatcher`, whose callback runs on notify's own thread.
    sender: Sender<WatcherInput>,
    state: Arc<Mutex<WatchState>>,
}

/// The live watch and the Paks generation it belongs to (`None`: no Paks folder).
struct Armed {
    _watch: Option<PaksWatcher>,
    generation: Option<u32>,
}

impl Watcher {
    fn run(&self, receiver: &Receiver<WatcherInput>) {
        let mut debounce = Debounce::standard();
        let mut armed = self.arm();
        loop {
            match receiver.recv_timeout(wait_time(&debounce, Instant::now())) {
                Ok(WatcherInput::Fs(WatchSignal::Changed | WatchSignal::Rescan)) => {
                    debounce.push(Instant::now());
                }
                Ok(WatcherInput::Fs(WatchSignal::LimitReached)) => {
                    self.set_state(WatchState::LimitReached);
                }
                Ok(WatcherInput::Rearm) => armed = self.rearm(armed),
                Ok(WatcherInput::Stop) | Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => {}
            }
            if self.current_generation() != armed.generation {
                armed = self.rearm(armed);
            }
            let now = Instant::now();
            if debounce.take_if_due(now) && self.refresh() == Refresh::Busy {
                debounce.push(now);
            }
        }
        log::debug!("Paks watcher stopped");
    }

    fn current_generation(&self) -> Option<u32> {
        self.core.game_paks().map(|paks| paks.generation)
    }

    fn rearm(&self, previous: Armed) -> Armed {
        // The old inotify watch goes before the new one is added.
        drop(previous);
        self.arm()
    }

    fn arm(&self) -> Armed {
        let Some(paks) = self.core.game_paks() else {
            self.set_state(WatchState::Unavailable);
            return Armed {
                _watch: None,
                generation: None,
            };
        };
        let sender = self.sender.clone();
        let started = PaksWatcher::start(&paks.paks, move |signal| {
            // A closed channel only means the thread is exiting.
            let _ = sender.send(WatcherInput::Fs(signal));
        });
        let (watch, state) = match started {
            Ok(watch) => (Some(watch), WatchState::Active),
            Err(WatchError::LimitReached) => {
                log::warn!("the inotify watch limit was reached while watching Paks");
                (None, WatchState::LimitReached)
            }
            Err(error) => {
                log::warn!("watching {} failed: {error}", paks.paks.display());
                (None, WatchState::Failed)
            }
        };
        self.set_state(state);
        Armed {
            _watch: watch,
            generation: Some(paks.generation),
        }
    }

    fn refresh(&self) -> Refresh {
        match self.core.mods_refresh() {
            Ok(refreshed) => {
                if refreshed.changed {
                    let revision = refreshed.snapshot.revision;
                    events::emit(&self.app, &ModsChanged { revision });
                }
                Refresh::Done
            }
            Err(CoreError::Fs(FsError::Busy) | CoreError::Mods(ModsError::Fs(FsError::Busy))) => {
                Refresh::Busy
            }
            Err(CoreError::Game(GameError::NotSet)) => Refresh::Done,
            Err(error) => {
                log::warn!("watcher rescan failed: {error}");
                Refresh::Done
            }
        }
    }

    fn set_state(&self, next: WatchState) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if *state == next {
            return;
        }
        *state = next;
        drop(state);
        log::info!("Paks watcher state: {next:?}");
        events::emit(&self.app, &WatchStatusChanged { state: next });
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Refresh {
    Done,
    /// Another writer holds the lock: retried once the debounce is due again.
    Busy,
}

fn wait_time(debounce: &Debounce, now: Instant) -> Duration {
    debounce.deadline().map_or(MAX_WAIT, |deadline| {
        deadline.saturating_duration_since(now).min(MAX_WAIT)
    })
}
