use std::io::ErrorKind;
use std::path::Path;
use std::time::Duration;

use super::{
    GRAPHICS_VERSION, GraphicsError, GraphicsFile, GraphicsLoad, GraphicsMode, GraphicsPaths,
    GraphicsSession, GraphicsStep, ModeSource, PendingMarker, StartInput, env_for,
    is_user_controlled, step_after_crash,
};
use crate::safe_fs::{self, AppLock, FsError};

/// Scans hold the shared app lock, so a write waits for a running one.
const LOCK_WAIT: Duration = Duration::from_secs(2);

enum DiskFile {
    Missing,
    Valid(GraphicsFile),
    Newer(u64),
    Corrupt,
}

/// What the locked part of `start` decided.
struct Prepared {
    file: GraphicsFile,
    read_only: bool,
    stepped_now: bool,
    marker_written: bool,
}

/// Decides the session's graphics mode and writes the crash marker. Never fails: problems end up
/// in `notes` and leave the session without a marker (so the next start never steps because of
/// them). `owner_alive(pid)` tells whether another RemMods process with that pid is running: a
/// marker it owns belongs to a start that has not created its session yet, not to a crash.
pub fn start(
    paths: &GraphicsPaths,
    input: &StartInput,
    owner_alive: &dyn Fn(u32) -> bool,
) -> GraphicsSession {
    let mut notes = Vec::new();
    let user_controlled = is_user_controlled(&input.present);
    let prepared = if input.other_instance {
        notes.push("another RemMods session is running; the graphics marker is skipped".to_owned());
        unlocked(paths, &mut notes)
    } else {
        match AppLock::exclusive_within(&paths.lock_file, LOCK_WAIT) {
            Ok(lock) => {
                let prepared = start_locked(paths, input, user_controlled, owner_alive, &mut notes);
                drop(lock);
                prepared
            }
            Err(error) => {
                notes.push(format!(
                    "graphics: the app lock is not free ({error}); no marker this session"
                ));
                unlocked(paths, &mut notes)
            }
        }
    };
    let (effective, source) = effective_mode(prepared.file.mode, input.safe_flag);
    let mut env = env_for(effective, input.facts);
    env.retain(|(name, _)| !input.present.iter().any(|present| present == name));
    if user_controlled {
        env.retain(|(name, _)| !name.starts_with("WEBKIT_"));
        notes.push(format!(
            "the environment sets WebKit renderer variables; they override graphics mode \
             {effective:?}"
        ));
    }
    GraphicsSession {
        stored: prepared.file.mode,
        effective,
        source,
        env,
        stepped: prepared.file.stepped,
        stepped_now: prepared.stepped_now,
        user_controlled,
        marker_written: prepared.marker_written,
        read_only: prepared.read_only,
        notes,
    }
}

/// `graphics.json` as it is now. A corrupt file is backed up and replaced by defaults under the
/// app lock, so the next load does not back up the same bytes again.
pub fn load(paths: &GraphicsPaths) -> GraphicsLoad {
    let mut notes = Vec::new();
    let loaded = match read_disk(&paths.file) {
        Ok(DiskFile::Corrupt) => match AppLock::exclusive_within(&paths.lock_file, LOCK_WAIT) {
            Ok(_lock) => load_locked(paths, &mut notes),
            Err(error) => {
                notes.push(format!("graphics: the app lock is not free ({error})"));
                peek(paths, &mut notes)
            }
        },
        disk => from_disk(paths, disk, &mut notes),
    };
    for note in notes {
        log::warn!("{note}");
    }
    loaded
}

/// Stores `mode` for the next start and drops an unacknowledged step.
pub fn set_mode(paths: &GraphicsPaths, mode: GraphicsMode) -> Result<GraphicsFile, GraphicsError> {
    change_file(paths, |file| {
        file.mode = mode;
        file.stepped = None;
        Ok(())
    })
}

/// Keeps (`revert` false) or undoes (`revert` true) the last automatic step.
pub fn ack_step(paths: &GraphicsPaths, revert: bool) -> Result<GraphicsFile, GraphicsError> {
    change_file(paths, |file| {
        let step = file.stepped.take().ok_or(GraphicsError::NoStep)?;
        if revert {
            file.mode = step.from;
        }
        Ok(())
    })
}

/// No app lock here: after an AppImage update the sealed gate of this process holds it, and the
/// exit path must still clear the marker. Single-instance keeps other writers away.
pub fn clear_marker(paths: &GraphicsPaths) -> Result<(), GraphicsError> {
    safe_fs::remove_app_file(&paths.marker, &[&paths.config_dir])?;
    Ok(())
}

