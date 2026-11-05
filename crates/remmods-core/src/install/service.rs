//! The install service: recent inspections, the one job slot and the test hooks.

use std::collections::VecDeque;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::SystemTime;

use super::error::InstallError;
use super::model::Inspection;
use crate::archive::{CancelToken, SourceFingerprint};

const KEPT_INSPECTIONS: usize = 8;

/// Called with the staging folders after extraction.
pub type AfterStageHook = Arc<dyn Fn(&[PathBuf]) + Send + Sync>;

/// Test hooks (`Core::inject_install_hooks`).
#[derive(Clone, Default)]
#[doc(hidden)]
pub struct InstallHooks {
    /// Runs after extraction with the staging folders.
    pub after_stage: Option<AfterStageHook>,
    pub available_space: Option<u64>,
    /// The install target panics in its first `create` (inside the backend's catch_unwind).
    pub panic_in_target: bool,
}

impl fmt::Debug for InstallHooks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InstallHooks")
            .field("after_stage", &self.after_stage.is_some())
            .field("available_space", &self.available_space)
            .field("panic_in_target", &self.panic_in_target)
            .finish()
    }
}

/// One file of a candidate as it will be staged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredFile {
    pub(crate) index: u32,
    /// Below the candidate folder, names fixed, without the staging `-x`.
    pub(crate) dest: Vec<String>,
    /// `.pak/.ucas/.utoc/.sig`: staged with `-x`, made live by the commit.
    pub(crate) live_payload: bool,
    pub(crate) size: u64,
    pub(crate) mtime: Option<SystemTime>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredCandidate {
    pub(crate) id: u32,
    pub(crate) files: Vec<StoredFile>,
}

#[derive(Clone, Debug)]
pub(crate) struct StoredInspection {
    pub(crate) source: PathBuf,
    pub(crate) folder: bool,
    pub(crate) fingerprint: Option<SourceFingerprint>,
    pub(crate) entry_count: usize,
    pub(crate) candidates: Vec<StoredCandidate>,
    /// Per clash id: entry indices in the order of `CaseClash::paths`.
    pub(crate) clash_members: Vec<Vec<u32>>,
    pub(crate) inspection: Inspection,
}

#[derive(Default)]
pub struct InstallService {
    inspections: Mutex<VecDeque<Arc<StoredInspection>>>,
    job: Mutex<Slot>,
    hooks: Mutex<InstallHooks>,
}

/// The one job slot. Seal and job start share this mutex, so they cannot interleave.
#[derive(Default)]
enum Slot {
    #[default]
    Idle,
    Job(CancelToken),
    /// A self-update holds it until the process exits (or the install fails).
    Sealed,
}

/// Holds the job slot; frees it when dropped.
pub(crate) struct JobSlot<'a> {
    service: &'a InstallService,
    pub(crate) cancel: CancelToken,
}

impl fmt::Debug for InstallService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inspections = self
            .inspections
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len();
        let busy = self.is_busy();
        f.debug_struct("InstallService")
            .field("inspections", &inspections)
            .field("busy", &busy)
            .finish_non_exhaustive()
    }
}

impl InstallService {
    pub(crate) fn store(&self, stored: StoredInspection) {
        let mut list = self
            .inspections
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        list.push_back(Arc::new(stored));
        while list.len() > KEPT_INSPECTIONS {
            list.pop_front();
        }
    }

    pub(crate) fn get(&self, id: &str) -> Option<Arc<StoredInspection>> {
        self.inspections
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .find(|stored| stored.inspection.id == id)
            .cloned()
    }

