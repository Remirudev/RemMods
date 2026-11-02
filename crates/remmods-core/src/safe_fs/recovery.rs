//! Rolls back the journal entries of dead sessions (inside `OpGate::write`, under the exclusive
//! app lock). Rules, per rename op in reverse order: placeholders first; `to` only → rename back;
//! `from` only or neither → not done; both → not done when the target is still held by the source
//! of an earlier op that has not run (replacing batches), a link-fallback leftover is removed,
//! anything else is stuck. Nothing is ever overwritten. Install staging entries come after every rename entry (a
//! rolled-back commit batch puts the staging folder back first): a missing folder drops the entry,
//! an inert temp folder in a mod root is removed, anything else is stuck.

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use super::FsError;
use super::aside::{check_inert_temp_tree, remove_tree};
use super::batch::{RetryMode, rename_with_retry};
use super::journal::{Entry, Journal, OpRecord, RecoveryReport};
use super::names::{fold_key, key_text};
use super::remove::{remove_empty_dir_in, remove_file_in};
use super::scope::{Scope, ScopeChecker};
use super::sessions::{self, Liveness};
use crate::platform;

struct RenameRecord {
    from: PathBuf,
    to: PathBuf,
    is_dir: bool,
}

enum OpOutcome {
    RolledBack,
    NotDone,
    Stuck,
}

pub(crate) fn recover(journal: &Journal, scope: &Scope) -> Result<RecoveryReport, FsError> {
    let mut report = RecoveryReport::default();
    let mut alive: HashSet<String> = HashSet::new();
    let mut dead: HashMap<String, Option<File>> = HashMap::new();
    let mut staging: Vec<Entry> = Vec::new();
    for entry in journal.entries()? {
        let owner = &entry.owner.session;
        if owner == journal.session() || alive.contains(owner) {
            continue;
        }
        if !dead.contains_key(owner) {
            match sessions::probe(journal.sessions_dir(), owner) {
                Liveness::Alive => {
                    alive.insert(owner.clone());
                    continue;
                }
                Liveness::Dead(lock) => {
                    dead.insert(owner.clone(), lock);
                }
            }
        }
        report.entries += 1;
        if is_staging_entry(&entry) {
            staging.push(entry);
        } else {
            recover_entry(journal, scope, &entry, &mut report)?;
        }
    }
    for entry in &staging {
        recover_staging(journal, scope, entry, &mut report)?;
    }
    for (session, lock) in dead {
        sessions::remove_dead(journal.sessions_dir(), &session, lock);
    }
    log_report(&report);
    Ok(report)
}

fn recover_entry(
    journal: &Journal,
    scope: &Scope,
    entry: &Entry,
    report: &mut RecoveryReport,
) -> Result<(), FsError> {
    let Some((renames, placeholders)) = decode_in_scope(scope, entry) else {
        log::warn!(
            "journal entry {} has a path outside the scope; left as is",
            entry.id
        );
        report.foreign += 1;
        return Ok(());
    };
    let mut stuck = 0;
    for (index, op) in renames.iter().enumerate().rev() {
        let earlier = &renames[..index];
        match recover_op(journal, scope, &entry.id, op, earlier, &placeholders) {
            OpOutcome::RolledBack => report.rolled_back += 1,
            OpOutcome::NotDone => {}
            OpOutcome::Stuck => {
                log::warn!(
                    "journal entry {}: {} -> {} is stuck",
                    entry.id,
                    op.from.display(),
                    op.to.display()
                );
                stuck += 1;
            }
        }
    }
    if stuck == 0 {
        journal.remove(&entry.id)
    } else {
        report.stuck += stuck;
        journal.mark_stuck(&entry.id)
    }
}

/// Every path decoded and in scope, else None (the entry is foreign).
fn decode_in_scope(scope: &Scope, entry: &Entry) -> Option<(Vec<RenameRecord>, Vec<PathBuf>)> {
    let mut checker = ScopeChecker::new(scope);
    let mut renames = Vec::new();
    let mut placeholders = Vec::new();
    for op in &entry.ops {
        match op {
            OpRecord::Rename { is_dir, from, to } => {
                let from = from.decode()?;
                let to = to.decode()?;
                checker.check(&from).ok()?;
                checker.check(&to).ok()?;
                renames.push(RenameRecord {
                    from,
                    to,
                    is_dir: *is_dir,
                });
            }
            OpRecord::Placeholder { path } => {
                let path = path.decode()?;
                checker.check(&path).ok()?;
                placeholders.push(path);
            }
            OpRecord::Staging { .. } => return None,
        }
    }
    Some((renames, placeholders))
}

fn is_staging_entry(entry: &Entry) -> bool {
    entry
        .ops
        .iter()
        .any(|op| matches!(op, OpRecord::Staging { .. }))
}

