//! All-or-none rename batches: preflight (writes nothing), journal, run, and on failure an undo
//! of every done op in reverse.

use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Instant;

use super::FsError;
use super::journal::Journal;
use super::preflight::{list_folded, preflight};
use super::rename::{RenameMethod, STUCK_UNLINK_OP, rename_one};
use super::scope::Scope;
use crate::platform::{self, OsErrorClass, RetryPolicy};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenameOp {
    pub from: PathBuf,
    pub to: PathBuf,
    pub is_dir: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetryMode {
    Normal,
    Never,
}

/// Test-only fault injection. Op indices count the expanded ops (a case-only op becomes two).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FaultPlan {
    /// Inject an error instead of running op k.
    pub fail_before: Option<usize>,
    /// That error; None → `io::ErrorKind::Other`.
    pub raw_os_error: Option<i32>,
    /// Return Err once k ops are done, before op k runs: no rollback, journal entry kept.
    pub crash_after: Option<usize>,
    /// The rollback of op k fails, so it is stuck.
    pub fail_rollback_of: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct BatchOptions {
    pub label: &'static str,
    pub retry: RetryMode,
    pub fault: Option<FaultPlan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchReport {
    pub done: usize,
    pub methods: Vec<RenameMethod>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StuckOp {
    pub from: PathBuf,
    pub to: PathBuf,
}

#[derive(Debug, thiserror::Error)]
#[error("batch failed at op {failed}: {cause}")]
pub struct BatchError {
    pub failed: usize,
    pub cause: FsError,
    pub rolled_back: usize,
    pub stuck: Vec<StuckOp>,
}

pub(crate) fn run(
    scope: &Scope,
    journal: &Journal,
    ops: &[RenameOp],
    options: &BatchOptions,
) -> Result<BatchReport, FsError> {
    let planned = preflight(scope, ops)?;
    let Some(last) = planned.ops.last() else {
        return Ok(BatchReport {
            done: 0,
            methods: Vec::new(),
        });
    };
    let entry = journal.begin(options.label, &planned.ops)?;
    let fault = options.fault.clone().unwrap_or_default();
    let mut methods = Vec::with_capacity(planned.ops.len());
    for (index, op) in planned.ops.iter().enumerate() {
        if fault.crash_after == Some(index) {
            return Err(crash_error(index, op));
        }
        let outcome = run_op(scope, journal, &entry, op, index, &fault, options.retry);
        let outcome = outcome.and_then(|method| {
            if planned.verify.get(index) == Some(&true) {
                verify_exact_name(&op.to).map_err(|error| (error, true))?;
            }
            Ok(method)
        });
        match outcome {
            Ok(method) => methods.push(method),
            Err((cause, done)) => {
                let done_ops = if done { index + 1 } else { index };
                return Err(roll_back(
                    scope,
                    journal,
                    &entry,
                    &planned.ops,
                    done_ops,
                    index,
                    cause,
                    &fault,
                ));
            }
        }
    }
    if fault.crash_after == Some(planned.ops.len()) {
        return Err(crash_error(planned.ops.len(), last));
    }
    if let Err(error) = journal.remove(&entry) {
        log::error!("removing journal entry {entry} after a finished batch failed: {error}");
    }
    Ok(BatchReport {
        done: planned.ops.len(),
        methods,
    })
}

/// Rename with the retry policy of its kind; a folder still held afterwards is `FolderBusy`.
pub(crate) fn rename_with_retry(
    scope: &Scope,
    from: &Path,
    to: &Path,
    is_dir: bool,
    retry: RetryMode,
    journal: Option<(&Journal, &str)>,
) -> Result<RenameMethod, FsError> {
    let policy_for: fn(OsErrorClass) -> RetryPolicy = match (retry, is_dir) {
        (RetryMode::Never, _) => |_| RetryPolicy::ONCE,
        (RetryMode::Normal, true) => platform::folder_retry_policy,
        (RetryMode::Normal, false) => platform::retry_policy,
    };
    let result = with_retry(policy_for, || rename_one(scope, from, to, is_dir, journal));
    result.map_err(|error| folder_busy(error, from, is_dir))
}

/// The op's result; on Err the flag says whether the op itself was done (verify failures).
fn run_op(
    scope: &Scope,
    journal: &Journal,
    entry: &str,
    op: &RenameOp,
    index: usize,
    fault: &FaultPlan,
    retry: RetryMode,
) -> Result<RenameMethod, (FsError, bool)> {
    if fault.fail_before == Some(index) {
        let injected = fault.raw_os_error.map_or_else(
            || io::Error::other("injected fault"),
            io::Error::from_raw_os_error,
        );
        let error = FsError::io("rename", &op.from, injected);
        return Err((folder_busy(error, &op.from, op.is_dir), false));
    }
    rename_with_retry(
        scope,
        &op.from,
        &op.to,
        op.is_dir,
        retry,
        Some((journal, entry)),
    )
    .map_err(|error| (error, false))
}

#[allow(clippy::too_many_arguments)]
fn roll_back(
    scope: &Scope,
    journal: &Journal,
    entry: &str,
    ops: &[RenameOp],
    done: usize,
    failed: usize,
    cause: FsError,
    fault: &FaultPlan,
) -> FsError {
    let mut stuck = Vec::new();
    if let (
        FsError::Io {
            op: STUCK_UNLINK_OP,
            ..
        },
        Some(op),
    ) = (&cause, ops.get(failed))
    {
        stuck.push(StuckOp {
            from: op.from.clone(),
            to: op.to.clone(),
        });
    }
    let mut rolled_back = 0;
    for (index, op) in ops.iter().enumerate().take(done).rev() {
        let undo = if fault.fail_rollback_of == Some(index) {
            Err(FsError::io(
                "undo",
                &op.to,
                io::Error::other("injected fault"),
            ))
        } else {
            rename_with_retry(
                scope,
                &op.to,
                &op.from,
                op.is_dir,
                RetryMode::Normal,
                Some((journal, entry)),
            )
        };
        match undo {
            Ok(_) => rolled_back += 1,
            Err(error) => {
                log::error!(
                    "undo of {} -> {} failed: {error}",
                    op.from.display(),
                    op.to.display()
                );
                stuck.push(StuckOp {
                    from: op.from.clone(),
                    to: op.to.clone(),
                });
            }
        }
    }
    let closed = if stuck.is_empty() {
        journal.remove(entry)
    } else {
        journal.mark_stuck(entry)
    };
    if let Err(error) = closed {
        log::error!("closing journal entry {entry} after a failed batch failed: {error}");
    }
    FsError::Batch(Box::new(BatchError {
        failed,
        cause,
        rolled_back,
        stuck,
    }))
}

fn crash_error(index: usize, op: &RenameOp) -> FsError {
    FsError::Batch(Box::new(BatchError {
        failed: index,
        cause: FsError::io("crash", &op.from, io::Error::other("injected crash")),
        rolled_back: 0,
        stuck: Vec::new(),
    }))
}

fn with_retry<T>(
    policy_for: fn(OsErrorClass) -> RetryPolicy,
    mut attempt: impl FnMut() -> Result<T, FsError>,
) -> Result<T, FsError> {
    let started = Instant::now();
    let mut tries = 0_u32;
    loop {
        tries += 1;
        let error = match attempt() {
            Ok(value) => return Ok(value),
            Err(error) => error,
        };
        let Some(class) = error.class() else {
            return Err(error);
        };
        let policy = policy_for(class);
        if tries >= policy.attempts || started.elapsed() >= policy.total {
            return Err(error);
        }
        log::debug!("rename failed (try {tries}), retrying: {error}");
        thread::sleep(policy.pause());
    }
}

fn folder_busy(error: FsError, path: &Path, is_dir: bool) -> FsError {
    let held = matches!(
        error.class(),
        Some(OsErrorClass::AccessDenied | OsErrorClass::InUse)
    );
    if is_dir && held && matches!(error, FsError::Io { .. }) {
        return FsError::FolderBusy {
            path: path.to_path_buf(),
            holders: platform::who_holds(&[path.to_path_buf()]),
        };
    }
    error
}

/// After a case-only pair the folder must list the exact new name (vfat, exfat and casefold
/// folders may keep the old spelling).
fn verify_exact_name(to: &Path) -> Result<(), FsError> {
    let (Some(parent), Some(name)) = (to.parent(), to.file_name()) else {
        return Err(unsupported_case(to));
    };
    let listed = list_folded(parent)?
        .iter()
        .any(|(_, entry)| entry.as_os_str() == name);
    if listed {
        Ok(())
    } else {
        Err(unsupported_case(to))
    }
}

fn unsupported_case(path: &Path) -> FsError {
    FsError::io(
        "verify_case",
        path,
        io::Error::from(io::ErrorKind::Unsupported),
    )
}
