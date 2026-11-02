//! The trash movers: freedesktop.org (renames mod items into a trash on the same mount and writes
//! their `.trashinfo`) and the Windows Recycle Bin (through `platform::recycle`). Neither copies
//! nor deletes item data.

use std::ffi::{OsStr, OsString};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};

use super::FsError;
use super::batch::{BatchError, FaultPlan, StuckOp};
use super::gate::WriteGuard;
use super::names;
use super::scope::Scope;
use crate::platform::{
    self, NoReplace, OsErrorClass, Recycled, TrashKind, TrashLocation, TrashUnavailableReason,
};

/// `name`, then `name.2` … `name.999`.
const MAX_NAME_SUFFIX: u32 = 999;
const INFO_EXT: &str = ".trashinfo";

/// trashed = `<trash>/files/<name>`, info = `<trash>/info/<name>.trashinfo`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrashedItem {
    pub original: PathBuf,
    pub trashed: PathBuf,
    pub info: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrashReport {
    pub kind: TrashKind,
    pub items: Vec<TrashedItem>,
}

impl WriteGuard<'_> {
    /// Moves `items` (one folder, one file, or one loose group in the given order) into one trash.
    /// All items must share one parent folder. Never copies, never deletes item data.
    pub fn trash(
        &self,
        env: &platform::TrashEnv,
        items: &[PathBuf],
        now: DateTime<Local>,
    ) -> Result<TrashReport, FsError> {
        self.trash_with_fault(env, items, now, None)
    }

    /// [`WriteGuard::trash`] with test fault injection: `fail_before: Some(k)` fails item k's
    /// rename with the injected error, like `rename_batch`; `abort_recycle: Some(k)` (Recycle Bin
    /// only) makes the shell sink refuse item k as if the shell could not recycle it.
    pub fn trash_with_fault(
        &self,
        env: &platform::TrashEnv,
        items: &[PathBuf],
        now: DateTime<Local>,
        fault: Option<&FaultPlan>,
    ) -> Result<TrashReport, FsError> {
        let first = check_items(self.scope(), items)?;
        if let Some(named) = self.first_journaled(items)? {
            return Err(FsError::AsideJournaled { path: named });
        }
        let location = match platform::trash_location(env, first) {
            Ok(Ok(location)) => location,
            Ok(Err(reason)) => return Err(FsError::TrashUnavailable { reason }),
            Err(error) => return Err(FsError::io("trash_location", first, error)),
        };
        let fault = fault.cloned().unwrap_or_default();
        if location.kind == TrashKind::RecycleBin {
            return recycle_items(items, &fault);
        }
        create_missing(&location, env.uid)?;
        let mut done: Vec<TrashedItem> = Vec::with_capacity(items.len());
        for (index, item) in items.iter().enumerate() {
            match move_one(&location, item, now, injected(&fault, index)) {
                Ok(trashed) => done.push(trashed),
                Err(cause) if done.is_empty() => return Err(cause),
                Err(cause) => return Err(roll_back(&done, index, cause)),
            }
        }
        Ok(TrashReport {
            kind: location.kind,
            items: done,
        })
    }
}

impl WriteGuard<'_> {
    /// Tests only: `scope.check(item)`, then `platform::recycle(item, force_abort)` without the
    /// name check of the trash path, so tests can prove the platform's own guard. Never called by
    /// core.
    #[doc(hidden)]
    pub fn recycle_forced(&self, item: &Path, force_abort: bool) -> Result<Recycled, FsError> {
        self.scope().check(item)?;
        platform::recycle(item, force_abort).map_err(|error| FsError::io("recycle", item, error))
    }
}

impl WriteGuard<'_> {
    /// An item that a journal record names (e.g. a live install staging folder).
    fn first_journaled(&self, items: &[PathBuf]) -> Result<Option<PathBuf>, FsError> {
        for item in items {
            if self.is_journaled(item)? {
                return Ok(Some(item.clone()));
            }
        }
        Ok(None)
    }
}

/// `[Trash Info]` with `Path=` (already percent-encoded) and the local `DeletionDate`.
pub fn trashinfo_text(path_field: &str, now: DateTime<Local>) -> String {
    format!(
        "[Trash Info]\nPath={path_field}\nDeletionDate={}\n",
        now.format("%Y-%m-%dT%H:%M:%S")
    )
}

/// Every byte except the unreserved and path-safe ASCII set becomes `%XX` (uppercase hex).
pub fn percent_encode_path(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &byte in bytes {
        if byte.is_ascii_alphanumeric() || b"-_.~/!$&'()*+,;=:@".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Non-empty, one shared parent, each in scope and present. Returns the first item.
fn check_items<'a>(scope: &Scope, items: &'a [PathBuf]) -> Result<&'a Path, FsError> {
    let Some(first) = items.first() else {
        return Err(FsError::InvalidBatch {
            reason: "nothing to trash",
        });
    };
    for item in items {
        if item.parent() != first.parent() {
            return Err(FsError::InvalidBatch {
                reason: "trash items need one parent folder",
            });
        }
        scope.check(item)?;
        let present = platform::fs_path(item).and_then(fs::symlink_metadata);
        if present.is_err() {
            return Err(FsError::SourceMissing { path: item.clone() });
        }
    }
    Ok(first)
}

