use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chrono::Utc;

use super::error::ModsError;
use super::model::{ScanInput, ScanOutput, ScanSnapshot};
use super::scan::scan_paks;
use super::scan_log;

/// Cache of the last published snapshot with single-flight refreshes (spec §4.8). P1 has no
/// writer under Paks, so there is no app lock or OpGate here yet.
pub struct ModsService {
    cache: Mutex<Option<Cached>>,
    scan_gate: Mutex<()>,
    /// One sequence for request registrations and scan starts, so their order is total.
    sequence: AtomicU64,
}

#[derive(Clone, Debug)]
pub struct Refreshed {
    pub snapshot: Arc<ScanSnapshot>,
    pub changed: bool,
}

struct Cached {
    snapshot: Arc<ScanSnapshot>,
    scan_started: u64,
}

impl ModsService {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(None),
            scan_gate: Mutex::new(()),
            sequence: AtomicU64::new(0),
        }
    }

    /// The cached snapshot when it was made for the same Paks folder and identity, else a refresh.
    pub fn list(&self, input: &ScanInput) -> Result<Arc<ScanSnapshot>, ModsError> {
        if let Some(cached) = self.cached_for(input) {
            return Ok(cached.snapshot);
        }
        Ok(self.refresh(input)?.snapshot)
    }

    /// Scans unless a scan that started after this call already published; at most one scan runs.
    pub fn refresh(&self, input: &ScanInput) -> Result<Refreshed, ModsError> {
        let request = self.sequence.fetch_add(1, Ordering::SeqCst);
        let _gate = lock(&self.scan_gate);
        if let Some(cached) = self.cached_for(input)
            && cached.scan_started > request
        {
            return Ok(Refreshed {
                snapshot: cached.snapshot,
                changed: false,
            });
        }
        let scan_started = self.sequence.fetch_add(1, Ordering::SeqCst);
        let scanned_at = Utc::now();
        let output = scan_paks(input)?;
        Ok(self.publish(output, scan_started, scanned_at))
    }

    pub fn last(&self) -> Option<Arc<ScanSnapshot>> {
        lock(&self.cache)
            .as_ref()
            .map(|cached| Arc::clone(&cached.snapshot))
    }

    fn cached_for(&self, input: &ScanInput) -> Option<CachedView> {
        let cache = lock(&self.cache);
        let cached = cache.as_ref()?;
        let content = &cached.snapshot.content;
        (content.paks == input.paks && content.paks_id == input.paks_id).then(|| CachedView {
            snapshot: Arc::clone(&cached.snapshot),
            scan_started: cached.scan_started,
        })
    }

    fn publish(
        &self,
        output: ScanOutput,
        scan_started: u64,
        scanned_at: chrono::DateTime<Utc>,
    ) -> Refreshed {
        let mut cache = lock(&self.cache);
        if let Some(cached) = cache.as_mut()
            && cached.snapshot.content == output.content
        {
            cached.scan_started = scan_started;
            return Refreshed {
                snapshot: Arc::clone(&cached.snapshot),
                changed: false,
            };
        }
        let revision = cache
            .as_ref()
            .map_or(1, |cached| cached.snapshot.revision.saturating_add(1));
        let snapshot = Arc::new(ScanSnapshot {
            revision,
            scanned_at,
            stats: output.stats,
            content: output.content,
        });
        scan_log::log_snapshot(&snapshot);
        *cache = Some(Cached {
            snapshot: Arc::clone(&snapshot),
            scan_started,
        });
        Refreshed {
            snapshot,
            changed: true,
        }
    }
}

impl Default for ModsService {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for ModsService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let revision = self.last().map(|snapshot| snapshot.revision);
        f.debug_struct("ModsService")
            .field("revision", &revision)
            .finish_non_exhaustive()
    }
}

struct CachedView {
    snapshot: Arc<ScanSnapshot>,
    scan_started: u64,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