    pub(crate) fn begin_job(&self) -> Result<JobSlot<'_>, InstallError> {
        let mut job = self.slot();
        if !matches!(*job, Slot::Idle) {
            return Err(InstallError::Busy);
        }
        let cancel = CancelToken::new();
        *job = Slot::Job(cancel.clone());
        Ok(JobSlot {
            service: self,
            cancel,
        })
    }

    /// Cancels the running job; false when none runs.
    pub(crate) fn cancel(&self) -> bool {
        match &*self.slot() {
            Slot::Job(token) => {
                token.cancel();
                true
            }
            Slot::Idle | Slot::Sealed => false,
        }
    }

    /// Idle → Sealed; false when a job runs or the slot is already sealed.
    pub(crate) fn seal(&self) -> bool {
        let mut job = self.slot();
        if matches!(*job, Slot::Idle) {
            *job = Slot::Sealed;
            true
        } else {
            false
        }
    }

    /// Sealed → Idle only; an idle slot or a running job stays as it is.
    pub(crate) fn unseal(&self) {
        let mut job = self.slot();
        if matches!(*job, Slot::Sealed) {
            *job = Slot::Idle;
        }
    }

    /// A job runs.
    pub(crate) fn is_busy(&self) -> bool {
        matches!(*self.slot(), Slot::Job(_))
    }

    fn slot(&self) -> std::sync::MutexGuard<'_, Slot> {
        self.job.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn hooks(&self) -> InstallHooks {
        self.hooks
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn set_hooks(&self, hooks: InstallHooks) {
        *self.hooks.lock().unwrap_or_else(PoisonError::into_inner) = hooks;
    }
}

impl Drop for JobSlot<'_> {
    fn drop(&mut self) {
        let mut job = self.service.slot();
        if matches!(*job, Slot::Job(_)) {
            *job = Slot::Idle;
        }
    }
}

/// S10, S11 (SPEC-P5 §9.1): the slot API is crate-private, so these run as unit tests.
#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Barrier};
    use std::thread;

    use super::{InstallService, Slot};

    fn is_sealed(service: &InstallService) -> bool {
        matches!(*service.slot(), Slot::Sealed)
    }

    #[test]
    fn s10_a_second_seal_fails_and_unseal_leaves_idle_and_jobs_alone() {
        let service = InstallService::default();
        service.unseal();
        assert!(!service.is_busy());
        assert!(!is_sealed(&service));

        let job = service.begin_job().expect("job");
        assert!(!service.seal());
        service.unseal();
        assert!(service.is_busy());
        assert!(!job.cancel.is_cancelled());
        drop(job);

        assert!(service.seal());
        assert!(!service.seal());
        assert!(is_sealed(&service));
        assert!(service.begin_job().is_err());
        assert!(is_sealed(&service));
        service.unseal();
        assert!(!is_sealed(&service));
        assert!(service.begin_job().is_ok());
    }

    #[test]
    fn s11_no_job_starts_while_the_slot_is_sealed() {
        const ROUNDS: usize = 1000;
        let service = Arc::new(InstallService::default());
        let violated = Arc::new(AtomicBool::new(false));
        let barrier = Arc::new(Barrier::new(2));
        let jobs = {
            let (service, violated, barrier) = (
                Arc::clone(&service),
                Arc::clone(&violated),
                Arc::clone(&barrier),
            );
            thread::spawn(move || {
                barrier.wait();
                for _ in 0..ROUNDS {
                    if let Ok(job) = service.begin_job() {
                        if is_sealed(&service) {
                            violated.store(true, Ordering::SeqCst);
                        }
                        drop(job);
                    }
                }
            })
        };
        let seals = {
            let (service, violated, barrier) = (
                Arc::clone(&service),
                Arc::clone(&violated),
                Arc::clone(&barrier),
            );
            thread::spawn(move || {
                barrier.wait();
                for _ in 0..ROUNDS {
                    if service.seal() {
                        if service.is_busy() {
                            violated.store(true, Ordering::SeqCst);
                        }
                        service.unseal();
                    }
                }
            })
        };
        jobs.join().expect("job thread");
        seals.join().expect("seal thread");
        assert!(!violated.load(Ordering::SeqCst));
        assert!(!service.is_busy());
        assert!(!is_sealed(&service));
    }
}