fn change_file(
    paths: &GraphicsPaths,
    change: impl FnOnce(&mut GraphicsFile) -> Result<(), GraphicsError>,
) -> Result<GraphicsFile, GraphicsError> {
    let _lock = AppLock::exclusive_within(&paths.lock_file, LOCK_WAIT)?;
    let mut notes = Vec::new();
    let loaded = load_locked(paths, &mut notes);
    for note in notes {
        log::warn!("{note}");
    }
    if loaded.read_only {
        return Err(GraphicsError::ReadOnly);
    }
    let mut file = loaded.file;
    change(&mut file)?;
    file.version = GRAPHICS_VERSION;
    write_file(&paths.file, &file)?;
    Ok(file)
}

fn start_locked(
    paths: &GraphicsPaths,
    input: &StartInput,
    user_controlled: bool,
    owner_alive: &dyn Fn(u32) -> bool,
    notes: &mut Vec<String>,
) -> Prepared {
    let loaded = load_locked(paths, notes);
    let mut prepared = Prepared {
        file: loaded.file,
        read_only: loaded.read_only,
        stepped_now: false,
        marker_written: false,
    };
    if prepared.read_only {
        notes.push("graphics.json is read-only; no crash marker this session".to_owned());
        return prepared;
    }
    if user_controlled {
        remove_surviving_marker(paths, notes);
        return prepared;
    }
    let (marker, corrupt) = read_marker(&paths.marker, notes);
    if let Some(owner) = marker.as_ref().map(|marker| marker.pid)
        && owner != input.pid
        && owner_alive(owner)
    {
        // A second launch before the first one created its session lock (so `other_instance`
        // was false) must neither step nor replace the first start's marker.
        notes.push(format!(
            "another RemMods start owns the graphics marker (pid {owner}); the marker is skipped"
        ));
        return prepared;
    }
    prepared.stepped_now = step_if_crashed(
        paths,
        &mut prepared.file,
        marker.as_ref(),
        corrupt,
        &input.now,
        notes,
    );
    let (effective, source) = effective_mode(prepared.file.mode, input.safe_flag);
    let marker = PendingMarker {
        version: GRAPHICS_VERSION,
        mode: effective,
        source,
        pid: input.pid,
    };
    prepared.marker_written = match write_marker(&paths.marker, &marker) {
        Ok(()) => true,
        Err(error) => {
            notes.push(format!("writing the graphics marker failed: {error}"));
            false
        }
    };
    prepared
}

fn effective_mode(stored: GraphicsMode, safe_flag: bool) -> (GraphicsMode, ModeSource) {
    if safe_flag {
        (GraphicsMode::Software, ModeSource::Flag)
    } else {
        (stored, ModeSource::Stored)
    }
}

/// Steps `file` down when the previous start crashed in its mode; true when the step is stored.
fn step_if_crashed(
    paths: &GraphicsPaths,
    file: &mut GraphicsFile,
    marker: Option<&PendingMarker>,
    corrupt: bool,
    now: &str,
    notes: &mut Vec<String>,
) -> bool {
    let Some(to) = step_after_crash(file.mode, marker, corrupt) else {
        return false;
    };
    let stepped = GraphicsFile {
        version: GRAPHICS_VERSION,
        mode: to,
        stepped: Some(GraphicsStep {
            from: file.mode,
            to,
            at: now.to_owned(),
        }),
    };
    match write_file(&paths.file, &stepped) {
        Ok(()) => {
            notes.push(format!(
                "the previous start in graphics mode {:?} did not finish; stepped to {to:?}",
                file.mode
            ));
            *file = stepped;
            true
        }
        Err(error) => {
            notes.push(format!("storing the graphics step failed: {error}"));
            false
        }
    }
}

fn remove_surviving_marker(paths: &GraphicsPaths, notes: &mut Vec<String>) {
    match safe_fs::remove_app_file(&paths.marker, &[&paths.config_dir]) {
        Ok(true) => notes
            .push("removed the graphics marker: the environment controls the renderer".to_owned()),
        Ok(false) => {}
        Err(error) => notes.push(format!("removing the graphics marker failed: {error}")),
    }
}

/// The marker and whether it exists but does not parse. A marker that cannot be read at all
/// (a folder, no permission) counts as absent: only a start that wrote it may step.
fn read_marker(marker: &Path, notes: &mut Vec<String>) -> (Option<PendingMarker>, bool) {
    match std::fs::read(marker) {
        Ok(bytes) => match serde_json::from_slice::<PendingMarker>(&bytes) {
            Ok(parsed) => (Some(parsed), false),
            Err(_) => (None, true),
        },
        Err(error) if error.kind() == ErrorKind::NotFound => (None, false),
        Err(error) => {
            notes.push(format!(
                "cannot read the graphics marker {}: {error}",
                marker.display()
            ));
            (None, false)
        }
    }
}

