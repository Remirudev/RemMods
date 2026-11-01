//! Rolls back the journal entries of dead sessions (inside `OpGate::write`, under the exclusive
//! app lock). Rules, per rename op in reverse order: placeholders first; `to` only → rename back;
//! `from` only or neither → not done; both → a link-fallback leftover is removed, anything else is
//! stuck. Nothing is ever overwritten.

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use super::FsError;
use super::batch::{RetryMode, rename_with_retry};
use super::journal::{Entry, Journal, OpRecord, RecoveryReport};
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
        recover_entry(journal, scope, &entry, &mut report)?;
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
    for op in renames.iter().rev() {
        match recover_op(journal, scope, &entry.id, op, &placeholders) {
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
        }
    }
    Some((renames, placeholders))
}

fn recover_op(
    journal: &Journal,
    scope: &Scope,
    entry: &str,
    op: &RenameRecord,
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
        (true, true) => OpOutcome::Stuck,
        (true, false) | (false, false) => OpOutcome::NotDone,
    }
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
        "journal recovery: {} entries, {} ops rolled back",
        report.entries,
        report.rolled_back
    );
    if report.stuck > 0 || report.foreign > 0 {
        log::warn!(
            "journal recovery: {} stuck ops, {} foreign entries kept",
            report.stuck,
            report.foreign
        );
    }
}
