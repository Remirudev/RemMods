//! Read-only registry access for Steam detection. Nothing here writes the registry.

use std::io;
use std::path::PathBuf;
use std::ptr;

use windows_sys::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA, ERROR_PATH_NOT_FOUND, ERROR_SUCCESS,
    ERROR_UNSUPPORTED_TYPE,
};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, REG_ROUTINE_FLAGS, RRF_RT_REG_SZ,
    RRF_SUBKEY_WOW6432KEY, RRF_SUBKEY_WOW6464KEY, RegGetValueW,
};

use super::wide_nul;
use crate::platform::{
    OsKind, RegistryError, RegistryHive, RegistrySteamValues, RegistryView, SteamHints,
};

/// MAX_PATH units; Steam paths fit, longer values grow the buffer.
const INITIAL_UNITS: usize = 260;
const MAX_TRIES: u32 = 3;

pub(in crate::platform) fn registry_string(
    hive: RegistryHive,
    subkey: &str,
    value: &str,
    view: RegistryView,
) -> io::Result<Option<String>> {
    let subkey_wide = wide_nul(subkey);
    let value_wide = wide_nul(value);
    let flags = RRF_RT_REG_SZ | view_flag(view);
    let mut buffer = vec![0u16; INITIAL_UNITS];
    for _ in 0..MAX_TRIES {
        let mut byte_len = u32::try_from(buffer.len() * 2).unwrap_or(u32::MAX);
        // SAFETY: both names are NUL-terminated UTF-16 that outlive the call; `buffer` is
        // writable for `byte_len` bytes, and `byte_len` is a valid in/out size pointer.
        let status = unsafe {
            RegGetValueW(
                hive_key(hive),
                subkey_wide.as_ptr(),
                value_wide.as_ptr(),
                flags,
                ptr::null_mut(),
                buffer.as_mut_ptr().cast(),
                &mut byte_len,
            )
        };
        match status {
            ERROR_SUCCESS => return decode(&buffer, byte_len).map(Some),
            ERROR_MORE_DATA => buffer.resize(byte_len as usize / 2 + 1, 0),
            ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => return Ok(None),
            ERROR_UNSUPPORTED_TYPE => {
                log::warn!("registry value {subkey}\\{value} is not a string; ignored");
                return Ok(None);
            }
            other => return Err(io::Error::from_raw_os_error(other as i32)),
        }
    }
    Err(io::Error::other(
        "registry value kept growing while it was read",
    ))
}

pub(in crate::platform) fn steam_hints() -> SteamHints {
    let mut errors = Vec::new();
    let steam_path = read_hint(
        &mut errors,
        RegistryHive::CurrentUser,
        r"Software\Valve\Steam",
        "SteamPath",
        RegistryView::Default,
        r"HKCU\Software\Valve\Steam\SteamPath",
    );
    let install_path_32 = read_hint(
        &mut errors,
        RegistryHive::LocalMachine,
        r"SOFTWARE\Valve\Steam",
        "InstallPath",
        RegistryView::Wow32,
        r"HKLM\SOFTWARE\Valve\Steam\InstallPath (32-bit)",
    );
    let install_path_64 = read_hint(
        &mut errors,
        RegistryHive::LocalMachine,
        r"SOFTWARE\Valve\Steam",
        "InstallPath",
        RegistryView::Wow64,
        r"HKLM\SOFTWARE\Valve\Steam\InstallPath (64-bit)",
    );
    SteamHints {
        os: OsKind::Windows,
        home: dirs::home_dir(),
        data_dir: dirs::data_dir(),
        registry: RegistrySteamValues {
            steam_path,
            install_path_32,
            install_path_64,
            errors,
        },
        program_files_x86: env_path("ProgramFiles(x86)"),
        program_files: env_path("ProgramFiles"),
    }
}

fn read_hint(
    errors: &mut Vec<RegistryError>,
    hive: RegistryHive,
    subkey: &str,
    value: &str,
    view: RegistryView,
    label: &str,
) -> Option<String> {
    match registry_string(hive, subkey, value, view) {
        Ok(found) => found,
        Err(error) => {
            log::warn!("registry value {label} unreadable: {error}");
            errors.push(RegistryError {
                value: label.to_owned(),
                not_unicode: error.kind() == io::ErrorKind::InvalidData,
            });
            None
        }
    }
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn hive_key(hive: RegistryHive) -> HKEY {
    match hive {
        RegistryHive::CurrentUser => HKEY_CURRENT_USER,
        RegistryHive::LocalMachine => HKEY_LOCAL_MACHINE,
    }
}

fn view_flag(view: RegistryView) -> REG_ROUTINE_FLAGS {
    match view {
        RegistryView::Default => 0,
        RegistryView::Wow32 => RRF_SUBKEY_WOW6432KEY,
        RegistryView::Wow64 => RRF_SUBKEY_WOW6464KEY,
    }
}

/// Cut to the returned byte count, trailing NULs dropped, strict UTF-16: an unpaired surrogate
/// is `InvalidData`, because a lossy path would name another folder.
fn decode(buffer: &[u16], byte_len: u32) -> io::Result<String> {
    let units = (byte_len as usize / 2).min(buffer.len());
    let mut text = &buffer[..units];
    while let [rest @ .., 0] = text {
        text = rest;
    }
    String::from_utf16(text).map_err(|_| io::Error::from(io::ErrorKind::InvalidData))
}