/// Caller holds the app lock.
fn load_locked(paths: &GraphicsPaths, notes: &mut Vec<String>) -> GraphicsLoad {
    let disk = read_disk(&paths.file);
    if !matches!(disk, Ok(DiskFile::Corrupt)) {
        return from_disk(paths, disk, notes);
    }
    match safe_fs::backup_file(&paths.file) {
        Ok(backup) => {
            notes.push(format!(
                "{} is unreadable; backed up to {}, using defaults",
                paths.file.display(),
                backup.display()
            ));
            if let Err(error) = write_file(&paths.file, &GraphicsFile::default()) {
                notes.push(format!(
                    "writing the default graphics settings failed: {error}"
                ));
            }
            GraphicsLoad {
                file: GraphicsFile::default(),
                read_only: false,
                recovered: true,
            }
        }
        Err(error) => {
            notes.push(format!(
                "corrupt graphics settings could not be backed up: {error}"
            ));
            read_only_defaults()
        }
    }
}

/// Read-only view: a corrupt file is neither backed up nor replaced.
fn unlocked(paths: &GraphicsPaths, notes: &mut Vec<String>) -> Prepared {
    let loaded = peek(paths, notes);
    Prepared {
        file: loaded.file,
        read_only: loaded.read_only,
        stepped_now: false,
        marker_written: false,
    }
}

fn peek(paths: &GraphicsPaths, notes: &mut Vec<String>) -> GraphicsLoad {
    match read_disk(&paths.file) {
        Ok(DiskFile::Corrupt) => {
            notes.push(format!(
                "{} is unreadable; using defaults read-only",
                paths.file.display()
            ));
            read_only_defaults()
        }
        disk => from_disk(paths, disk, notes),
    }
}

/// Every case but `Corrupt`, which needs the caller's policy.
fn from_disk(
    paths: &GraphicsPaths,
    disk: Result<DiskFile, FsError>,
    notes: &mut Vec<String>,
) -> GraphicsLoad {
    match disk {
        Ok(DiskFile::Missing) => clean(GraphicsFile::default()),
        Ok(DiskFile::Valid(file)) => clean(file),
        Ok(DiskFile::Newer(version)) => {
            notes.push(format!(
                "{} has format version {version}; using defaults read-only",
                paths.file.display()
            ));
            read_only_defaults()
        }
        Ok(DiskFile::Corrupt) => read_only_defaults(),
        Err(error) => {
            notes.push(format!("cannot read graphics settings: {error}"));
            read_only_defaults()
        }
    }
}

fn clean(file: GraphicsFile) -> GraphicsLoad {
    GraphicsLoad {
        file,
        read_only: false,
        recovered: false,
    }
}

fn read_only_defaults() -> GraphicsLoad {
    GraphicsLoad {
        file: GraphicsFile::default(),
        read_only: true,
        recovered: false,
    }
}

fn read_disk(file: &Path) -> Result<DiskFile, FsError> {
    match std::fs::read(file) {
        Ok(bytes) => Ok(parse_file(&bytes)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(DiskFile::Missing),
        Err(error) => Err(FsError::io("read", file, error)),
    }
}

fn parse_file(bytes: &[u8]) -> DiskFile {
    // serde also reads a struct from an array, so `[]` would pass as defaults.
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return DiskFile::Corrupt;
    };
    if !value.is_object() {
        return DiskFile::Corrupt;
    }
    if let Some(version) = value.get("version").and_then(serde_json::Value::as_u64)
        && version > u64::from(GRAPHICS_VERSION)
    {
        return DiskFile::Newer(version);
    }
    match serde_json::from_value::<GraphicsFile>(value) {
        Ok(file) => DiskFile::Valid(file),
        Err(_) => DiskFile::Corrupt,
    }
}

/// Pretty-printed with a trailing newline, like `settings.json`.
fn write_file(path: &Path, file: &GraphicsFile) -> Result<(), GraphicsError> {
    let mut bytes = serde_json::to_vec_pretty(file)?;
    bytes.push(b'\n');
    safe_fs::write_atomic(path, &bytes)?;
    Ok(())
}

fn write_marker(path: &Path, marker: &PendingMarker) -> Result<(), GraphicsError> {
    let mut bytes = serde_json::to_vec(marker)?;
    bytes.push(b'\n');
    safe_fs::write_atomic(path, &bytes)?;
    Ok(())
}
