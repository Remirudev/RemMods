//! A start's checks (spec §5.4 steps 1, 4 and 6): the request against its inspection, and the
//! targets against a fresh scan. Pure apart from listing the root folder.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::error::{InstallBlock, InstallError, blocked};
use super::existing::{find_existing, foreign_entries};
use super::model::{Candidate, IssueCode, IssueSeverity, StartRequest};
use super::service::{StoredCandidate, StoredInspection};
use crate::mods::{RootKind, ScanContent};
use crate::safe_fs::names::fold_key;
use crate::safe_fs::{Scope, StagingDir};

/// A validated pick.
pub(crate) struct Pick<'a> {
    pub(crate) candidate: &'a Candidate,
    pub(crate) stored: &'a StoredCandidate,
}

impl Pick<'_> {
    pub(crate) fn final_name(&self) -> &str {
        self.candidate
            .fixed_name
            .as_deref()
            .unwrap_or(&self.candidate.name)
    }
}

/// What guard A set up: the staging folders and the scope they live in.
pub(crate) struct Job {
    pub(crate) scope: Scope,
    pub(crate) paks: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) staging: Vec<StagingDir>,
}

/// Step 1: every check that needs no disk. Returns the picks and the dropped clash members.
pub(crate) fn validate<'a>(
    stored: &'a StoredInspection,
    request: &StartRequest,
) -> Result<(Vec<Pick<'a>>, HashSet<u32>), InstallError> {
    let inspection = &stored.inspection;
    if inspection.blocked {
        return Err(blocked(InstallBlock::Issues));
    }
    let needs_fix = inspection
        .issues
        .iter()
        .any(|issue| issue.severity == IssueSeverity::Warn && issue.code == IssueCode::InvalidName);
    if needs_fix && !request.auto_fix {
        return Err(blocked(InstallBlock::InvalidName));
    }
    let dropped = clash_drops(stored, request)?;
    let picks = resolve_picks(stored, request)?;
    let mut names = HashSet::new();
    for pick in &picks {
        if !names.insert(fold_key(pick.final_name())) {
            return Err(blocked(InstallBlock::InvalidName));
        }
    }
    Ok((picks, dropped))
}

/// Each clash needs exactly one pick; the members not kept are not extracted.
fn clash_drops(
    stored: &StoredInspection,
    request: &StartRequest,
) -> Result<HashSet<u32>, InstallError> {
    let mut dropped = HashSet::new();
    for clash in &stored.inspection.clashes {
        let mut picks = request.clashes.iter().filter(|pick| pick.id == clash.id);
        let (Some(pick), None) = (picks.next(), picks.next()) else {
            return Err(blocked(InstallBlock::UnresolvedClash));
        };
        let members = usize::try_from(clash.id)
            .ok()
            .and_then(|id| stored.clash_members.get(id))
            .ok_or_else(|| blocked(InstallBlock::UnresolvedClash))?;
        let keep = usize::try_from(pick.keep).unwrap_or(usize::MAX);
        if keep >= members.len() {
            return Err(blocked(InstallBlock::UnresolvedClash));
        }
        dropped.extend(
            members
                .iter()
                .enumerate()
                .filter(|(position, _)| *position != keep)
                .map(|(_, index)| *index),
        );
    }
    let unknown = request.clashes.iter().any(|pick| {
        stored
            .inspection
            .clashes
            .iter()
            .all(|clash| clash.id != pick.id)
    });
    if unknown {
        return Err(blocked(InstallBlock::UnresolvedClash));
    }
    Ok(dropped)
}

fn resolve_picks<'a>(
    stored: &'a StoredInspection,
    request: &StartRequest,
) -> Result<Vec<Pick<'a>>, InstallError> {
    if request.picks.is_empty() {
        return Err(blocked(InstallBlock::NoSelection));
    }
    let mut picks = Vec::new();
    let mut ids = HashSet::new();
    let mut groups = HashSet::new();
    for pick in &request.picks {
        let candidate = stored
            .inspection
            .candidates
            .iter()
            .find(|candidate| candidate.id == pick.id);
        let files = stored
            .candidates
            .iter()
            .find(|candidate| candidate.id == pick.id);
        let (Some(candidate), Some(files)) = (candidate, files) else {
            return Err(blocked(InstallBlock::NoSelection));
        };
        if !ids.insert(pick.id) {
            return Err(blocked(InstallBlock::NoSelection));
        }
        if let Some(group) = candidate.group
            && !groups.insert(group)
        {
            return Err(blocked(InstallBlock::VariantConflict));
        }
        match (&candidate.existing, pick.replace) {
            (Some(existing), _) if existing.ambiguous => {
                return Err(blocked(InstallBlock::Ambiguous));
            }
            (Some(_), false) => return Err(blocked(InstallBlock::ReplaceNotConfirmed)),
            (None, true) => return Err(blocked(InstallBlock::ReplaceMismatch)),
            _ => {}
        }
        picks.push(Pick {
            candidate,
            stored: files,
        });
    }
    Ok(picks)
}

/// Steps 4 and 6: every pick's existing match is still the inspected one, and no other entry
/// fold-equal to its name appeared in the root folder.
pub(crate) fn check_targets(
    content: &ScanContent,
    kind: RootKind,
    root: Option<&Path>,
    picks: &[Pick<'_>],
) -> Result<(), InstallError> {
    let changed = || blocked(InstallBlock::TargetChanged);
    for pick in picks {
        let (current, _) = find_existing(content, kind, pick.final_name());
        let same = match (&current, &pick.candidate.existing) {
            (None, None) => true,
            (Some(now), Some(then)) => now.key == then.key && now.mod_ref == then.mod_ref,
            _ => false,
        };
        if !same {
            return Err(changed());
        }
        let except = current
            .as_ref()
            .and_then(|existing| existing.rel.rsplit_once('/'))
            .map(|(_, folder)| folder);
        if root.is_some_and(|root| foreign_entries(root, pick.final_name(), except)) {
            return Err(changed());
        }
    }
    Ok(())
}
