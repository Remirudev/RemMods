//! `install_start` (spec §5.4): validate, guard A (targets, space, root, staging), extraction
//! without the lock, guard B (re-checks, one commit batch, meta, Trash); any error or cancel
//! before the commit discards the staging folders under a fresh guard on the job's scope.

use std::collections::HashSet;
use std::path::PathBuf;

use super::api::{check_fingerprint, open_source};
use super::commit::{PickOps, pick_ops, trash_aside};
use super::error::{InstallBlock, InstallError, blocked};
use super::existing::{find_existing, root_folder};
use super::inspect::{available_space, needs_space};
use super::job::{Job, Pick, check_targets, validate};
use super::model::{InstallOutcome, InstallPhase, InstallProgress, InstalledMod, StartRequest};
use super::service::{InstallHooks, JobSlot, StoredInspection};
use super::stage::{InstallTarget, build_plan, run_extract};
use crate::archive::{ArchiveError, ExtractPlan, Limits};
use crate::mods::{ModRow, RootKind, ScanContent};
use crate::safe_fs::{RetryMode, WriteGuard};
use crate::{Core, CoreError};

const INSTALL_LABEL: &str = "install";

type Progress<'a> = &'a (dyn Fn(&InstallProgress) + Send + Sync);

/// Totals of the files a start stages.
#[derive(Clone, Copy)]
struct Totals {
    bytes: u64,
    files: u32,
}

impl Core {
    pub fn install_start(
        &self,
        request: &StartRequest,
        progress: Progress<'_>,
    ) -> Result<InstallOutcome, CoreError> {
        let stored = self
            .install
            .get(&request.inspection)
            .ok_or(InstallError::UnknownInspection)?;
        let (picks, dropped) = validate(&stored, request)?;
        let slot = self.install.begin_job()?;
        let retry = self.running_retry(request.apply_while_running)?;
        check_fingerprint(&stored)?;
        let hooks = self.install.hooks();
        let (plan, inert) = {
            let stored_picks: Vec<_> = picks.iter().map(|pick| pick.stored).collect();
            build_plan(&stored_picks, &dropped)
        };
        let totals = Totals {
            bytes: plan.files.iter().map(|file| file.size).sum(),
            files: u32::try_from(plan.files.len()).unwrap_or(u32::MAX),
        };
        let mut job = self.prepare_job(&stored, &picks, &hooks, totals.bytes)?;
        report(progress, InstallPhase::Preparing, totals, 0, 0);
        let staged = self.stage(&stored, &job, &slot, &hooks, progress, (&plan, inert));
        let result = staged.and_then(|inert| {
            report(
                progress,
                InstallPhase::Committing,
                totals,
                totals.bytes,
                totals.files,
            );
            self.commit(&stored, &picks, &mut job, &inert, retry)
        });
        match result {
            Ok(outcome) => Ok(outcome),
            Err(error) => {
                report(progress, InstallPhase::CleaningUp, totals, 0, 0);
                self.clean_up(job);
                if let Err(refresh_error) = self.mods_refresh() {
                    log::warn!("refresh after a failed install failed: {refresh_error}");
                }
                Err(error)
            }
        }
    }

