//! The only module with `cfg` and `unsafe`. Every function here has the same signature on every
//! OS; the per-OS files only fill in the details. Raw OS error codes collide across systems
//! (Windows 5/32/33 are EIO/EPIPE/EDOM on Linux), so errors are classified here and nowhere else.

use std::ffi::OsStr;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

mod entry;
mod procfs;

pub use entry::{FileId, RawName, file_id, fs_path, raw_name};
pub use procfs::{ProcessRecord, parse_cmdline, parse_comm, read_proc_tree};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as imp;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as imp;

#[cfg(not(any(target_os = "linux", windows)))]
mod fallback;
#[cfg(not(any(target_os = "linux", windows)))]
use fallback as imp;

/// How long `spawn_detached` watches a new child for an early failure.
const DETACHED_EARLY_EXIT_WINDOW: Duration = Duration::from_millis(300);
const DETACHED_POLL_INTERVAL: Duration = Duration::from_millis(20);

/// OS-independent meaning of an I/O error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OsErrorClass {
    InUse,
    AccessDenied,
    NoSpace,
    ReadOnlyFs,
    CrossDevice,
    NameTooLong,
    NotFound,
    Unsupported,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OsKind {
    Linux,
    Windows,
    Other,
}

/// How often an operation that failed with a given [`OsErrorClass`] may be tried in total.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetryPolicy {
    pub attempts: u32,
    pub total: Duration,
}

impl RetryPolicy {
    pub const ONCE: Self = Self {
        attempts: 1,
        total: Duration::ZERO,
    };

    /// Pause between two attempts, spreading the attempts evenly over `total`.
    pub fn pause(&self) -> Duration {
        match self.attempts {
            0 | 1 => Duration::ZERO,
            attempts => self.total / (attempts - 1),
        }
    }
}

/// Where Steam may live, gathered once per detection run. Tests inject their own values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SteamHints {
    pub os: OsKind,
    pub home: Option<PathBuf>,
    pub data_dir: Option<PathBuf>,
    /// Windows only; default elsewhere.
    pub registry: RegistrySteamValues,
    /// Env `ProgramFiles(x86)`, Windows only.
    pub program_files_x86: Option<PathBuf>,
    /// Env `ProgramFiles`, Windows only.
    pub program_files: Option<PathBuf>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RegistrySteamValues {
    /// HKCU `Software\Valve\Steam` `SteamPath` (default view).
    pub steam_path: Option<String>,
    /// HKLM `SOFTWARE\Valve\Steam` `InstallPath`, 32-bit view.
    pub install_path_32: Option<String>,
    /// The same value in the 64-bit view.
    pub install_path_64: Option<String>,
    /// Values whose read failed with anything but "not found".
    pub errors: Vec<RegistryError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistryError {
    /// For example `HKLM\SOFTWARE\Valve\Steam\InstallPath (32-bit)`.
    pub value: String,
    /// `registry_string` returned `ErrorKind::InvalidData`.
    pub not_unicode: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistryHive {
    CurrentUser,
    LocalMachine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistryView {
    Default,
    Wow32,
    Wow64,
}

pub fn classify(error: &io::Error) -> OsErrorClass {
    imp::classify_raw(error).unwrap_or_else(|| classify_kind(error.kind()))
}

pub fn retry_policy(class: OsErrorClass) -> RetryPolicy {
    imp::retry_policy(class)
}

/// Policy for replacing an existing file (the rename that ends `write_atomic`). On Windows a
/// target held open without `FILE_SHARE_DELETE` (scanner, indexer, sync tool, editor) makes the
/// replace fail with ACCESS_DENIED, so there that class means "in use" as well.
pub fn replace_retry_policy(class: OsErrorClass) -> RetryPolicy {
    imp::replace_retry_policy(class)
}

pub fn os_kind() -> OsKind {
    imp::OS_KIND
}

/// Origin the bundled frontend is served from (the custom protocol differs per OS).
pub fn app_origin() -> &'static str {
    imp::APP_ORIGIN
}

/// Last-resort error report for failures before the window exists.
pub fn fatal_message(title: &str, text: &str) {
    imp::fatal_message(title, text);
}

/// Every process of the session. Linux: `read_proc_tree("/proc")`. Windows: Toolhelp32 (names
/// only, no command lines). Other OSes: `ErrorKind::Unsupported`.
pub fn processes() -> io::Result<Vec<ProcessRecord>> {
    imp::processes()
}

/// Never fails: read errors are logged, recorded in `registry.errors` and leave the value `None`.
pub fn steam_hints() -> SteamHints {
    imp::steam_hints()
}

/// Windows: `RegGetValueW(RRF_RT_REG_SZ | view flag)`. `Ok(None)` when the key or value does not
/// exist or is not a string. Invalid UTF-16 is `ErrorKind::InvalidData`, never a lossy string
/// (a replaced unit would point detection at another path). Other OSes: `ErrorKind::Unsupported`.
/// Never writes the registry.
pub fn registry_string(
    hive: RegistryHive,
    subkey: &str,
    value: &str,
    view: RegistryView,
) -> io::Result<Option<String>> {
    imp::registry_string(hive, subkey, value, view)
}

/// A command for a host program: working directory = home when known, all three streams null.
pub fn host_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    if let Some(home) = dirs::home_dir() {
        command.current_dir(home);
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

/// Starts `command` without waiting for it. Linux: the child runs in its own session (setsid), so
/// it outlives RemMods. A child that exits with a non-zero status within 300 ms is an error;
/// still running or exit 0 is Ok. A thread named `detached-child` then reaps the child.
pub fn spawn_detached(mut command: Command) -> io::Result<()> {
    imp::prepare_detached(&mut command);
    let mut child = command.spawn()?;
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            if status.success() {
                log::debug!("detached child {} exited at once with 0", child.id());
                return Ok(());
            }
            let code = status
                .code()
                .map_or_else(|| status.to_string(), |code| code.to_string());
            return Err(io::Error::other(format!("exited with {code}")));
        }
        if started.elapsed() >= DETACHED_EARLY_EXIT_WINDOW {
            break;
        }
        thread::sleep(DETACHED_POLL_INTERVAL);
    }
    let pid = child.id();
    let waiter = thread::Builder::new()
        .name("detached-child".to_owned())
        .spawn(move || match child.wait() {
            Ok(status) => log::debug!("detached child {pid} ended: {status}"),
            Err(error) => log::warn!("waiting for detached child {pid} failed: {error}"),
        });
    if let Err(error) = waiter {
        log::warn!("cannot start the waiter for detached child {pid}: {error}");
    }
    Ok(())
}

fn classify_kind(kind: io::ErrorKind) -> OsErrorClass {
    match kind {
        io::ErrorKind::NotFound => OsErrorClass::NotFound,
        io::ErrorKind::PermissionDenied => OsErrorClass::AccessDenied,
        io::ErrorKind::Unsupported => OsErrorClass::Unsupported,
        _ => OsErrorClass::Other,
    }
}
