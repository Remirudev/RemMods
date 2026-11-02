//! Folder-busy waits (WG-2): while a folder rename waits for another program, the UI gets a
//! notice about once a second and may cancel the wait. One `BusyWaits` per `OpGate`, so at most
//! one wait runs at a time.

use std::fmt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use super::FsError;
use crate::platform::{self, OsErrorClass, RetryPolicy};

pub const BUSY_NOTICE_AFTER: Duration = Duration::from_secs(1);
const BUSY_UPDATE_EVERY: Duration = Duration::from_secs(1);
const BUSY_CANCEL_POLL: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BusyPhase {
    Waiting,
    Done,
    TimedOut,
    Cancelled,
    Failed,
}

/// One notice for the UI. `folder`: the path below Paks, `/`-joined, lossy (the file name when
/// outside); `holders`: None until WG-1; `limit`: the policy's total; `cancellable`: false for
/// rollbacks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BusyEvent {
    pub wait_id: u32,
    pub phase: BusyPhase,
    pub folder: String,
    pub holders: Option<Vec<String>>,
    pub elapsed: Duration,
    pub limit: Duration,
    pub cancellable: bool,
}

pub type BusySink = Arc<dyn Fn(&BusyEvent) + Send + Sync>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ActiveWait {
    id: u32,
    cancellable: bool,
    cancelled: bool,
}

struct BusyState {
    next_id: AtomicU32,
    active: Mutex<Option<ActiveWait>>,
    /// Set by an accepted cancel; survives `finish` until the next write guard is taken.
    cancel_requested: AtomicBool,
    sink: Mutex<Option<BusySink>>,
    notice_after: Mutex<Duration>,
}

impl Default for BusyState {
    fn default() -> Self {
        Self {
            next_id: AtomicU32::new(1),
            active: Mutex::new(None),
            cancel_requested: AtomicBool::new(false),
            sink: Mutex::new(None),
            notice_after: Mutex::new(BUSY_NOTICE_AFTER),
        }
    }
}

impl fmt::Debug for BusyState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BusyState")
            .field("active", &*lock(&self.active))
            .field(
                "cancel_requested",
                &self.cancel_requested.load(Ordering::SeqCst),
            )
            .field("sink", &lock(&self.sink).is_some())
            .field("notice_after", &*lock(&self.notice_after))
            .finish()
    }
}

impl BusyState {
    fn cancel(&self, wait_id: u32) -> bool {
        let mut active = lock(&self.active);
        match active.as_mut() {
            Some(wait) if wait.id == wait_id && wait.cancellable => {
                wait.cancelled = true;
                self.cancel_requested.store(true, Ordering::SeqCst);
                true
            }
            _ => false,
        }
    }
}

#[derive(Debug, Default)]
pub struct BusyWaits {
    state: Arc<BusyState>,
}

/// A `'static`, Send + Sync handle that can only cancel; a sink may capture it (no Arc around
/// Core or OpGate, no reference cycle).
#[derive(Clone, Debug)]
pub struct BusyCanceller(Arc<BusyState>);

impl BusyCanceller {
    /// True when `wait_id` is the running, cancellable wait; it then stops before its next try.
    /// A true result also sets the sticky "cancel requested" flag (it survives the wait's end).
    pub fn cancel(&self, wait_id: u32) -> bool {
        self.0.cancel(wait_id)
    }
}

impl BusyWaits {
    pub fn set_sink(&self, sink: BusySink) {
        *lock(&self.state.sink) = Some(sink);
    }

    /// Tests: how long a wait runs before its first notice (default `BUSY_NOTICE_AFTER`).
    pub fn set_notice_after(&self, after: Duration) {
        *lock(&self.state.notice_after) = after;
    }

    pub fn canceller(&self) -> BusyCanceller {
        BusyCanceller(Arc::clone(&self.state))
    }

    /// Delegates to `BusyCanceller::cancel`.
    pub fn cancel(&self, wait_id: u32) -> bool {
        self.state.cancel(wait_id)
    }

    pub(crate) fn notice_after(&self) -> Duration {
        *lock(&self.state.notice_after)
    }

    /// Starts a wait and returns its id (never 0).
    pub(crate) fn begin(&self, cancellable: bool) -> u32 {
        let mut id = self.state.next_id.fetch_add(1, Ordering::SeqCst);
        if id == 0 {
            id = self.state.next_id.fetch_add(1, Ordering::SeqCst);
        }
        *lock(&self.state.active) = Some(ActiveWait {
            id,
            cancellable,
            cancelled: false,
        });
        id
    }

    pub(crate) fn is_cancelled(&self, wait_id: u32) -> bool {
        lock(&self.state.active).is_some_and(|wait| wait.id == wait_id && wait.cancelled)
    }

    pub(crate) fn finish(&self, wait_id: u32) {
        let mut active = lock(&self.state.active);
        if active.is_some_and(|wait| wait.id == wait_id) {
            *active = None;
        }
    }

    /// `OpGate::write` calls it whenever it hands out a write guard.
    pub(crate) fn clear_cancel_requested(&self) {
        self.state.cancel_requested.store(false, Ordering::SeqCst);
    }

    /// True when a `cancel` returned true since the current write guard was taken.
    pub fn cancel_requested(&self) -> bool {
        self.state.cancel_requested.load(Ordering::SeqCst)
    }