    /// Guard A: fresh scan, target re-check, space, the root folder, one staging folder per pick.
    fn prepare_job(
        &self,
        stored: &StoredInspection,
        picks: &[Pick<'_>],
        hooks: &InstallHooks,
        bytes: u64,
    ) -> Result<Job, CoreError> {
        let input = self.mods_scan_input()?;
        let mut guard = self.ops.write(self.manage_scope(&input)?)?;
        self.check_marker(&input)?;
        let fresh = self.mods.refresh(&input)?;
        let kind = stored.inspection.root;
        let root = root_folder(&fresh.snapshot.content, &input, kind);
        check_targets(&fresh.snapshot.content, kind, root.as_deref(), picks)?;
        let space_at = root.clone().unwrap_or_else(|| input.paks.clone());
        let available = hooks.available_space.or_else(|| available_space(&space_at));
        let needed = needs_space(bytes);
        if let Some(available) = available
            && available < needed
        {
            return Err(InstallError::NoSpace { needed, available }.into());
        }
        let root = match root {
            Some(root) => root,
            None => guard.create_mod_root(kind.folder())?,
        };
        let mut staging = Vec::with_capacity(picks.len());
        for _ in picks {
            let suffix = uuid::Uuid::new_v4().simple().to_string();
            match guard.create_staging(&root, &suffix) {
                Ok(dir) => staging.push(dir),
                Err(error) => {
                    for dir in staging {
                        if let Err(discard) = guard.discard_staging(dir) {
                            log::warn!("discarding a staging folder failed: {discard}");
                        }
                    }
                    return Err(error.into());
                }
            }
        }
        Ok(Job {
            scope: guard.scope().clone(),
            paks: input.paks,
            root,
            staging,
        })
    }

    /// Extraction without the lock; then the source must be unchanged and the job not
    /// cancelled. Returns the inert files per pick.
    fn stage(
        &self,
        stored: &StoredInspection,
        job: &Job,
        slot: &JobSlot<'_>,
        hooks: &InstallHooks,
        progress: Progress<'_>,
        (plan, inert): (&ExtractPlan, HashSet<u32>),
    ) -> Result<Vec<Vec<PathBuf>>, CoreError> {
        let reader = open_source(&stored.source, stored.folder, &Limits::default())?;
        if reader.listing().entries.len() != stored.entry_count {
            return Err(InstallError::SourceChanged.into());
        }
        let mut target = InstallTarget::new(&job.staging, inert, hooks.panic_in_target);
        match run_extract(reader, plan, &mut target, &slot.cancel, progress) {
            Ok(_) => {}
            Err(ArchiveError::Cancelled) => return Err(InstallError::Cancelled.into()),
            Err(error) => return Err(InstallError::Archive(error).into()),
        }
        check_fingerprint(stored)?;
        if let Some(after_stage) = &hooks.after_stage {
            let folders: Vec<PathBuf> = job
                .staging
                .iter()
                .map(|dir| dir.path().to_path_buf())
                .collect();
            after_stage(&folders);
        }
        if slot.cancel.is_cancelled() {
            return Err(InstallError::Cancelled.into());
        }
        Ok(target.inert)
    }

    /// Guard B: the same Paks, fresh re-checks, one all-or-none batch, then meta and Trash.
    fn commit(
        &self,
        stored: &StoredInspection,
        picks: &[Pick<'_>],
        job: &mut Job,
        inert: &[Vec<PathBuf>],
        retry: RetryMode,
    ) -> Result<InstallOutcome, CoreError> {
        let input = self.mods_scan_input()?;
        if input.paks != job.paks {
            return Err(blocked(InstallBlock::TargetChanged).into());
        }
        let guard = self.ops.write(self.manage_scope(&input)?)?;
        self.check_marker(&input)?;
        let fresh = self.mods.refresh(&input)?;
        let content = &fresh.snapshot.content;
        let kind = stored.inspection.root;
        let root = root_folder(content, &input, kind);
        if root.as_deref() != Some(job.root.as_path()) {
            return Err(blocked(InstallBlock::TargetChanged).into());
        }
        check_targets(content, kind, Some(&job.root), picks)?;
        let mut ops = Vec::new();
        let mut asides = Vec::new();
        for ((pick, staging), inert) in picks.iter().zip(&job.staging).zip(inert) {
            guard.record_inert(staging, inert)?;
            let old = existing_row(content, kind, pick);
            let target = job.root.join(pick.final_name());
            let PickOps {
                ops: pick_ops,
                aside,
            } = pick_ops(staging, inert, &target, old.map(|row| (row, &input)))?;
            ops.extend(pick_ops);
            asides.push(aside.zip(old.map(|row| row.key.clone())));
        }
        guard.rename_batch_replacing(&ops, &self.batch_options(INSTALL_LABEL, retry))?;
        for dir in std::mem::take(&mut job.staging) {
            if let Err(error) = guard.finish_staging(dir) {
                log::warn!("dropping a staging entry after the commit failed: {error}");
            }
        }
        let root_name = job.root.file_name().map_or_else(
            || kind.folder().to_owned(),
            |name| name.to_string_lossy().into_owned(),
        );
        let installed = self.after_commit(&guard, stored, picks, &asides, &root_name);
        let refreshed = self.final_refresh(&input)?;
        let installed = installed
            .into_iter()
            .map(|mut item| {
                item.mod_ref = refreshed
                    .snapshot
                    .content
                    .rows
                    .iter()
                    .find(|row| row.key == item.key)
                    .map(|row| row.mod_ref.clone());
                item
            })
            .collect();
        Ok(InstallOutcome {
            refreshed,
            installed,
        })
    }

    /// mod-meta.json (added, source, no disable record) and the Trash for replaced mods. Errors
    /// here are logged: the install itself is done.
    fn after_commit(
        &self,
        guard: &WriteGuard<'_>,
        stored: &StoredInspection,
        picks: &[Pick<'_>],
        asides: &[Option<(PathBuf, String)>],
        root_name: &str,
    ) -> Vec<InstalledMod> {
        let kind = stored.inspection.root;
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let keys: Vec<String> = picks
            .iter()
            .map(|pick| format!("{}:{}", kind.key_prefix(), pick.final_name()))
            .collect();
        let source = stored.inspection.source_name.clone();
        let meta = self.mods.meta().update(guard, |meta| {
            for (key, aside) in keys.iter().zip(asides) {
                if let Some((_, old_key)) = aside
                    && old_key != key
                {
                    meta.remove(old_key);
                }
                let entry = meta.mods.entry(key.clone()).or_default();
                entry.added = Some(now.clone());
                entry.source = Some(source.clone());
                entry.disabled = None;
            }
        });
        if let Err(error) = meta {
            log::warn!("writing mod-meta.json after an install failed: {error}");
        }
        picks
            .iter()
            .zip(keys)
            .zip(asides)
            .map(|((pick, key), aside)| InstalledMod {
                key,
                mod_ref: None,
                rel: format!("{root_name}/{}", pick.final_name()),
                files: pick.candidate.files,
                replaced: aside
                    .as_ref()
                    .map(|(folder, _)| trash_aside(guard, &self.trash_env, folder)),
            })
            .collect()
    }

    /// Discards what is left of the staging folders under a fresh guard on the job's own scope
    /// (the game folder may have changed meanwhile). Failures are logged; the folders stay inert.
    fn clean_up(&self, job: Job) {
        if job.staging.is_empty() {
            return;
        }
        match self.ops.write(job.scope.clone()) {
            Ok(guard) => {
                for dir in job.staging {
                    if let Err(error) = guard.discard_staging(dir) {
                        log::warn!("discarding a staging folder failed: {error}");
                    }
                }
            }
            Err(error) => log::warn!("no guard to discard the staging folders: {error}"),
        }
    }
}

fn existing_row<'a>(
    content: &'a ScanContent,
    kind: RootKind,
    pick: &Pick<'_>,
) -> Option<&'a ModRow> {
    let (existing, _) = find_existing(content, kind, pick.final_name());
    let existing = existing?;
    content
        .rows
        .iter()
        .find(|row| row.key == existing.key && row.mod_ref == existing.mod_ref)
}

fn report(progress: Progress<'_>, phase: InstallPhase, totals: Totals, bytes: u64, files: u32) {
    progress(&InstallProgress {
        phase,
        done_bytes: bytes,
        total_bytes: totals.bytes,
        done_files: files,
        total_files: totals.files,
    });
}
