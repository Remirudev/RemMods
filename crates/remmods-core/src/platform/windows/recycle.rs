//! The Recycle Bin mover: IFileOperation on a fresh STA thread whose progress sink refuses every
//! delete the shell would not send to the Recycle Bin. An item ends in the bin or stays untouched;
//! the result is decided by the disk, never by the shell's return codes alone.
// DeleteItem and PerformOperations are shell mutators; only safe_fs calls `recycle`, after its
// scope, journal and name checks.
#![allow(clippy::disallowed_methods)]

use std::ffi::{OsString, c_void};
use std::fs;
use std::io;
use std::os::windows::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;

use windows::Win32::Foundation::E_ABORT;
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, COINIT_MULTITHREADED,
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IBindCtx,
};
use windows::Win32::System::SystemServices::SFGAO_FILESYSTEM;
use windows::Win32::UI::Shell::{
    COPYENGINE_E_ACCESS_DENIED_DEST, COPYENGINE_E_ACCESS_DENIED_SRC,
    COPYENGINE_E_REQUIRES_ELEVATION, COPYENGINE_E_SHARING_VIOLATION_DEST,
    COPYENGINE_E_SHARING_VIOLATION_SRC, FOF_ALLOWUNDO, FOF_NO_UI, FOFX_EARLYFAILURE,
    FOFX_RECYCLEONDELETE, FileOperation, IFileOperation, IFileOperationProgressSink,
    IFileOperationProgressSink_Impl, IShellItem, SHCreateItemFromParsingName, SIGDN_FILESYSPATH,
    TSF_DELETE_RECYCLE_IF_POSSIBLE,
};
use windows_core::{HRESULT, PCWSTR, Ref, implement};
use windows_sys::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_SHARING_VIOLATION};
use windows_sys::Win32::Storage::FileSystem::GetFullPathNameW;

use super::super::{Recycled, fs_path};

const VERBATIM: &str = r"\\?\";
const VERBATIM_UNC: &str = r"\\?\UNC\";
/// Shell APIs are not long-path aware: MAX_PATH minus the NUL.
const SHELL_MAX_UNITS: usize = 259;
const WIN32_FACILITY_MASK: u32 = 0xFFFF_0000;
const WIN32_FACILITY: u32 = 0x8007_0000;

/// What the sink saw. `root_post`: the root's PostDeleteItem result and its new item in the bin;
/// `other_ok`: children the shell already moved into the bin.
#[derive(Debug, Default)]
struct SinkState {
    refused: bool,
    root_post: Option<(HRESULT, Option<PathBuf>)>,
    other_ok: u32,
}

/// Every COM result as plain data: `windows::core::Error` carries a thread-bound IErrorInfo and
/// never leaves the STA thread.
enum ShellRun {
    NotShellSafe,
    SetupFailed(HRESULT),
    Ran {
        performed: Result<(), HRESULT>,
        aborted: bool,
        state: SinkState,
    },
}

pub(in crate::platform) fn recycle(item: &Path, force_abort: bool) -> io::Result<Recycled> {
    let verbatim = fs_path(item)?;
    fs::symlink_metadata(&verbatim)?;
    let Some(plain) = plain_form(&verbatim) else {
        return Ok(Recycled::NotShellSafe);
    };
    if plain.len() > SHELL_MAX_UNITS || full_path_name(&plain)? != plain {
        return Ok(Recycled::NotShellSafe);
    }
    // A fresh thread: the caller's apartment is unknown, and CoInitializeEx with another model
    // fails with RPC_E_CHANGED_MODE. The operation has no UI, so no timeout.
    let run = thread::Builder::new()
        .name("recycle-sta".to_owned())
        .spawn(move || shell_delete(plain, force_abort))?
        .join()
        .map_err(|_| io::Error::other("the recycle thread panicked"))?;
    match run {
        ShellRun::NotShellSafe => Ok(Recycled::NotShellSafe),
        ShellRun::SetupFailed(hr) => Err(hresult_io(hr)),
        ShellRun::Ran {
            performed,
            aborted,
            state,
        } => decide(item, &verbatim, performed, aborted, state),
    }
}

