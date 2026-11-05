//! Applying a profile (spec §5.8): one batch under one write guard, after an auto snapshot.
//! Switching a group off goes the same way; only switching on records `last_applied`.

use super::api::{ApplyOutcome, ApplyRequest, position};
use super::capture::capture_states;
use super::diff::{ApplyDirection, DiffSkip, ProfileDiff, diff_switch};
use super::error::ProfileError;
use super::model::{
    LastApplied, Profile, ProfileMode, ProfileSource, ProfilesFile, SNAPSHOT_KEEP, now_rfc3339,
};
use crate::mods::{ModRow, ScanContent, ScanInput, plan_rows};
use crate::safe_fs::WriteGuard;
use crate::{Core, CoreError};

const APPLY_LABEL: &str = "profile_apply";

struct Applied {
    changed: u32,
    snapshot_id: Option<String>,
    skipped: Vec<DiffSkip>,
}

impl Core {
    pub fn profiles_apply(&self, request: &ApplyRequest) -> Result<ApplyOutcome, CoreError> {
        let (applied, refreshed) = self.run_managed(|guard, input, content| {
            self.apply_under_guard(guard, input, content, request)
        })?;
        Ok(ApplyOutcome {
            refreshed,
            changed: applied.changed,
            snapshot_id: applied.snapshot_id,
            skipped: applied.skipped,
        })
    }

    fn apply_under_guard(
        &self,
        guard: &WriteGuard<'_>,
        input: &ScanInput,
        content: &ScanContent,
        request: &ApplyRequest,
    ) -> Result<Applied, CoreError> {
        let retry = self.running_retry(request.apply_while_running)?;
        let loaded = self.profiles.load();
        let profile = loaded
            .file
            .profiles
            .iter()
            .find(|profile| profile.id == request.profile_id)
            .ok_or(ProfileError::NotFound)?;
        let diff = diff_switch(profile, content, request.direction);
        if diff.fingerprint != request.fingerprint {
            return Err(ProfileError::DiffChanged.into());
        }
        let switching_on = request.direction == ApplyDirection::On;
        if diff.changes.is_empty() {
            if switching_on {
                self.record_last_applied(guard, &request.profile_id);
            }
            return Ok(Applied {
                changed: 0,
                snapshot_id: None,
                skipped: diff.skipped,
            });
        }
        let snapshot_id = self.profiles.update(guard, |file| {
            Ok(add_snapshot(file, content, &request.profile_id))
        })?;
        let (enable_rows, disable_rows) = changed_rows(&diff, content);
        let meta = self.mods.meta().load();
        let disable = plan_rows(&disable_rows, input, &meta.meta, false)?;
        let enable = plan_rows(&enable_rows, input, &meta.meta, true)?;
        let mut ops = disable.ops;
        ops.extend(enable.ops);
        let mut records = disable.records;
        records.extend(enable.records);
        if !ops.is_empty() {
            guard.rename_batch(&ops, &self.batch_options(APPLY_LABEL, retry))?;
        }
        if !meta.read_only {
            self.write_records(guard, &records);
        }
        if switching_on {
            self.record_last_applied(guard, &request.profile_id);
        }
        Ok(Applied {
            changed: u32::try_from(diff.changes.len()).unwrap_or(u32::MAX),
            snapshot_id: Some(snapshot_id),
            skipped: diff.skipped,
        })
    }

    /// After the renames stand; a failure is only logged.
    fn record_last_applied(&self, guard: &WriteGuard<'_>, profile_id: &str) {
        let written = self.profiles.update(guard, |file| {
            let at = now_rfc3339();
            let index = position(file, profile_id)?;
            file.profiles[index].last_applied = Some(at.clone());
            file.last_applied = Some(LastApplied {
                profile_id: profile_id.to_owned(),
                at,
            });
            Ok(())
        });
        if let Err(error) = written {
            log::error!("profiles.json: last applied not recorded: {error}");
        }
    }
}

/// The rows of the diff's changes, split into (enable, disable).
fn changed_rows<'a>(
    diff: &ProfileDiff,
    content: &'a ScanContent,
) -> (Vec<&'a ModRow>, Vec<&'a ModRow>) {
    let mut enable = Vec::new();
    let mut disable = Vec::new();
    for change in &diff.changes {
        let row = content
            .rows
            .iter()
            .find(|row| row.mod_ref == change.mod_ref && row.key == change.key);
        if let Some(row) = row {
            if change.enable {
                enable.push(row);
            } else {
                disable.push(row);
            }
        }
    }
    (enable, disable)
}

/// An Exclusive, unnamed snapshot of the current states; keeps the newest `SNAPSHOT_KEEP`.
fn add_snapshot(file: &mut ProfilesFile, content: &ScanContent, of: &str) -> String {
    let snapshot = Profile::fresh(
        String::new(),
        String::new(),
        ProfileMode::Exclusive,
        capture_states(content),
        ProfileSource::Snapshot { of: of.to_owned() },
    );
    let id = snapshot.id.clone();
    file.profiles.push(snapshot);
    let mut snapshots: Vec<(String, usize)> = file
        .profiles
        .iter()
        .enumerate()
        .filter(|(_, profile)| profile.is_snapshot())
        .map(|(index, profile)| (profile.created.clone(), index))
        .collect();
    if snapshots.len() > SNAPSHOT_KEEP {
        snapshots.sort();
        let drop_count = snapshots.len() - SNAPSHOT_KEEP;
        let dropped: Vec<usize> = snapshots
            .iter()
            .take(drop_count)
            .map(|(_, index)| *index)
            .collect();
        let mut index = 0;
        file.profiles.retain(|_| {
            let keep = !dropped.contains(&index);
            index += 1;
            keep
        });
    }
    id
}
