//! Serialises file-system work: in-process an RwLock (scans read, mutations write), across
//! processes the app lock (shared for scans, exclusive for mutations). Lock order: OpGate first,
//! then the mods service's scan gate. Nothing under a WriteGuard may take either lock again.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Duration;

use super::batch::{self, BatchOptions, BatchReport, RenameOp};
use super::journal::{Journal, RecoveryReport};
use super::preflight::Chains;
use super::scope::Scope;
use super::sessions::{self, SESSIONS_DIR};
use super::{AppLock, FsError, ensure_dir, preflight, recovery, write_atomic};
use crate::paths::AppPaths;

const LOCK_WAIT: Duration = Duration::from_secs(2);

#[derive(Debug)]
pub struct OpGate {
    lock: RwLock<()>,
    lock_path: PathBuf,
    journal: Journal,
    /// The exclusive app lock a self-update holds from its seal until the process exits
    /// (SPEC-P5 §5.2): writes fail with Busy, scans need no app lock of their own.
    sealed: Mutex<Option<AppLock>>,
    /// Held exclusively for the gate's life: it tells other processes this session is alive.
    _session_file: File,
}

/// A scan's guard. Field order is drop order: the app lock goes first.
#[derive(Debug)]
pub struct ReadGuard<'a> {
    _app_lock: Option<AppLock>,
    _inner: RwLockReadGuard<'a, ()>,
}

/// A mutation's guard; dead sessions' journal entries were recovered when it was taken.
#[derive(Debug)]
pub struct WriteGuard<'a> {
    gate: &'a OpGate,
    scope: Scope,
    recovery: RecoveryReport,
    _app_lock: AppLock,
    _inner: RwLockWriteGuard<'a, ()>,
}

impl OpGate {
    /// Creates this process's session lock file and removes those of dead sessions that own no
    /// journal entry. Takes no app lock, so startup never fails with Busy.
    pub fn open(paths: &AppPaths) -> Result<OpGate, FsError> {
        let sessions_dir = paths.data_dir.join(SESSIONS_DIR);
        ensure_dir(&sessions_dir)?;
        let (session, session_file) = sessions::create_own(&sessions_dir)?;
        let journal = Journal::new(&paths.data_dir, sessions_dir.clone(), session.clone());
        match journal.owners_unlocked() {
            Some(owners) => sessions::clean_dead(&sessions_dir, &session, &owners),
            None => log::warn!("journal not readable; dead session files are kept"),
        }
        Ok(OpGate {
            lock: RwLock::new(()),
            lock_path: paths.lock_file.clone(),
            journal,
            sealed: Mutex::new(None),
            _session_file: session_file,
        })
    }

    pub fn session(&self) -> &str {
        self.journal.session()
    }

    /// Scans: the in-process read lock (blocks), then the shared app lock within 2 s, else Busy.
    /// While sealed this process already holds the exclusive app lock, which keeps other
    /// processes' writers out, so the scan takes none.
    pub fn read(&self) -> Result<ReadGuard<'_>, FsError> {
        let inner = self.lock.read().unwrap_or_else(PoisonError::into_inner);
        let app_lock = if self.is_sealed() {
            None
        } else {
            Some(AppLock::shared_within(&self.lock_path, LOCK_WAIT)?)
        };
        Ok(ReadGuard {
            _app_lock: app_lock,
            _inner: inner,
        })
    }

    /// Mutations: the in-process write lock (blocks), then the exclusive app lock within 2 s,
    /// else Busy; then journal recovery of dead owners within `scope`.
    /// Sealed for an update: Busy at once.
    pub fn write(&self, scope: Scope) -> Result<WriteGuard<'_>, FsError> {
        let inner = self.lock.write().unwrap_or_else(PoisonError::into_inner);
        if self.is_sealed() {
            return Err(FsError::Busy);
        }
        let app_lock = AppLock::exclusive_within(&self.lock_path, LOCK_WAIT)?;
        let recovery = recovery::recover(&self.journal, &scope)?;
        Ok(WriteGuard {
            gate: self,
            scope,
            recovery,
            _app_lock: app_lock,
            _inner: inner,
        })
    }

    pub fn is_sealed(&self) -> bool {
        self.sealed_lock().is_some()
    }

    /// Drops the held app lock. Takes the in-process write lock first, so no scan that skipped
    /// the shared app lock is still running when other processes may write again.
    pub fn unseal(&self) {
        let _inner = self.lock.write().unwrap_or_else(PoisonError::into_inner);
        if self.sealed_lock().take().is_some() {
            log::info!("operation gate unsealed");
        }
    }

    fn sealed_lock(&self) -> std::sync::MutexGuard<'_, Option<AppLock>> {
        self.sealed.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl WriteGuard<'_> {
    /// Seals the gate for a self-update if the journal is empty: the guard's exclusive app lock
    /// moves into the gate and the in-process write lock is released. Any journal entry →
    /// `Ok(false)` and the guard is dropped normally.
    pub fn seal_gate(self) -> Result<bool, FsError> {
        if self.journal_has_entries()? {
            return Ok(false);
        }
        let WriteGuard {
            gate,
            _app_lock: app_lock,
            _inner: inner,
            ..
        } = self;
        *gate.sealed_lock() = Some(app_lock);
        drop(inner);
        log::info!("operation gate sealed for an update");
        Ok(true)
    }

    /// Entries of any session, read under this guard.
    pub fn journal_has_entries(&self) -> Result<bool, FsError> {
        Ok(!self.gate.journal.entries()?.is_empty())
    }

    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    pub(super) fn journal(&self) -> &Journal {
        &self.gate.journal
    }

    pub fn recovery(&self) -> &RecoveryReport {
        &self.recovery
    }

    /// The batch checks alone; writes nothing.
    pub fn preflight(&self, ops: &[RenameOp]) -> Result<(), FsError> {
        preflight::preflight(&self.scope, ops, Chains::Reject).map(drop)
    }

    pub fn rename_batch(
        &self,
        ops: &[RenameOp],
        options: &BatchOptions,
    ) -> Result<BatchReport, FsError> {
        batch::run(
            &self.scope,
            &self.gate.journal,
            ops,
            options,
            Chains::Reject,
        )
    }

    /// `rename_batch` that also allows a target equal (fold key) to the pre-batch source of an
    /// EARLIER op of the same batch: that name is vacated by then. Everything else as
    /// `rename_batch`.
    pub fn rename_batch_replacing(
        &self,
        ops: &[RenameOp],
        options: &BatchOptions,
    ) -> Result<BatchReport, FsError> {
        batch::run(
            &self.scope,
            &self.gate.journal,
            ops,
            options,
            Chains::AllowVacated,
        )
    }

    pub(super) fn scope_mut(&mut self) -> &mut Scope {
        &mut self.scope
    }

    /// App-data JSON (mod-meta.json): only inside the app dirs of the scope.
    pub fn write_json(&self, path: &Path, bytes: &[u8]) -> Result<(), FsError> {
        self.scope.check_app(path)?;
        write_atomic(path, bytes)
    }
}
