//! The environment of programs RemMods starts (PLAN §5 `host_command`). Inside an AppImage, AppRun
//! and the GTK hook point library and data paths into the mount; a host program must not inherit
//! them, nor the renderer variables RemMods set for itself. Cfg-free: AppImage values are
//! compared by their encoded form, which the tests also run on Windows.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use super::entry::{RawName, os_string, raw_name};

/// Variables the AppImage runtime sets for the mounted app.
pub const APPIMAGE_RUNTIME_VARS: &[&str] = &["APPDIR", "APPIMAGE", "ARGV0", "OWD"];
/// Variables AppRun and the GTK hook set to single values inside the mount.
pub const APPRUN_VALUE_VARS: &[&str] = &[
    "PYTHONHOME",
    "PYTHONDONTWRITEBYTECODE",
    "GTK_THEME",
    "GTK_DATA_PREFIX",
    "GTK_EXE_PREFIX",
    "GTK_PATH",
    "GTK_IM_MODULE_FILE",
    "GDK_PIXBUF_MODULE_FILE",
    "GIO_MODULE_DIR",
    "GSETTINGS_SCHEMA_DIR",
    "GI_TYPELIB_PATH",
    "APPIMAGE_GTK_THEME",
];
/// `:`-separated lists AppRun prepends mount entries to.
pub const APPRUN_PATH_LISTS: &[&str] = &[
    "PATH",
    "LD_LIBRARY_PATH",
    "XDG_DATA_DIRS",
    "XDG_CONFIG_DIRS",
    "PYTHONPATH",
    "PERLLIB",
    "QT_PLUGIN_PATH",
];
/// `PATH` when nothing but mount entries was left.
pub const FALLBACK_PATH: &str = "/usr/local/bin:/usr/bin:/bin";

const LIST_SEPARATOR: u8 = b':';
const SLASH: u8 = b'/';

/// What to change in an inherited environment: names to remove, then values to set.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HostEnvPlan {
    pub remove: Vec<OsString>,
    pub set: Vec<(OsString, OsString)>,
}

/// `own_vars` (the renderer variables RemMods set itself) are always removed. With an AppImage
/// `APPDIR` (non-empty, first byte `/`), the runtime and AppRun value variables are removed too and
/// the path lists lose every entry inside the mount.
pub fn host_env_plan(env: &[(OsString, OsString)], own_vars: &[&str]) -> HostEnvPlan {
    let mut plan = HostEnvPlan::default();
    for name in own_vars {
        if lookup(env, name).is_some() {
            plan.remove_name(name);
        }
    }
    let Some(appdir) = lookup(env, "APPDIR").filter(|appdir| is_appimage_dir(appdir)) else {
        return plan;
    };
    let single_values = APPIMAGE_RUNTIME_VARS.iter().chain(APPRUN_VALUE_VARS);
    for name in single_values {
        if lookup(env, name).is_some() {
            plan.remove_name(name);
        }
    }
    for name in APPRUN_PATH_LISTS {
        let Some(value) = lookup(env, name) else {
            continue;
        };
        if plan.removes(name) {
            continue;
        }
        let Some(cleaned) = clean_list(value, appdir) else {
            continue;
        };
        if cleaned.as_os_str() == value {
            continue;
        }
        if !cleaned.is_empty() {
            plan.set.push((OsString::from(name), cleaned));
        } else if *name == "PATH" {
            plan.set
                .push((OsString::from(name), OsString::from(FALLBACK_PATH)));
        } else {
            plan.remove_name(name);
        }
    }
    plan
}

/// PLAN §5: CLI paths resolve against `$OWD` inside an AppImage (AppRun changes into the mount).
pub fn resolve_cli_path(path: &Path, owd: Option<&OsStr>, appdir: Option<&OsStr>) -> PathBuf {
    match (owd, appdir) {
        (Some(owd), Some(appdir))
            if !path.has_root() && is_appimage_dir(owd) && is_appimage_dir(appdir) =>
        {
            Path::new(owd).join(path)
        }
        _ => path.to_path_buf(),
    }
}

impl HostEnvPlan {
    fn removes(&self, name: &str) -> bool {
        self.remove.iter().any(|removed| removed == name)
    }

    fn remove_name(&mut self, name: &str) {
        if !self.removes(name) {
            self.remove.push(OsString::from(name));
        }
    }
}

fn lookup<'a>(env: &'a [(OsString, OsString)], name: &str) -> Option<&'a OsStr> {
    env.iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_os_str())
}

/// Non-empty and starting with `/` (AppImage is Linux-only; `Path::is_absolute` would differ on
/// Windows).
fn is_appimage_dir(value: &OsStr) -> bool {
    value.as_encoded_bytes().first() == Some(&SLASH)
}

/// `value` without empty entries, entries equal to `appdir` or below it, and later duplicates.
/// None when the OS form of the two values differs (never happens for one environment).
fn clean_list(value: &OsStr, appdir: &OsStr) -> Option<OsString> {
    let cleaned = match (raw_name(value), raw_name(appdir)) {
        (RawName::Bytes(value), RawName::Bytes(appdir)) => {
            RawName::Bytes(clean_units(&value, &appdir, LIST_SEPARATOR, SLASH))
        }
        (RawName::Wide(value), RawName::Wide(appdir)) => RawName::Wide(clean_units(
            &value,
            &appdir,
            u16::from(LIST_SEPARATOR),
            u16::from(SLASH),
        )),
        _ => return None,
    };
    os_string(&cleaned)
}

fn clean_units<T: Copy + PartialEq>(value: &[T], appdir: &[T], separator: T, slash: T) -> Vec<T> {
    let mut kept: Vec<&[T]> = Vec::new();
    for entry in value.split(|unit| *unit == separator) {
        let inside = entry
            .strip_prefix(appdir)
            .is_some_and(|rest| rest.is_empty() || rest.first() == Some(&slash));
        if entry.is_empty() || inside || kept.contains(&entry) {
            continue;
        }
        kept.push(entry);
    }
    kept.join(&separator)
}