/// Every item is checked before anything moves; then one item at a time, in order. Nothing is
/// moved back out of the Recycle Bin: after a failure at item k ≥ 1, items 0..k stay there
/// (restorable) and are reported as stuck. For a loose group the `.pak` went first, so what is
/// left behind is inert.
fn recycle_items(items: &[PathBuf], fault: &FaultPlan) -> Result<TrashReport, FsError> {
    if !items.iter().all(|item| names::shell_safe_path(item)) {
        return Err(FsError::TrashUnavailable {
            reason: TrashUnavailableReason::NotShellSafe,
        });
    }
    let mut done: Vec<TrashedItem> = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let force_abort = fault.abort_recycle == Some(index);
        match recycle_one(item, injected(fault, index), force_abort) {
            Ok(recycled) => done.push(recycled),
            Err(cause) if done.is_empty() => return Err(cause),
            Err(cause) => return Err(left_in_bin(&done, index, cause)),
        }
    }
    Ok(TrashReport {
        kind: TrashKind::RecycleBin,
        items: done,
    })
}

/// `info` stays empty: the bin's `$I…` file is the shell's business.
fn recycle_one(
    item: &Path,
    fault: Option<io::Error>,
    force_abort: bool,
) -> Result<TrashedItem, FsError> {
    if let Some(error) = fault {
        return Err(FsError::io("recycle", item, error));
    }
    let reason = match platform::recycle(item, force_abort) {
        Ok(Recycled::Done { bin_path }) => {
            return Ok(TrashedItem {
                original: item.to_path_buf(),
                trashed: bin_path,
                info: PathBuf::new(),
            });
        }
        Ok(Recycled::Refused) => TrashUnavailableReason::RecycleRefused,
        Ok(Recycled::NotShellSafe) => TrashUnavailableReason::NotShellSafe,
        Err(error) => return Err(FsError::io("recycle", item, error)),
    };
    Err(FsError::TrashUnavailable { reason })
}

fn left_in_bin(done: &[TrashedItem], failed: usize, cause: FsError) -> FsError {
    for item in done {
        log::error!(
            "{} stays in the Recycle Bin as {} after item {failed} failed: {cause}",
            item.original.display(),
            item.trashed.display()
        );
    }
    FsError::Batch(Box::new(BatchError {
        failed,
        cause,
        rolled_back: 0,
        stuck: done
            .iter()
            .map(|item| StuckOp {
                from: item.trashed.clone(),
                to: item.original.clone(),
            })
            .collect(),
    }))
}

fn create_missing(location: &TrashLocation, uid: Option<u32>) -> Result<(), FsError> {
    for folder in &location.missing {
        match platform::create_private_dir(folder) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                let usable =
                    platform::dir_traits(folder).is_ok_and(|traits| traits.is_private_dir_of(uid));
                if !usable {
                    return Err(no_usable_trash(folder, &error));
                }
            }
            Err(error) => return Err(no_usable_trash(folder, &error)),
        }
    }
    Ok(())
}

fn no_usable_trash(folder: &Path, error: &io::Error) -> FsError {
    log::warn!("trash folder {} is not usable: {error}", folder.display());
    FsError::TrashUnavailable {
        reason: TrashUnavailableReason::NoUsableTrash,
    }
}

fn injected(fault: &FaultPlan, index: usize) -> Option<io::Error> {
    (fault.fail_before == Some(index)).then(|| match fault.raw_os_error {
        Some(code) => io::Error::from_raw_os_error(code),
        None => io::Error::other("injected fault"),
    })
}

