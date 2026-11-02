//! The only module with `cfg` and `unsafe`. Every function here has the same signature on every
//! OS; the per-OS files only fill in the details. Raw OS error codes collide across systems
//! (Windows 5/32/33 are EIO/EPIPE/EDOM on Linux), so errors are classified here and nowhere else.

use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};

mod entry;
mod host_env;
mod meminfo;
mod procfs;
mod trash;

pub(crate) use entry::os_str_prefix;
pub use entry::{FileId, RawName, file_id, fs_path, os_str_cut, os_string, raw_name};
pub use host_env::{
    APPIMAGE_RUNTIME_VARS, APPRUN_PATH_LISTS, APPRUN_VALUE_VARS, FALLBACK_PATH, HostEnvPlan,
    host_env_plan, resolve_cli_path,
};
pub use meminfo::parse_meminfo_total;
pub use procfs::{ProcessRecord, parse_cmdline, parse_comm, read_proc_tree};
pub(crate) use trash::DirTraits;
pub use trash::{
    MountId, Recycled, TopdirPolicy, TrashEnv, TrashKind, TrashLocation, TrashUnavailableReason,
};

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

/// The renderer variables this process set for itself (`os::graphics::prepare`).
static OWN_ENV_VARS: OnceLock<Vec<&'static str>> = OnceLock::new();

/// OS-independent meaning of an I/O error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OsErrorClass {
    InUse,
    AccessDenied,
    NoSpace,
    ReadOnlyFs,
    CrossDevice,
    NameTooLong,
    AlreadyExists,
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

/// Outcome of [`rename_noreplace`] when the OS call itself did not fail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NoReplace {
    Done,
    /// The file system rejects the no-replace flag (ntfs-3g, NFS, old kernels); nothing changed.
    /// Windows never reports it.
    #[cfg_attr(windows, allow(dead_code))]
    FlagUnsupported,
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

/// Policy for renaming a mod folder. Windows: a folder that Explorer, an indexer or an antivirus
/// scanner holds fails with InUse or AccessDenied for a while, so those retry for 30 s.
pub fn folder_retry_policy(class: OsErrorClass) -> RetryPolicy {
    imp::folder_retry_policy(class)
}

/// Renames `from` to `to` (same parent folder) and never replaces an existing `to`. Linux:
/// renameat2(RENAME_NOREPLACE); EINVAL or ENOSYS mean the flag is unsupported. Windows:
/// MoveFileExW without REPLACE_EXISTING or COPY_ALLOWED on verbatim paths. Other OSes: Unsupported.
pub(crate) fn rename_noreplace(from: &Path, to: &Path) -> io::Result<NoReplace> {
    if from.parent() != to.parent() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "rename_noreplace needs one parent folder",
        ));
    }
    imp::rename_noreplace(from, to)
}

/// Rename to a different folder on the same mount, never replacing `to`. Linux: renameat2
/// NOREPLACE; EINVAL/ENOSYS → `NoReplace::FlagUnsupported` and nothing changed. Windows:
/// MoveFileExW(verbatim, 0). EXDEV is classified CrossDevice.
pub(crate) fn rename_noreplace_across(from: &Path, to: &Path) -> io::Result<NoReplace> {
    imp::rename_noreplace_across(from, to)
}

/// mkdir with mode 0700 on Unix (plain create_dir on Windows); AlreadyExists is an error.
pub(crate) fn create_private_dir(path: &Path) -> io::Result<()> {
    imp::create_private_dir(path)
}

/// open(O_CREAT|O_EXCL|O_WRONLY, 0600) on Unix, CREATE_NEW on Windows.
pub(crate) fn create_new_file(path: &Path) -> io::Result<std::fs::File> {
    imp::create_new_file(path)
}

/// Owner, kind and sticky bit of `path` without following it (owner None and sticky false on
/// Windows).
pub(crate) fn dir_traits(path: &Path) -> io::Result<DirTraits> {
    imp::dir_traits(path)
}

/// Linux: the real uid; Windows and other OSes: None. Read-only.
pub fn current_uid() -> Option<u32> {
    imp::current_uid()
}

