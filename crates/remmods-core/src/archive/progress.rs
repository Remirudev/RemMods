use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

const TICK_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Clone, Debug, Default)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
    #[cfg(feature = "rar-builtin")]
    rar: rars::ReadCancellation,
}

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
        #[cfg(feature = "rar-builtin")]
        self.rar.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    #[cfg(feature = "rar-builtin")]
    pub(crate) fn rar(&self) -> &rars::ReadCancellation {
        &self.rar
    }
}

type TickFn = Arc<dyn Fn(u64, u32) + Send + Sync>;

struct ProgressInner {
    bytes: AtomicU64,
    files: AtomicU32,
    last_tick: Mutex<Option<Instant>>,
    on_tick: Option<TickFn>,
}

#[derive(Clone)]
pub struct ProgressCounter {
    inner: Arc<ProgressInner>,
}

impl ProgressCounter {
    /// `on_tick(bytes_done, files_done)` runs at most every 100 ms and once more at the end.
    pub fn new(on_tick: Arc<dyn Fn(u64, u32) + Send + Sync>) -> Self {
        Self::with_callback(Some(on_tick))
    }

    pub fn silent() -> Self {
        Self::with_callback(None)
    }

    fn with_callback(on_tick: Option<TickFn>) -> Self {
        Self {
            inner: Arc::new(ProgressInner {
                bytes: AtomicU64::new(0),
                files: AtomicU32::new(0),
                last_tick: Mutex::new(None),
                on_tick,
            }),
        }
    }

    pub fn bytes(&self) -> u64 {
        self.inner.bytes.load(Ordering::SeqCst)
    }

    pub fn files(&self) -> u32 {
        self.inner.files.load(Ordering::SeqCst)
    }

    /// Forces a final tick.
    pub fn flush(&self) {
        self.tick(true);
    }

    pub(crate) fn add_bytes(&self, bytes: u64) {
        if bytes > 0 {
            self.inner.bytes.fetch_add(bytes, Ordering::SeqCst);
            self.tick(false);
        }
    }

    pub(crate) fn add_file(&self) {
        self.inner.files.fetch_add(1, Ordering::SeqCst);
        self.tick(false);
    }

    fn tick(&self, force: bool) {
        let Some(on_tick) = &self.inner.on_tick else {
            return;
        };
        {
            let mut last = self
                .inner
                .last_tick
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let now = Instant::now();
            if !force && last.is_some_and(|at| now.duration_since(at) < TICK_INTERVAL) {
                return;
            }
            *last = Some(now);
        }
        on_tick(self.bytes(), self.files());
    }
}

impl fmt::Debug for ProgressCounter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProgressCounter")
            .field("bytes", &self.bytes())
            .field("files", &self.files())
            .finish_non_exhaustive()
    }
}
