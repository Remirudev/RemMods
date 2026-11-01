//! The toggle half of the `Core` facade: one all-or-none batch per call (spec §5.5).

use super::error::ModsError;
use super::model::{ScanContent, ScanInput};
use super::service::Refreshed;
use super::toggle_apply::{
    Plan, Selection, ToggleOutcome, ToggleRequest, plan_rows, select_disable_all, select_targets,
};
use super::toggle_plan::RecordChange;
use crate::platform;
use crate::safe_fs::{BatchOptions, RetryMode, Scope, WriteGuard};
use crate::{Core, CoreError};

const BATCH_LABEL: &str = "toggle";

enum Choice<'r> {
    Targets(&'r ToggleRequest),
    DisableAll,
}

impl Core {
    pub fn mods_toggle(&self, request: &ToggleRequest) -> Result<ToggleOutcome, CoreError> {
        self.apply_toggle(
            Choice::Targets(request),
            request.enable,
            request.apply_while_running,
        )
    }

    pub fn mods_disable_all(&self, apply_while_running: bool) -> Result<ToggleOutcome, CoreError> {
        self.apply_toggle(Choice::DisableAll, false, apply_while_running)
    }

    fn apply_toggle(
        &self,
        choice: Choice<'_>,
        enable: bool,
        apply_while_running: bool,
    ) -> Result<ToggleOutcome, CoreError> {
        let input = self.mods_scan_input()?;
        let app_dirs = [
            self.paths.data_dir.as_path(),
            self.paths.config_dir.as_path(),
        ];
        let scope = Scope::for_paks(&input.paks, &app_dirs)?;
        let guard = self.ops.write(scope)?;
        self.check_marker(&input)?;
        let result = self.toggle_under_guard(&guard, &input, &choice, enable, apply_while_running);
        // Always publish what is on disk now; the service directly, never `mods_refresh` (§3).
        let refreshed = self.mods.refresh(&input);
        match (result, refreshed) {
            (Ok(applied), Ok(refreshed)) => Ok(ToggleOutcome {
                refreshed,
                skipped: applied.skipped,
                unmanaged: applied.unmanaged,
            }),
            (Ok(_), Err(error)) => Err(error.into()),
            (Err(error), refreshed) => {
                if let Err(refresh_error) = refreshed {
                    log::warn!("refresh after a failed toggle failed: {refresh_error}");
                }
                Err(error)
            }
        }
    }

    fn check_marker(&self, input: &ScanInput) -> Result<(), CoreError> {
        let Some(marker) = &self.require_marker else {
            return Ok(());
        };
        let path = input.paks.join(marker);
        let is_file = platform::fs_path(&path)
            .and_then(std::fs::symlink_metadata)
            .is_ok_and(|metadata| metadata.is_file());
        if is_file {
            Ok(())
        } else {
            Err(ModsError::NotAFixture.into())
        }
    }

    fn toggle_under_guard(
        &self,
        guard: &WriteGuard<'_>,
        input: &ScanInput,
        choice: &Choice<'_>,
        enable: bool,
        apply_while_running: bool,
    ) -> Result<Applied, CoreError> {
        let fresh: Refreshed = self.mods.refresh(input)?;
        let content: &ScanContent = &fresh.snapshot.content;
        let selection = match choice {
            Choice::Targets(request) => select_targets(content, &request.targets)?,
            Choice::DisableAll => select_disable_all(content),
        };
        let meta = self.mods.meta().load();
        let plan = plan_rows(&selection.rows, input, &meta.meta, enable)?;
        if !plan.ops.is_empty() {
            guard.preflight(&plan.ops)?;
            let running = self.game_running_now();
            if running && !apply_while_running {
                return Err(ModsError::GameRunning.into());
            }
            let retry = if running {
                RetryMode::Never
            } else {
                RetryMode::Normal
            };
            let options = BatchOptions {
                label: BATCH_LABEL,
                retry,
                fault: None,
            };
            guard.rename_batch(&plan.ops, &options)?;
        }
        if !meta.read_only {
            self.write_records(guard, &plan);
        }
        Ok(Applied::from(selection))
    }

    /// After a successful batch; a failure is logged and the toggle still succeeds (A7).
    fn write_records(&self, guard: &WriteGuard<'_>, plan: &Plan) {
        if plan.records.is_empty() {
            return;
        }
        let written = self.mods.meta().update(guard, |meta| {
            for (key, change) in &plan.records {
                match change {
                    RecordChange::Set(record) => meta.set_record(key, Some(record.clone())),
                    RecordChange::Clear => meta.set_record(key, None),
                    RecordChange::Keep => {}
                }
            }
        });
        if let Err(error) = written {
            log::error!("mod-meta.json not written; the next enable strips every -x: {error}");
        }
    }
}

struct Applied {
    skipped: Vec<super::toggle_apply::SkippedRow>,
    unmanaged: u32,
}

impl From<Selection<'_>> for Applied {
    fn from(selection: Selection<'_>) -> Self {
        Self {
            skipped: selection.skipped,
            unmanaged: selection.unmanaged,
        }
    }
}