/// One staging folder per entry (the only op).
fn recover_staging(
    journal: &Journal,
    scope: &Scope,
    entry: &Entry,
    report: &mut RecoveryReport,
) -> Result<(), FsError> {
    let path = match entry.ops.as_slice() {
        [OpRecord::Staging { path, .. }] => path.decode(),
        _ => None,
    };
    let Some(path) = path.filter(|path| scope.check(path).is_ok()) else {
        log::warn!(
            "journal entry {} names a staging folder outside the scope; left as is",
            entry.id
        );
        report.foreign += 1;
        return Ok(());
    };
    if !exists(&path) {
        return journal.remove(&entry.id);
    }
    match check_inert_temp_tree(scope, &path) {
        Ok(raw) => {
            journal.remove(&entry.id)?;
            match remove_tree(&raw, &path) {
                Ok(()) => report.discarded += 1,
                Err(error) => log::warn!(
                    "removing the staging folder {} failed; left for Repair: {error}",
                    path.display()
                ),
            }
            Ok(())
        }
        Err(error) => {
            log::warn!(
                "journal entry {}: staging folder {} is stuck: {error}",
                entry.id,
                path.display()
            );
            report.stuck += 1;
            journal.mark_stuck(&entry.id)
        }
    }
}

fn recover_op(
    journal: &Journal,
    scope: &Scope,
    entry: &str,
    op: &RenameRecord,
    earlier: &[RenameRecord],
    placeholders: &[PathBuf],
) -> OpOutcome {
    let at_from = placeholders.contains(&op.from);
    let at_to = placeholders.contains(&op.to);
    if at_from && at_to {
        return OpOutcome::Stuck;
    }
    let placeholder = match (at_from, at_to) {
        (true, _) => Some((&op.from, &op.to)),
        (_, true) => Some((&op.to, &op.from)),
        _ => None,
    };
    if let Some((placeholder, other)) = placeholder
        && is_empty_dir(placeholder)
        && exists(other)
        && let Err(error) = remove_empty_dir_in(scope, placeholder)
    {
        log::warn!("removing placeholder failed: {error}");
        return OpOutcome::Stuck;
    }
    match (exists(&op.from), exists(&op.to)) {
        (false, true) => {
            let undo = rename_with_retry(
                scope,
                &op.to,
                &op.from,
                op.is_dir,
                RetryMode::Normal,
                Some((journal, entry)),
                None,
            );
            match undo {
                Ok(_) => OpOutcome::RolledBack,
                Err(error) => {
                    log::warn!("rolling back failed: {error}");
                    OpOutcome::Stuck
                }
            }
        }
        (true, true) if is_link_leftover(op) => match remove_file_in(scope, &op.to) {
            Ok(()) => OpOutcome::RolledBack,
            Err(error) => {
                log::warn!("removing the link-fallback leftover failed: {error}");
                OpOutcome::Stuck
            }
        },
        (true, true) if waits_for_vacate(op, earlier) => OpOutcome::NotDone,
        (true, true) => OpOutcome::Stuck,
        (true, false) | (false, false) => OpOutcome::NotDone,
    }
}

/// `rename_batch_replacing`: the target is the (fold-equal) source of an earlier op that has not
/// run yet (its source is still there, its target is not), so what exists at the target is that
/// op's source and this op has not run either.
fn waits_for_vacate(op: &RenameRecord, earlier: &[RenameRecord]) -> bool {
    earlier.iter().any(|before| {
        same_folded(&before.from, &op.to) && exists(&before.from) && !exists(&before.to)
    })
}

fn same_folded(a: &Path, b: &Path) -> bool {
    let name = |path: &Path| path.file_name().map(|name| fold_key(&key_text(name)));
    a.parent() == b.parent() && name(a).is_some() && name(a) == name(b)
}

/// Both names are non-folders, listed as two exact entries in their folder (on case-insensitive
/// file systems two names can be one entry) and share one file id: the link fallback stopped
/// between link and unlink.
fn is_link_leftover(op: &RenameRecord) -> bool {
    if op.is_dir || is_dir(&op.from) || is_dir(&op.to) {
        return false;
    }
    let (Some(parent), Some(from_name), Some(to_name)) =
        (op.from.parent(), op.from.file_name(), op.to.file_name())
    else {
        return false;
    };
    let Ok(listing) = platform::fs_path(parent).and_then(fs::read_dir) else {
        return false;
    };
    let names: Vec<_> = listing
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .collect();
    if !(names.iter().any(|name| name == from_name) && names.iter().any(|name| name == to_name)) {
        return false;
    }
    match (platform::file_id(&op.from), platform::file_id(&op.to)) {
        (Ok(from_id), Ok(to_id)) => from_id == to_id,
        _ => false,
    }
}

/// A dangling link exists too.
fn exists(path: &Path) -> bool {
    platform::fs_path(path)
        .and_then(fs::symlink_metadata)
        .is_ok()
}

fn is_dir(path: &Path) -> bool {
    platform::fs_path(path)
        .and_then(fs::symlink_metadata)
        .is_ok_and(|metadata| metadata.is_dir())
}

fn is_empty_dir(path: &Path) -> bool {
    is_dir(path)
        && platform::fs_path(path)
            .and_then(fs::read_dir)
            .is_ok_and(|mut entries| entries.next().is_none())
}

fn log_report(report: &RecoveryReport) {
    if report.entries == 0 {
        return;
    }
    log::info!(
        "journal recovery: {} entries, {} ops rolled back, {} staging folders removed",
        report.entries,
        report.rolled_back,
        report.discarded
    );
    if report.stuck > 0 || report.foreign > 0 {
        log::warn!(
            "journal recovery: {} stuck ops, {} foreign entries kept",
            report.stuck,
            report.foreign
        );
    }
}
