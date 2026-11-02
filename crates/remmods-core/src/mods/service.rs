use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chrono::Utc;

use super::error::ModsError;
use super::meta_store::ModMetaStore;
use super::model::{ScanInput, ScanOutput, ScanSnapshot};
use super::scan::scan_paks_with_base;
use super::scan_log;
use crate::content::{BaseIndex, ContentCache};

/// Cache of the last published snapshot with single-flight refreshes (P1 spec §4.8). The OpGate
/// is taken by the callers (`Core`), always before the scan gate.
pub struct ModsService {
    meta: ModMetaStore,
    cache: Mutex<Option<Cached>>,
    scan_gate: Mutex<()>,
    /// One sequence for request registrations and scan starts, so their order is total.
    sequence: AtomicU64,
    contents: ContentCache,
    /// Built by `Core::mods_base_refresh` in the background, never by a scan; scans only use it.
    base: Mutex<Option<Arc<BaseIndex>>>,
    /// Single flight for base builds and the rescan that follows them.
    base_gate: Mutex<()>,
}

#[derive(Clone, Debug)]
pub struct Refreshed {
    pub snapshot: Arc<ScanSnapshot>,
    pub changed: bool,
}

struct Cached {
    snapshot: Arc<ScanSnapshot>,
    scan_started: u64,
    /// Set when a rescan after a mutation failed: the disk changed after this scan, so it is
    /// never answered from the cache again; the next scan clears it.
    stale: bool,
}

impl ModsService {
    /// `meta_path`: `data_dir/mod-meta.json`.
    pub fn new(meta_path: PathBuf) -> Self {
        Self {
            meta: ModMetaStore::new(meta_path),
            cache: Mutex::new(None),
            scan_gate: Mutex::new(()),
            sequence: AtomicU64::new(0),
            contents: ContentCache::new(),
            base: Mutex::new(None),
            base_gate: Mutex::new(()),
        }
    }

    pub fn contents(&self) -> &ContentCache {
        &self.contents
    }

    pub(crate) fn base_gate(&self) -> MutexGuard<'_, ()> {
        lock(&self.base_gate)
    }

    /// The last built base index of `paks`, if any; never reads the disk.
    pub(crate) fn stored_base_index(&self, paks: &Path) -> Option<Arc<BaseIndex>> {
        lock(&self.base)
            .as_ref()
            .filter(|base| base.is_for(paks))
            .map(Arc::clone)
    }

    /// Rebuilds the base index of `paks` unless its top-level base `.utoc` files are unchanged;
    /// `true` when a new index was stored. The build runs outside the `base` lock.
    pub(crate) fn update_base_index(&self, paks: &Path) -> bool {
        let previous = self.stored_base_index(paks);
        let current = BaseIndex::current(paks, previous.clone());
        if previous.is_some_and(|previous| Arc::ptr_eq(&previous, &current)) {
            return false;
        }
        *lock(&self.base) = Some(current);
        true
    }

    pub fn meta(&self) -> &ModMetaStore {
        &self.meta
    }

    /// The cached snapshot when it was made for the same Paks folder and identity, else a refresh.
    pub fn list(&self, input: &ScanInput) -> Result<Arc<ScanSnapshot>, ModsError> {
        if let Some(cached) = self.cached(input) {
            return Ok(cached);
        }
        Ok(self.refresh(input)?.snapshot)
    }

    /// The cached snapshot when it was made for the same Paks folder and identity.
    pub fn cached(&self, input: &ScanInput) -> Option<Arc<ScanSnapshot>> {
        self.cached_for(input).map(|cached| cached.snapshot)
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
        let base = self.stored_base_index(&input.paks);
        let output = scan_paks_with_base(input, &self.contents, base.as_deref())?;
        Ok(self.publish(output, scan_started, scanned_at))
    }

    /// After a mutation whose final rescan failed: `list`, `cached` and `refresh` scan again
    /// instead of answering with the snapshot taken before the mutation. The revision is kept.
    pub(crate) fn mark_stale(&self) {
        if let Some(cached) = lock(&self.cache).as_mut() {
            cached.stale = true;
        }
    }

    pub fn last(&self) -> Option<Arc<ScanSnapshot>> {
        lock(&self.cache)
            .as_ref()
            .map(|cached| Arc::clone(&cached.snapshot))
    }

    fn cached_for(&self, input: &ScanInput) -> Option<CachedView> {
        let cache = lock(&self.cache);
        let cached = cache.as_ref().filter(|cached| !cached.stale)?;
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
            cached.stale = false;
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
            stale: false,
        });
        Refreshed {
            snapshot,
            changed: true,
        }
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