/// Tests only (W10): the calling thread joins the MTA and stays there until it ends.
pub(in crate::platform) fn enter_mta_for_tests() -> io::Result<()> {
    // SAFETY: the reserved pointer is None; tests call it once per thread and never uninitialise.
    let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    if hr.is_err() {
        return Err(hresult_io(hr));
    }
    Ok(())
}

/// `\\?\X:\…` → `X:\…`, `\\?\UNC\server\share\…` → `\\server\share\…`; any other form (device
/// paths, `\\?\Volume{…}`) or a path that is not valid Unicode → None.
fn plain_form(verbatim: &Path) -> Option<Vec<u16>> {
    let text = verbatim.to_str()?;
    let plain = if let Some(rest) = text.strip_prefix(VERBATIM_UNC) {
        format!(r"\\{rest}")
    } else {
        let rest = text.strip_prefix(VERBATIM)?;
        let bytes = rest.as_bytes();
        let is_drive = bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && bytes[2] == b'\\';
        if !is_drive {
            return None;
        }
        rest.to_owned()
    };
    Some(plain.encode_utf16().collect())
}

/// What Win32 makes of `plain` (GetFullPathNameW); a path it rewrites never reaches the shell.
fn full_path_name(plain: &[u16]) -> io::Result<Vec<u16>> {
    let input = wide_nul(plain);
    let mut buffer = vec![0_u16; plain.len() + 16];
    loop {
        let capacity = u32::try_from(buffer.len()).map_err(|_| too_long())?;
        // SAFETY: `input` is NUL-terminated, `buffer` has `capacity` writable units, and the
        // file-part pointer may be null.
        let written = unsafe {
            GetFullPathNameW(
                input.as_ptr(),
                capacity,
                buffer.as_mut_ptr(),
                ptr::null_mut(),
            )
        };
        let written = usize::try_from(written).map_err(|_| too_long())?;
        if written == 0 {
            return Err(io::Error::last_os_error());
        }
        if written < buffer.len() {
            buffer.truncate(written);
            return Ok(buffer);
        }
        // Too small: `written` is the size needed, NUL included.
        buffer.resize(written, 0);
    }
}

/// Runs on the STA thread. The apartment guard is created first, so it drops after every COM
/// object of `run_operation`; the sink state is taken out after that.
fn shell_delete(plain: Vec<u16>, force_abort: bool) -> ShellRun {
    // SAFETY: the reserved pointer is None; this fresh thread has no apartment yet.
    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
    if hr.is_err() {
        return ShellRun::SetupFailed(hr);
    }
    let apartment = Apartment;
    let state = Arc::new(Mutex::new(SinkState::default()));
    let run = run_operation(&plain, force_abort, &state);
    drop(apartment);
    let state = std::mem::take(&mut *lock(&state));
    match run {
        Ok(Some((performed, aborted))) => ShellRun::Ran {
            performed,
            aborted,
            state,
        },
        Ok(None) => ShellRun::NotShellSafe,
        Err(hr) => ShellRun::SetupFailed(hr),
    }
}

/// `Ok(None)`: the shell item is not exactly `plain` (nothing was handed to DeleteItem).
#[allow(clippy::type_complexity)]
fn run_operation(
    plain: &[u16],
    force_abort: bool,
    state: &Arc<Mutex<SinkState>>,
) -> Result<Option<(Result<(), HRESULT>, bool)>, HRESULT> {
    // SAFETY: COM is initialised on this thread; FileOperation implements IFileOperation.
    let op: IFileOperation =
        unsafe { CoCreateInstance(&FileOperation, None::<&windows_core::IUnknown>, CLSCTX_ALL) }
            .map_err(|error| error.code())?;
    // Never FOF_WANTNUKEWARNING: the sink refuses a permanent delete instead of asking.
    let flags = FOF_NO_UI | FOF_ALLOWUNDO | FOFX_RECYCLEONDELETE | FOFX_EARLYFAILURE;
    // SAFETY: `op` is a live IFileOperation of this apartment.
    unsafe { op.SetOperationFlags(flags) }.map_err(|error| error.code())?;
    let wide = wide_nul(plain);
    // SAFETY: `wide` is NUL-terminated and outlives the call; no bind context.
    let item: IShellItem =
        unsafe { SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None::<&IBindCtx>) }
            .map_err(|error| error.code())?;
    if !is_file_system_item(&item, plain) {
        return Ok(None);
    }
    let sink: IFileOperationProgressSink = RecycleSink {
        force_abort,
        root: plain.to_vec(),
        state: Arc::clone(state),
    }
    .into();
    // SAFETY: `op` and `sink` are live objects of this apartment.
    let cookie = unsafe { op.Advise(&sink) }.map_err(|error| error.code())?;
    // SAFETY: as above; the global sink sees every delete, children included.
    let queued = unsafe { op.DeleteItem(&item, None::<&IFileOperationProgressSink>) };
    let performed = match queued {
        // SAFETY: as above; FOF_NO_UI shows no dialog.
        Ok(()) => unsafe { op.PerformOperations() }.map_err(|error| error.code()),
        Err(error) => {
            unadvise_logged(&op, cookie);
            return Err(error.code());
        }
    };
    // SAFETY: as above.
    let aborted = unsafe { op.GetAnyOperationsAborted() }.map_or(true, |flag| flag.as_bool());
    unadvise_logged(&op, cookie);
    Ok(Some((performed, aborted)))
}

