//! Applying a profile (spec §5.8): one batch under one write guard, after an auto snapshot.
//! Switching a group off goes the same way; only switching on records `last_applied`. Several
//! groups switched together are one batch, one snapshot and one Undo.

use std::collections::BTreeSet;

use super::api::{ApplyOutcome, ApplyRequest, position, switched_profile};
use super::capture::capture_states;
use super::diff::{ApplyDirection, DiffSkip, ProfileDiff, diff_switch};
use super::error::ProfileError;
use super::model::{
    LastApplied, Profile, ProfileMode, ProfileSource, ProfilesFile, SNAPSHOT_KEEP, now_rfc3339,
};
use crate::mods::{ModRow, ScanContent, ScanInput, plan_rows};
use crate::safe_fs::WriteGuard;
use crate::skins::LinkedSkins;
use crate::{Core, CoreError};

const APPLY_LABEL: &str = "profile_apply";

struct Applied {
    changed: u32,
    snapshot_id: Option<String>,
    skipped: Vec<DiffSkip>,
    /// The skin sets the switched-on groups link.
    skin_sets: BTreeSet<String>,
}

impl Core {
    pub fn profiles_apply(&self, request: &ApplyRequest) -> Result<ApplyOutcome, CoreError> {
        let (applied, refreshed) = self.run_managed(|guard, input, content| {
            self.apply_under_guard(guard, input, content, request)
        })?;
        let skins = self.linked_skins(&applied.skin_sets);
        Ok(ApplyOutcome {
            refreshed,
            changed: applied.changed,
            snapshot_id: applied.snapshot_id,
            skipped: applied.skipped,
            skins,
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
        let profile = switched_profile(&loaded.file, &request.profile_ids)?;
        let diff = diff_switch(&profile, content, request.direction);
        if diff.fingerprint != request.fingerprint {
            return Err(ProfileError::DiffChanged.into());
        }
        let switching_on = request.direction == ApplyDirection::On;
        let skin_sets = if switching_on {
            linked_sets(&loaded.file, &request.profile_ids)
        } else {
            BTreeSet::new()
        };
        if diff.changes.is_empty() {
            if switching_on {
                self.record_last_applied(guard, &request.profile_ids);
            }
            return Ok(Applied {
                changed: 0,
                snapshot_id: None,
                skipped: diff.skipped,
                skin_sets,
            });
        }
        // Undo applies this snapshot, so it remembers the skins to go back to.
        let skins_before = if skin_sets.is_empty() {
            None
        } else {
            self.active_skin_set()
        };
        let snapshot_id = self.profiles.update(guard, |file| {
            Ok(add_snapshot(
                file,
                content,
                &profile.id,
                skins_before.as_deref(),
            ))
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
            self.record_last_applied(guard, &request.profile_ids);
        }
        Ok(Applied {
            changed: u32::try_from(diff.changes.len()).unwrap_or(u32::MAX),
            snapshot_id: Some(snapshot_id),
            skipped: diff.skipped,
            skin_sets,
        })
    }

    /// The game holds one set, so groups that link different sets leave the skins alone. A link to
    /// a set that is gone means nothing.
    fn linked_skins(&self, skin_sets: &BTreeSet<String>) -> Option<LinkedSkins> {
        if skin_sets.is_empty() {
            return None;
        }
        let view = self.skins_view();
        let mut existing = skin_sets
            .iter()
            .filter(|id| view.sets.iter().any(|set| set.set.id == **id));
        match (existing.next(), existing.next()) {
            (Some(id), None) => self.switch_linked_skins(id),
            (Some(_), Some(_)) => Some(LinkedSkins::Several),
            (None, _) => None,
        }
    }

    /// After the renames stand; a failure is only logged.
    fn record_last_applied(&self, guard: &WriteGuard<'_>, profile_ids: &[String]) {
        let written = self.profiles.update(guard, |file| {
            let at = now_rfc3339();
            for id in profile_ids {
                let index = position(file, id)?;
                file.profiles[index].last_applied = Some(at.clone());
                file.last_applied = Some(LastApplied {
                    profile_id: id.clone(),
                    at: at.clone(),
                });
            }
            Ok(())
        });
        if let Err(error) = written {
            log::error!("profiles.json: last applied not recorded: {error}");
        }
    }
}

fn linked_sets(file: &ProfilesFile, profile_ids: &[String]) -> BTreeSet<String> {
    file.profiles
        .iter()
        .filter(|profile| profile_ids.contains(&profile.id))
        .filter_map(|profile| profile.skin_set().map(str::to_owned))
        .collect()
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
fn add_snapshot(
    file: &mut ProfilesFile,
    content: &ScanContent,
    of: &str,
    skins: Option<&str>,
) -> String {
    let mut snapshot = Profile::fresh(
        String::new(),
        String::new(),
        ProfileMode::Exclusive,
        capture_states(content),
        ProfileSource::Snapshot { of: of.to_owned() },
    );
    snapshot.set_skin_set(skins);
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
