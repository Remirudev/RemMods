//! Toolhelp32 process list: names only, Windows has no cheap way to read other command lines.

use std::io;

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_NO_MORE_FILES, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};

use crate::platform::ProcessRecord;

struct Snapshot(HANDLE);

impl Drop for Snapshot {
    fn drop(&mut self) {
        // SAFETY: the handle came from CreateToolhelp32Snapshot and is closed exactly once.
        unsafe { CloseHandle(self.0) };
    }
}

pub(in crate::platform) fn processes() -> io::Result<Vec<ProcessRecord>> {
    // SAFETY: no pointers are passed; the process id is ignored for TH32CS_SNAPPROCESS.
    let handle = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let snapshot = Snapshot(handle);
    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut records = Vec::new();
    // SAFETY: `entry` is a writable PROCESSENTRY32W whose dwSize is set, as the API requires.
    let mut more = unsafe { Process32FirstW(snapshot.0, &mut entry) } != 0;
    while more {
        records.push(ProcessRecord {
            pid: entry.th32ProcessID,
            name: utf16_until_nul(&entry.szExeFile),
            argv: Vec::new(),
        });
        // SAFETY: as above.
        more = unsafe { Process32NextW(snapshot.0, &mut entry) } != 0;
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(ERROR_NO_MORE_FILES as i32) {
        records.sort_unstable_by_key(|record| record.pid);
        Ok(records)
    } else {
        Err(error)
    }
}

fn utf16_until_nul(units: &[u16]) -> String {
    let len = units
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(units.len());
    String::from_utf16_lossy(&units[..len])
}
