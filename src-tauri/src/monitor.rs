//! The "game-monitor" thread: polls the process list and emits `game-status-changed`. It never
//! calls window getters, which block on the event loop.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use remmods_core::Core;
use tauri::AppHandle;

use crate::events::{self, GameStatusChanged};

/// Right after a launch, so "running" shows within a second of the game process appearing.
const BOOSTED_INTERVAL: Duration = Duration::from_secs(1);
const BOOST_DURATION: Duration = Duration::from_secs(60);
/// While the window is visible and not minimized (also behind a fullscreen game).
const ACTIVE_INTERVAL: Duration = Duration::from_secs(2);
const IDLE_INTERVAL: Duration = Duration::from_secs(10);

pub enum MonitorMsg {
    /// Poll now (window focused).
    Wake,
    /// Poll every second for the next minute (game launched).
    Boost,
    Stop,
}

pub struct MonitorHandle {
    sender: Sender<MonitorMsg>,
    active: Arc<AtomicBool>,
}

impl MonitorHandle {
    pub fn wake(&self) {
        self.send(MonitorMsg::Wake);
    }

    pub fn boost(&self) {
        self.send(MonitorMsg::Boost);
    }

    /// Asks the thread to finish; it is not joined, so exit never waits on a process scan.
    pub fn stop(&self) {
        self.send(MonitorMsg::Stop);
    }

    /// Active = the window is visible and not minimized; focus does not matter.
    pub fn set_active(&self, active: bool) {
        self.active.store(active, Ordering::Relaxed);
    }

    fn send(&self, message: MonitorMsg) {
        if self.sender.send(message).is_err() {
            log::debug!("the game monitor has already stopped");
        }
    }
}

pub fn start(app: AppHandle, core: Arc<Core>) -> std::io::Result<MonitorHandle> {
    let (sender, receiver) = mpsc::channel();
    let active = Arc::new(AtomicBool::new(false));
    let thread_active = Arc::clone(&active);
    thread::Builder::new()
        .name("game-monitor".into())
        .spawn(move || run(&app, &core, &receiver, &thread_active))?;
    Ok(MonitorHandle { sender, active })
}

fn run(app: &AppHandle, core: &Core, receiver: &Receiver<MonitorMsg>, active: &AtomicBool) {
    // Detection runs here first, so the main thread never waits for it.
    core.game_status();
    let mut boosted_until = None;
    loop {
        if let Some(change) = core.game_poll_running() {
            events::emit(app, &GameStatusChanged::from(change));
        }
        let now = Instant::now();
        let interval = poll_interval(now, boosted_until, active.load(Ordering::Relaxed));
        match receiver.recv_timeout(interval) {
            Ok(MonitorMsg::Wake) | Err(RecvTimeoutError::Timeout) => {}
            Ok(MonitorMsg::Boost) => boosted_until = Some(Instant::now() + BOOST_DURATION),
            Ok(MonitorMsg::Stop) | Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    log::debug!("game monitor stopped");
}

fn poll_interval(now: Instant, boosted_until: Option<Instant>, active: bool) -> Duration {
    if boosted_until.is_some_and(|until| now < until) {
        BOOSTED_INTERVAL
    } else if active {
        ACTIVE_INTERVAL
    } else {
        IDLE_INTERVAL
    }
}