/// The shell namespace has rewrites of its own (a folder `X.{CLSID}` parses as a junction into a
/// virtual folder): only a file-system item whose path is exactly `plain` may be deleted.
fn is_file_system_item(item: &IShellItem, plain: &[u16]) -> bool {
    // SAFETY: `item` is a live shell item of this apartment.
    let attributes = unsafe { item.GetAttributes(SFGAO_FILESYSTEM) };
    attributes.is_ok_and(|found| found.contains(SFGAO_FILESYSTEM))
        && file_system_path(item).as_deref() == Some(plain)
}

fn unadvise_logged(op: &IFileOperation, cookie: u32) {
    // SAFETY: `cookie` came from Advise on this `op`.
    if let Err(error) = unsafe { op.Unadvise(cookie) } {
        log::warn!("IFileOperation::Unadvise failed: {:#010x}", error.code().0);
    }
}

/// SIGDN_FILESYSPATH of a shell item as UTF-16 units; None when it has none.
fn file_system_path(item: &IShellItem) -> Option<Vec<u16>> {
    // SAFETY: `item` is live; the returned string is CoTaskMemAlloc'ed, copied, then freed once.
    unsafe {
        let name = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        if name.is_null() {
            return None;
        }
        let units = name.as_wide().to_vec();
        CoTaskMemFree(Some(name.0.cast_const().cast::<c_void>()));
        Some(units)
    }
}

/// Decided by the disk after the thread joined; `performed` and `aborted` only add log lines.
fn decide(
    item: &Path,
    verbatim: &Path,
    performed: Result<(), HRESULT>,
    aborted: bool,
    state: SinkState,
) -> io::Result<Recycled> {
    let gone = matches!(
        fs::symlink_metadata(verbatim),
        Err(error) if error.kind() == io::ErrorKind::NotFound
    );
    log::debug!(
        "recycle {}: performed {:?}, aborted {aborted}, {state:?}, gone {gone}",
        item.display(),
        performed.map_err(|hr| hr.0),
    );
    let root_hr = state.root_post.as_ref().map(|(hr, _)| *hr);
    if gone {
        if let Some((hr, Some(bin_path))) = state.root_post
            && hr.is_ok()
        {
            return Ok(Recycled::Done { bin_path });
        }
        log::error!(
            "the shell removed {} without reporting a Recycle Bin copy",
            item.display()
        );
        return Err(io::Error::other(
            "the shell removed the item without a Recycle Bin copy",
        ));
    }
    if state.other_ok > 0 {
        log::error!(
            "{} is partially recycled: {} entries are in the Recycle Bin",
            item.display(),
            state.other_ok
        );
        return Err(io::Error::other("partially recycled"));
    }
    if state.refused {
        return Ok(Recycled::Refused);
    }
    let failing = performed.err().or_else(|| root_hr.filter(|hr| hr.is_err()));
    Err(failing.map_or_else(
        || io::Error::other("the shell did not recycle the item"),
        hresult_io,
    ))
}

