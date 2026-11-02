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
    job: Mutex<Option<CancelToken>>,
    hooks: Mutex<InstallHooks>,
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
        let busy = self
            .job
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_some();
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
        let mut job = self.job.lock().unwrap_or_else(PoisonError::into_inner);
        if job.is_some() {
            return Err(InstallError::Busy);
        }
        let cancel = CancelToken::new();
        *job = Some(cancel.clone());
        Ok(JobSlot {
            service: self,
            cancel,
        })
    }

    /// Cancels the running job; false when none runs.
    pub(crate) fn cancel(&self) -> bool {
        match &*self.job.lock().unwrap_or_else(PoisonError::into_inner) {
            Some(token) => {
                token.cancel();
                true
            }
            None => false,
        }
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
        *self
            .service
            .job
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = None;
    }
}
