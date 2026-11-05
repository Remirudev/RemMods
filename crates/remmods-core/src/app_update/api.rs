//! The update half of the `Core` facade (SPEC-P5 §5.1): the cheap preflight before a download and
//! the seal that holds the app lock while the plugin replaces the app.

use super::{AppUpdateError, UpdateBlock};
use crate::safe_fs::{FsError, Scope};
use crate::{Core, CoreError};

impl Core {
    /// Cheap refusal before the download: an install job runs (InstallRunning), or after the same
    /// dead-owner recovery every mutation does, the journal still has entries (JournalPending).
    /// Takes and releases one write guard; Busy when the app lock is not free within 2 s.
    pub fn update_preflight(&self) -> Result<(), CoreError> {
        if self.ops.is_sealed() {
            return Err(blocked(UpdateBlock::Busy));
        }
        if self.install.is_busy() {
            return Err(blocked(UpdateBlock::InstallRunning));
        }
        let guard = self.ops.write(self.update_scope()?).map_err(busy_blocked)?;
        if guard.journal_has_entries()? {
            return Err(blocked(UpdateBlock::JournalPending));
        }
        Ok(())
    }

    /// Takes the install slot (InstallRunning if a job runs), then a write guard (recovering dead
    /// owners; Paks scope when a location is set, else the app scope) and seals the gate if the
    /// journal is empty (JournalPending otherwise). Sealed: every later `ops.write` and install
    /// job start fails with Busy until `unseal_after_update`; scans keep working.
    pub fn seal_for_update(&self) -> Result<(), CoreError> {
        if self.ops.is_sealed() {
            return Err(blocked(UpdateBlock::Busy));
        }
        if !self.install.seal() {
            return Err(blocked(UpdateBlock::InstallRunning));
        }
        let sealed = self.seal_gate();
        if sealed.is_err() {
            self.install.unseal();
        }
        sealed
    }

    /// Only after a failed install: reopens the gate and the install slot.
    pub fn unseal_after_update(&self) {
        self.ops.unseal();
        self.install.unseal();
    }

    pub fn is_sealed_for_update(&self) -> bool {
        self.ops.is_sealed()
    }

    fn seal_gate(&self) -> Result<(), CoreError> {
        let guard = self.ops.write(self.update_scope()?).map_err(busy_blocked)?;
        if guard.seal_gate()? {
            Ok(())
        } else {
            Err(blocked(UpdateBlock::JournalPending))
        }
    }

    fn update_scope(&self) -> Result<Scope, CoreError> {
        match self.mods_scan_input() {
            Ok(input) => self.manage_scope(&input),
            Err(_) => self.app_scope(),
        }
    }
}

fn blocked(reason: UpdateBlock) -> CoreError {
    AppUpdateError::Blocked(reason).into()
}

fn busy_blocked(error: FsError) -> CoreError {
    match error {
        FsError::Busy => blocked(UpdateBlock::Busy),
        other => other.into(),
    }
}