/// HRESULTs `platform::classify` understands: Win32 codes become raw OS errors, the copy engine's
/// sharing and access codes become 32 and 5. A recycle reports a file held open inside the item
/// as a destination sharing violation (windows-2025 CI), so both sides map alike.
fn hresult_io(hr: HRESULT) -> io::Error {
    let code = hr.0.cast_unsigned();
    if code & WIN32_FACILITY_MASK == WIN32_FACILITY {
        return io::Error::from_raw_os_error((code & 0xFFFF).cast_signed());
    }
    match hr {
        COPYENGINE_E_SHARING_VIOLATION_SRC | COPYENGINE_E_SHARING_VIOLATION_DEST => {
            io::Error::from_raw_os_error(ERROR_SHARING_VIOLATION.cast_signed())
        }
        COPYENGINE_E_ACCESS_DENIED_SRC
        | COPYENGINE_E_ACCESS_DENIED_DEST
        | COPYENGINE_E_REQUIRES_ELEVATION => {
            io::Error::from_raw_os_error(ERROR_ACCESS_DENIED.cast_signed())
        }
        E_ABORT => io::Error::from(io::ErrorKind::Interrupted),
        _ => io::Error::other(format!("shell error {code:#010x}")),
    }
}

fn wide_nul(units: &[u16]) -> Vec<u16> {
    units.iter().copied().chain(std::iter::once(0)).collect()
}

fn too_long() -> io::Error {
    io::Error::from(io::ErrorKind::InvalidInput)
}

fn lock(state: &Mutex<SinkState>) -> MutexGuard<'_, SinkState> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

fn path_from_wide(units: &[u16]) -> PathBuf {
    PathBuf::from(OsString::from_wide(units))
}

/// Ends the thread's COM apartment after every COM object of the call is gone.
struct Apartment;

impl Drop for Apartment {
    fn drop(&mut self) {
        // SAFETY: paired with the successful CoInitializeEx on this thread.
        unsafe { CoUninitialize() };
    }
}

/// `Arc<Mutex>`, not `Rc<RefCell>`: the shell may release the sink on a copy-engine thread.
#[implement(IFileOperationProgressSink)]
struct RecycleSink {
    force_abort: bool,
    root: Vec<u16>,
    state: Arc<Mutex<SinkState>>,
}

impl IFileOperationProgressSink_Impl for RecycleSink_Impl {
    fn StartOperations(&self) -> windows_core::Result<()> {
        Ok(())
    }

    fn FinishOperations(&self, _result: HRESULT) -> windows_core::Result<()> {
        Ok(())
    }

    fn PreRenameItem(
        &self,
        _flags: u32,
        _item: Ref<'_, IShellItem>,
        _new_name: &PCWSTR,
    ) -> windows_core::Result<()> {
        Ok(())
    }

    fn PostRenameItem(
        &self,
        _flags: u32,
        _item: Ref<'_, IShellItem>,
        _new_name: &PCWSTR,
        _result: HRESULT,
        _created: Ref<'_, IShellItem>,
    ) -> windows_core::Result<()> {
        Ok(())
    }

    fn PreMoveItem(
        &self,
        _flags: u32,
        _item: Ref<'_, IShellItem>,
        _destination: Ref<'_, IShellItem>,
        _new_name: &PCWSTR,
    ) -> windows_core::Result<()> {
        Ok(())
    }

    fn PostMoveItem(
        &self,
        _flags: u32,
        _item: Ref<'_, IShellItem>,
        _destination: Ref<'_, IShellItem>,
        _new_name: &PCWSTR,
        _result: HRESULT,
        _created: Ref<'_, IShellItem>,
    ) -> windows_core::Result<()> {
        Ok(())
    }

    fn PreCopyItem(
        &self,
        _flags: u32,
        _item: Ref<'_, IShellItem>,
        _destination: Ref<'_, IShellItem>,
        _new_name: &PCWSTR,
    ) -> windows_core::Result<()> {
        Ok(())
    }

    fn PostCopyItem(
        &self,
        _flags: u32,
        _item: Ref<'_, IShellItem>,
        _destination: Ref<'_, IShellItem>,
        _new_name: &PCWSTR,
        _result: HRESULT,
        _created: Ref<'_, IShellItem>,
    ) -> windows_core::Result<()> {
        Ok(())
    }

