//! Debounces process scans into a running state. Time-based, so a faster poll right after a
//! launch does not shorten the grace period.

use std::time::{Duration, Instant};

use super::{ProcessVerdict, RunningState};

pub const NOT_RUNNING_AFTER_SCANS: u8 = 2;
pub const NOT_RUNNING_AFTER: Duration = Duration::from_secs(3);

/// A positive scan means Running at once. Leaving Running needs at least two negative scans
/// spanning at least 3 s: this bridges the gap between Steam's install-script reaper and the
/// game's own reaper, and the churn while the game shuts down.
#[derive(Clone, Debug)]
pub struct RunningTracker {
    state: RunningState,
    negatives: u8,
    first_negative_at: Option<Instant>,
    session: u32,
    original_app: bool,
}

impl Default for RunningTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl RunningTracker {
    /// Unknown, session 0.
    pub fn new() -> Self {
        Self {
            state: RunningState::Unknown,
            negatives: 0,
            first_negative_at: None,
            session: 0,
            original_app: false,
        }
    }

    /// Feeds one scan (`Err` = the process list could not be read); true when the state, the
    /// session or `original_app` changed.
    pub fn observe(&mut self, scan: Result<ProcessVerdict, ()>, now: Instant) -> bool {
        let before = (self.state, self.session, self.original_app);
        match scan {
            Err(()) => {
                self.state = RunningState::Unknown;
                self.original_app = false;
                self.reset_streak();
            }
            Ok(verdict) => {
                self.original_app = verdict.original_app;
                if verdict.game.is_some() {
                    self.observe_positive();
                } else {
                    self.observe_negative(now);
                }
            }
        }
        before != (self.state, self.session, self.original_app)
    }

    pub fn state(&self) -> RunningState {
        self.state
    }

    pub fn session(&self) -> u32 {
        self.session
    }

    pub fn original_app(&self) -> bool {
        self.original_app
    }

    fn observe_positive(&mut self) {
        if self.state != RunningState::Running {
            self.session = self.session.wrapping_add(1);
            self.state = RunningState::Running;
        }
        self.reset_streak();
    }

    fn observe_negative(&mut self, now: Instant) {
        if self.state != RunningState::Running {
            self.state = RunningState::NotRunning;
            return;
        }
        self.negatives = self.negatives.saturating_add(1);
        let first = *self.first_negative_at.get_or_insert(now);
        let span = now.saturating_duration_since(first);
        if self.negatives >= NOT_RUNNING_AFTER_SCANS && span >= NOT_RUNNING_AFTER {
            self.state = RunningState::NotRunning;
            self.reset_streak();
        }
    }

    fn reset_streak(&mut self) {
        self.negatives = 0;
        self.first_negative_at = None;
    }
}