/// Read-only probe for the trash that `item` (an existing absolute path, not followed if it is a
/// link) would move into. Linux: the freedesktop.org rules, never `RecycleBin`. Windows: `Detect`
/// → the volume's Recycle Bin (no drive-type or bin-size probe: the shell decides and `recycle`
/// never deletes), `Deny`/`Fixed` → `Ok(Err(NotSupported))`. Other OSes: `Ok(Err(NotSupported))`.
pub fn trash_location(
    env: &TrashEnv,
    item: &Path,
) -> io::Result<Result<TrashLocation, TrashUnavailableReason>> {
    imp::trash_location(env, item)
}

/// Windows: moves one existing file or folder into its volume's Recycle Bin through
/// IFileOperation on a fresh STA thread and never deletes it (PLAN §4 Trash). `force_abort`
/// (tests): the sink refuses every delete as if the shell could not recycle. Linux/other:
/// Err(Unsupported).
pub(crate) fn recycle(item: &Path, force_abort: bool) -> io::Result<Recycled> {
    imp::recycle(item, force_abort)
}

/// Tests only (W10): initialises COM as MTA on the calling thread, so the test needs no
/// `unsafe`. Linux/other: Err(Unsupported).
#[doc(hidden)]
pub fn enter_mta_for_tests() -> io::Result<()> {
    imp::enter_mta_for_tests()
}

/// Linux: statx(AT_SYMLINK_NOFOLLOW, STATX_MNT_ID) → (st_dev, mount id when the kernel reports
/// it). Windows/other: Unsupported.
pub fn mount_identity(path: &Path) -> io::Result<MountId> {
    imp::mount_identity(path)
}

/// Windows: opens `path` for DELETE with full sharing and closes it again; a holder that did not
/// grant FILE_SHARE_DELETE (the game) makes it fail with InUse. Other OSes: Ok at once.
pub fn lock_probe(path: &Path, is_dir: bool) -> io::Result<()> {
    imp::lock_probe(path, is_dir)
}

/// Names of the processes holding `paths`. P2: None everywhere (Restart Manager is WG-1).
pub fn who_holds(_paths: &[PathBuf]) -> Option<Vec<String>> {
    None
}

/// Physical memory in bytes. Linux: `MemTotal` from `/proc/meminfo`; Windows:
/// `GlobalMemoryStatusEx`; other OSes: None. Read-only.
pub fn total_memory() -> Option<u64> {
    imp::total_memory()
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

/// Records the environment variables RemMods set for itself, so host programs do not inherit
/// them. Only the first call counts; a later one is logged and ignored.
pub fn set_own_env_vars(vars: Vec<&'static str>) {
    if let Err(ignored) = OWN_ENV_VARS.set(vars) {
        log::warn!("own environment variables already recorded; ignoring {ignored:?}");
    }
}

pub fn own_env_vars() -> &'static [&'static str] {
    OWN_ENV_VARS.get().map_or(&[], Vec::as_slice)
}

/// A command for a host program with the environment of this process, without RemMods' own
/// renderer variables and AppImage mount paths ([`host_env_plan`]): working directory = home when
/// known, all three streams null.
pub fn host_command(program: impl AsRef<OsStr>) -> Command {
    let env: Vec<(OsString, OsString)> = std::env::vars_os().collect();
    host_command_in(program, &env, own_env_vars(), dirs::home_dir().as_deref())
}

/// [`host_command`] with the inherited environment, own variables and home passed in.
pub fn host_command_in(
    program: impl AsRef<OsStr>,
    env: &[(OsString, OsString)],
    own_vars: &[&str],
    home: Option<&Path>,
) -> Command {
    let plan = host_env_plan(env, own_vars);
    let mut command = Command::new(program);
    for name in &plan.remove {
        command.env_remove(name);
    }
    for (name, value) in &plan.set {
        command.env(name, value);
    }
    if let Some(home) = home {
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
        io::ErrorKind::AlreadyExists => OsErrorClass::AlreadyExists,
        _ => OsErrorClass::Other,
    }
}