    /// A delete without the recycle flag would be permanent: abort it, nothing is deleted.
    fn PreDeleteItem(&self, flags: u32, _item: Ref<'_, IShellItem>) -> windows_core::Result<()> {
        let recycles = flags & TSF_DELETE_RECYCLE_IF_POSSIBLE.0.cast_unsigned() != 0;
        if self.force_abort || !recycles {
            lock(&self.state).refused = true;
            return Err(E_ABORT.into());
        }
        Ok(())
    }

    fn PostDeleteItem(
        &self,
        _flags: u32,
        item: Ref<'_, IShellItem>,
        result: HRESULT,
        created: Ref<'_, IShellItem>,
    ) -> windows_core::Result<()> {
        let is_root = item
            .as_ref()
            .and_then(file_system_path)
            .is_some_and(|path| path == self.root);
        let mut state = lock(&self.state);
        if is_root {
            let bin_path = created
                .as_ref()
                .and_then(file_system_path)
                .map(|units| path_from_wide(&units));
            state.root_post = Some((result, bin_path));
        } else if result.is_ok() {
            state.other_ok += 1;
        }
        Ok(())
    }

    fn PreNewItem(
        &self,
        _flags: u32,
        _destination: Ref<'_, IShellItem>,
        _new_name: &PCWSTR,
    ) -> windows_core::Result<()> {
        Ok(())
    }

    fn PostNewItem(
        &self,
        _flags: u32,
        _destination: Ref<'_, IShellItem>,
        _new_name: &PCWSTR,
        _template: &PCWSTR,
        _attributes: u32,
        _result: HRESULT,
        _created: Ref<'_, IShellItem>,
    ) -> windows_core::Result<()> {
        Ok(())
    }

    fn UpdateProgress(&self, _total: u32, _done: u32) -> windows_core::Result<()> {
        Ok(())
    }

    fn ResetTimer(&self) -> windows_core::Result<()> {
        Ok(())
    }

    fn PauseTimer(&self) -> windows_core::Result<()> {
        Ok(())
    }

    fn ResumeTimer(&self) -> windows_core::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::{OsErrorClass, classify};
    use super::*;

    #[test]
    fn hresult_io_maps_win32_and_copyengine_codes() {
        let cases = [
            (HRESULT(0x8007_0020_u32.cast_signed()), OsErrorClass::InUse),
            (HRESULT(0x8007_0021_u32.cast_signed()), OsErrorClass::InUse),
            (
                HRESULT(0x8007_0005_u32.cast_signed()),
                OsErrorClass::AccessDenied,
            ),
            (
                HRESULT(0x8007_0002_u32.cast_signed()),
                OsErrorClass::NotFound,
            ),
            (COPYENGINE_E_SHARING_VIOLATION_SRC, OsErrorClass::InUse),
            (COPYENGINE_E_SHARING_VIOLATION_DEST, OsErrorClass::InUse),
            (COPYENGINE_E_ACCESS_DENIED_SRC, OsErrorClass::AccessDenied),
            (COPYENGINE_E_ACCESS_DENIED_DEST, OsErrorClass::AccessDenied),
            (COPYENGINE_E_REQUIRES_ELEVATION, OsErrorClass::AccessDenied),
        ];
        for (hr, class) in cases {
            assert_eq!(classify(&hresult_io(hr)), class, "{:#010x}", hr.0);
        }
        assert_eq!(hresult_io(E_ABORT).kind(), io::ErrorKind::Interrupted);
        let other = hresult_io(HRESULT(0x8000_4005_u32.cast_signed()));
        assert_eq!(classify(&other), OsErrorClass::Other);
        assert!(other.to_string().contains("0x80004005"), "{other}");
    }

    #[test]
    fn plain_form_keeps_drive_and_unc_paths_only() {
        let plain =
            |text: &str| plain_form(Path::new(text)).map(|units| String::from_utf16_lossy(&units));
        assert_eq!(
            plain(r"\\?\C:\Games\Mods").as_deref(),
            Some(r"C:\Games\Mods")
        );
        assert_eq!(
            plain(r"\\?\UNC\srv\share\Mods").as_deref(),
            Some(r"\\srv\share\Mods")
        );
        assert_eq!(plain(r"\\?\Volume{0}\Mods"), None);
        assert_eq!(plain(r"\\.\C:\Mods"), None);
        assert_eq!(plain(r"C:\Mods"), None);
    }
}