    /// Calls the sink with no lock held (a sink may call `cancel`).
    pub(crate) fn emit(&self, event: &BusyEvent) {
        let sink = lock(&self.state.sink).clone();
        if let Some(sink) = sink {
            sink(event);
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// What a folder rename of a batch needs to wait visibly (`batch::run`, `roll_back`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct WaitCtx<'a> {
    pub(crate) busy: &'a BusyWaits,
    /// The scope's Paks folder, for the event's `folder`.
    pub(crate) paks: Option<&'a Path>,
    pub(crate) cancellable: bool,
    /// `FaultPlan.folder_policy`: replaces the platform policy for held folders.
    pub(crate) policy: Option<RetryPolicy>,
    /// `FaultPlan.busy`: this many tries fail with AccessDenied before the real rename runs.
    pub(crate) injected: u32,
}

impl WaitCtx<'_> {
    fn policy_for(&self, class: OsErrorClass) -> RetryPolicy {
        match self.policy {
            Some(policy) if is_held(class) => policy,
            _ => platform::folder_retry_policy(class),
        }
    }

    fn event(&self, wait_id: u32, phase: BusyPhase, from: &Path, timing: Timing) -> BusyEvent {
        BusyEvent {
            wait_id,
            phase,
            folder: folder_label(from, self.paks),
            holders: None,
            elapsed: timing.started.elapsed(),
            limit: timing.limit,
            cancellable: self.cancellable,
        }
    }

    /// Emits the last event of an announced wait, then ends it (in that order: a cancel sent
    /// from the sink during `Done` is still accepted and stays sticky).
    fn end(&self, wait: Option<u32>, phase: BusyPhase, from: &Path, timing: Timing) {
        if let Some(id) = wait {
            self.busy.emit(&self.event(id, phase, from, timing));
            self.busy.finish(id);
        }
    }

    fn cancelled(&self, wait: Option<u32>, from: &Path, timing: Timing) -> Option<FsError> {
        let id = wait?;
        if !self.busy.is_cancelled(id) {
            return None;
        }
        self.end(wait, BusyPhase::Cancelled, from, timing);
        Some(FsError::BusyCancelled {
            path: from.to_path_buf(),
        })
    }
}

#[derive(Clone, Copy)]
struct Timing {
    started: Instant,
    limit: Duration,
}

/// The retry loop of a held folder rename (spec §5.1): tries until the policy is used up,
/// announces the wait once it ran `notice_after`, updates about once a second and stops at an
/// accepted cancel before the next try.
pub(crate) fn wait_loop<T>(
    ctx: &WaitCtx<'_>,
    from: &Path,
    mut attempt: impl FnMut() -> Result<T, FsError>,
) -> Result<T, FsError> {
    let notice_after = ctx.busy.notice_after();
    let mut timing = Timing {
        started: Instant::now(),
        limit: Duration::ZERO,
    };
    let mut tries = 0_u32;
    let mut wait: Option<u32> = None;
    let mut last_notice = timing.started;
    loop {
        if tries > 0
            && let Some(error) = ctx.cancelled(wait, from, timing)
        {
            return Err(error);
        }
        tries += 1;
        let error = match attempt() {
            Ok(value) => {
                ctx.end(wait, BusyPhase::Done, from, timing);
                return Ok(value);
            }
            Err(error) => error,
        };
        let Some(class) = error.class() else {
            ctx.end(wait, BusyPhase::Failed, from, timing);
            return Err(error);
        };
        let policy = ctx.policy_for(class);
        timing.limit = policy.total;
        if tries >= policy.attempts || timing.started.elapsed() >= policy.total {
            let phase = if is_held(class) {
                BusyPhase::TimedOut
            } else {
                BusyPhase::Failed
            };
            ctx.end(wait, phase, from, timing);
            return Err(error);
        }
        log::debug!("folder rename failed (try {tries}), waiting: {error}");
        if timing.started.elapsed() >= notice_after {
            let id = match wait {
                Some(_) if last_notice.elapsed() < BUSY_UPDATE_EVERY => None,
                Some(id) => Some(id),
                None => {
                    let id = ctx.busy.begin(ctx.cancellable);
                    wait = Some(id);
                    Some(id)
                }
            };
            if let Some(id) = id {
                last_notice = Instant::now();
                ctx.busy
                    .emit(&ctx.event(id, BusyPhase::Waiting, from, timing));
            }
        }
        let mut left = policy.pause();
        while !left.is_zero() {
            let slice = left.min(BUSY_CANCEL_POLL);
            thread::sleep(slice);
            left = left.saturating_sub(slice);
            if let Some(error) = ctx.cancelled(wait, from, timing) {
                return Err(error);
            }
        }
    }
}

fn is_held(class: OsErrorClass) -> bool {
    matches!(class, OsErrorClass::InUse | OsErrorClass::AccessDenied)
}

/// `Root/name` below Paks, `/`-joined, lossy; the file name when the path is outside Paks.
fn folder_label(path: &Path, paks: Option<&Path>) -> String {
    match paks.and_then(|paks| path.strip_prefix(paks).ok()) {
        Some(below) => below
            .components()
            .map(|part| part.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/"),
        None => path
            .file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy()
            .into_owned(),
    }
}