fn move_one(
    location: &TrashLocation,
    item: &Path,
    now: DateTime<Local>,
    fault: Option<io::Error>,
) -> Result<TrashedItem, FsError> {
    let name = item.file_name().ok_or(FsError::InvalidBatch {
        reason: "a trash item has no file name",
    })?;
    let path_field = path_field(location, item)?;
    let text = trashinfo_text(
        &percent_encode_path(path_field.as_os_str().as_encoded_bytes()),
        now,
    );
    let files_dir = location.root.join("files");
    let info_dir = location.root.join("info");
    let mut fault = fault;
    for attempt in 1..=MAX_NAME_SUFFIX {
        let candidate = candidate_name(name, attempt);
        let trashed = files_dir.join(&candidate);
        let info = info_dir.join(info_name(&candidate));
        if !reserve_info(&info, &trashed, &text, &info_dir)? {
            continue;
        }
        let outcome = match fault.take() {
            Some(error) => Err(error),
            None => platform::rename_noreplace_across(item, &trashed),
        };
        match outcome {
            Ok(NoReplace::Done) => {
                sync_dir_logged(item.parent());
                sync_dir_logged(Some(&files_dir));
                return Ok(TrashedItem {
                    original: item.to_path_buf(),
                    trashed,
                    info,
                });
            }
            Ok(NoReplace::FlagUnsupported) => {
                remove_info_logged(&info);
                return Err(FsError::TrashUnavailable {
                    reason: TrashUnavailableReason::NoUsableTrash,
                });
            }
            // A file manager took the name between the check and the rename: next name.
            Err(error) if platform::classify(&error) == OsErrorClass::AlreadyExists => {
                remove_info_logged(&info);
            }
            Err(error) => {
                remove_info_logged(&info);
                if platform::classify(&error) == OsErrorClass::CrossDevice {
                    return Err(FsError::TrashUnavailable {
                        reason: TrashUnavailableReason::OtherDevice,
                    });
                }
                return Err(FsError::io("trash", item, error));
            }
        }
    }
    Err(FsError::TargetExists {
        path: files_dir.join(name),
    })
}

/// Reserves `info` with O_EXCL and writes `text` (fsync file and folder). False when the name is
/// taken in `info/` or `files/`; nothing is left behind then.
fn reserve_info(info: &Path, trashed: &Path, text: &str, info_dir: &Path) -> Result<bool, FsError> {
    let mut file = match platform::create_new_file(info) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => return Ok(false),
        Err(error) => return Err(FsError::io("trashinfo", info, error)),
    };
    if platform::fs_path(trashed)
        .and_then(fs::symlink_metadata)
        .is_ok()
    {
        drop(file);
        remove_info_logged(info);
        return Ok(false);
    }
    let written = file
        .write_all(text.as_bytes())
        .and_then(|()| file.sync_all());
    drop(file);
    if let Err(error) = written {
        remove_info_logged(info);
        return Err(FsError::io("trashinfo", info, error));
    }
    sync_dir_logged(Some(info_dir));
    Ok(true)
}

/// `Path=`: absolute for the home trash, relative to the topdir otherwise; the item's folder is
/// resolved the same way the probe resolved it.
fn path_field(location: &TrashLocation, item: &Path) -> Result<PathBuf, FsError> {
    let parent = item.parent().unwrap_or(item);
    let resolved = platform::fs_path(parent)
        .and_then(fs::canonicalize)
        .map_err(|error| FsError::io("trash", item, error))?;
    let full = match item.file_name() {
        Some(name) => resolved.join(name),
        None => resolved,
    };
    match &location.topdir {
        None => Ok(full),
        Some(topdir) => full
            .strip_prefix(topdir)
            .map(Path::to_path_buf)
            .map_err(|_| FsError::TrashUnavailable {
                reason: TrashUnavailableReason::NoUsableTrash,
            }),
    }
}

fn candidate_name(name: &OsStr, attempt: u32) -> OsString {
    let mut candidate = name.to_os_string();
    if attempt > 1 {
        candidate.push(format!(".{attempt}"));
    }
    candidate
}

fn info_name(candidate: &OsStr) -> OsString {
    let mut info = candidate.to_os_string();
    info.push(INFO_EXT);
    info
}

/// Moves items 0..failed back in reverse order; what cannot move back stays in the trash with its
/// info file and is reported as stuck.
fn roll_back(done: &[TrashedItem], failed: usize, cause: FsError) -> FsError {
    let mut rolled_back = 0;
    let mut stuck = Vec::new();
    for item in done.iter().rev() {
        match platform::rename_noreplace_across(&item.trashed, &item.original) {
            Ok(NoReplace::Done) => {
                remove_info_logged(&item.info);
                rolled_back += 1;
            }
            outcome => {
                log::error!(
                    "moving {} back from the trash failed: {outcome:?}",
                    item.trashed.display()
                );
                stuck.push(StuckOp {
                    from: item.trashed.clone(),
                    to: item.original.clone(),
                });
            }
        }
    }
    FsError::Batch(Box::new(BatchError {
        failed,
        cause,
        rolled_back,
        stuck,
    }))
}

fn remove_info_logged(info: &Path) {
    if let Err(error) = platform::fs_path(info).and_then(fs::remove_file) {
        log::warn!("removing {} failed: {error}", info.display());
    }
}

fn sync_dir_logged(dir: Option<&Path>) {
    let Some(dir) = dir else {
        return;
    };
    let synced = platform::fs_path(dir)
        .and_then(File::open)
        .and_then(|folder| folder.sync_all());
    if let Err(error) = synced {
        log::debug!("fsync of {} failed: {error}", dir.display());
    }
}
