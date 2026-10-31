use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};

use crate::paths::AppPaths;
use crate::safe_fs::{self, AppLock, FsError};

pub const SETTINGS_VERSION: u32 = 1;
pub const UI_SCALE_MIN: f64 = 0.8;
pub const UI_SCALE_MAX: f64 = 1.5;
pub const UI_SCALE_DEFAULT: f64 = 1.0;
const UI_SCALE_STEPS_PER_UNIT: f64 = 20.0;

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("settings are read-only (newer format or unreadable file)")]
    ReadOnly,
    #[error("invalid value for {field}")]
    InvalidValue { field: &'static str },
    #[error("cannot encode settings: {0}")]
    Encode(#[from] serde_json::Error),
    #[error(transparent)]
    Fs(#[from] FsError),
}

/// On-disk `settings.json`. Missing fields take defaults and unknown fields are ignored.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub version: u32,
    pub appearance: Appearance,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Appearance {
    pub scheme: ColorScheme,
    pub ui_scale: f64,
    pub title_bar: TitleBarMode,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorScheme {
    #[default]
    Dark,
    Light,
    System,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TitleBarMode {
    #[default]
    Custom,
    System,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SettingsUpdate {
    pub appearance: Option<AppearanceUpdate>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AppearanceUpdate {
    pub scheme: Option<ColorScheme>,
    pub ui_scale: Option<f64>,
    pub title_bar: Option<TitleBarMode>,
}

/// Settings plus how they were loaded: `read_only` = never written back (newer format or
/// unreadable file); `recovered` = the file was corrupt, backed up and replaced by defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingsSnapshot {
    pub settings: Settings,
    pub read_only: bool,
    pub recovered: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            appearance: Appearance::default(),
        }
    }
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            scheme: ColorScheme::default(),
            ui_scale: UI_SCALE_DEFAULT,
            title_bar: TitleBarMode::default(),
        }
    }
}

impl Settings {
    fn normalized(mut self) -> Self {
        self.version = SETTINGS_VERSION;
        self.appearance.ui_scale =
            normalize_ui_scale(self.appearance.ui_scale).unwrap_or(UI_SCALE_DEFAULT);
        self
    }

    fn with_update(mut self, update: &SettingsUpdate) -> Self {
        if let Some(appearance) = &update.appearance {
            let current = &mut self.appearance;
            current.scheme = appearance.scheme.unwrap_or(current.scheme);
            current.ui_scale = appearance.ui_scale.unwrap_or(current.ui_scale);
            current.title_bar = appearance.title_bar.unwrap_or(current.title_bar);
        }
        self.normalized()
    }
}

impl SettingsUpdate {
    fn validate(&self) -> Result<(), SettingsError> {
        let ui_scale = self.appearance.and_then(|appearance| appearance.ui_scale);
        match ui_scale {
            Some(value) if !value.is_finite() => Err(SettingsError::InvalidValue {
                field: "appearance.uiScale",
            }),
            _ => Ok(()),
        }
    }
}

impl SettingsSnapshot {
    fn clean(settings: Settings) -> Self {
        Self {
            settings,
            read_only: false,
            recovered: false,
        }
    }

    fn read_only_defaults() -> Self {
        Self {
            settings: Settings::default(),
            read_only: true,
            recovered: false,
        }
    }
}

/// Clamps to 80–150 % and rounds to 5 % steps; `None` for NaN or infinity.
pub fn normalize_ui_scale(value: f64) -> Option<f64> {
    if !value.is_finite() {
        return None;
    }
    let clamped = value.clamp(UI_SCALE_MIN, UI_SCALE_MAX);
    Some((clamped * UI_SCALE_STEPS_PER_UNIT).round() / UI_SCALE_STEPS_PER_UNIT)
}

#[derive(Debug)]
pub struct SettingsStore {
    file: PathBuf,
    lock_file: PathBuf,
    state: Mutex<StoreState>,
}

#[derive(Debug)]
struct StoreState {
    snapshot: SettingsSnapshot,
    backed_up_fingerprint: Option<u64>,
}

enum DiskSettings {
    Missing,
    Valid(Settings),
    Newer(u64),
    Corrupt { fingerprint: u64 },
}

impl SettingsStore {
    /// Never fails: problems degrade to defaults, flagged `recovered` or `read_only`.
    pub fn load(paths: &AppPaths) -> Self {
        let file = paths.settings_file.clone();
        let mut backed_up_fingerprint = None;
        let snapshot = match read_disk(&file) {
            Ok(DiskSettings::Missing) => SettingsSnapshot::clean(Settings::default()),
            Ok(DiskSettings::Valid(settings)) => SettingsSnapshot::clean(settings),
            Ok(DiskSettings::Newer(version)) => {
                log::warn!(
                    "{} has format version {version}; using defaults read-only",
                    file.display()
                );
                SettingsSnapshot::read_only_defaults()
            }
            Ok(DiskSettings::Corrupt { fingerprint }) => match safe_fs::backup_file(&file) {
                Ok(backup) => {
                    log::warn!(
                        "{} is unreadable as settings; backed up to {}, using defaults",
                        file.display(),
                        backup.display()
                    );
                    replace_corrupt_with_defaults(&file, &paths.lock_file, fingerprint);
                    backed_up_fingerprint = Some(fingerprint);
                    SettingsSnapshot {
                        recovered: true,
                        ..SettingsSnapshot::clean(Settings::default())
                    }
                }
                Err(error) => {
                    log::error!("corrupt settings could not be backed up: {error}");
                    SettingsSnapshot::read_only_defaults()
                }
            },
            Err(error) => {
                log::error!("cannot read settings, using defaults read-only: {error}");
                SettingsSnapshot::read_only_defaults()
            }
        };
        Self {
            file,
            lock_file: paths.lock_file.clone(),
            state: Mutex::new(StoreState {
                snapshot,
                backed_up_fingerprint,
            }),
        }
    }

    pub fn snapshot(&self) -> SettingsSnapshot {
        self.lock_state().snapshot.clone()
    }

    /// Validates, takes the app lock, re-reads the file, merges and writes atomically.
    pub fn update(&self, update: &SettingsUpdate) -> Result<SettingsSnapshot, SettingsError> {
        let mut state = self.lock_state();
        if state.snapshot.read_only {
            return Err(SettingsError::ReadOnly);
        }
        update.validate()?;
        let _app_lock = AppLock::try_exclusive(&self.lock_file)?;
        let base = match read_disk(&self.file)? {
            DiskSettings::Missing => Settings::default(),
            DiskSettings::Valid(settings) => settings,
            DiskSettings::Newer(version) => {
                log::warn!("settings became format version {version}; now read-only");
                state.snapshot.read_only = true;
                return Err(SettingsError::ReadOnly);
            }
            DiskSettings::Corrupt { fingerprint } => {
                if state.backed_up_fingerprint != Some(fingerprint) {
                    safe_fs::backup_file(&self.file)?;
                    state.backed_up_fingerprint = Some(fingerprint);
                }
                state.snapshot.recovered = true;
                Settings::default()
            }
        };
        let next = base.with_update(update);
        safe_fs::write_atomic(&self.file, &encode(&next)?)?;
        state.snapshot.settings = next;
        Ok(state.snapshot.clone())
    }

    fn lock_state(&self) -> MutexGuard<'_, StoreState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Writes defaults over a corrupt file once it is backed up, so the next start does not back up
/// the same bytes again. If another process holds the lock or the file changed meanwhile, the
/// file stays as it is until the next save.
fn replace_corrupt_with_defaults(file: &Path, lock_file: &Path, fingerprint: u64) {
    match write_defaults_if_unchanged(file, lock_file, fingerprint) {
        Ok(true) => log::info!("replaced {} with the default settings", file.display()),
        Ok(false) => log::info!("{} changed meanwhile; left as it is", file.display()),
        Err(error) => log::warn!("corrupt settings stay in place until the next save: {error}"),
    }
}

fn write_defaults_if_unchanged(
    file: &Path,
    lock_file: &Path,
    fingerprint: u64,
) -> Result<bool, SettingsError> {
    let _app_lock = AppLock::try_exclusive(lock_file)?;
    match read_disk(file)? {
        DiskSettings::Corrupt {
            fingerprint: current,
        } if current == fingerprint => {
            safe_fs::write_atomic(file, &encode(&Settings::default())?)?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn read_disk(file: &Path) -> Result<DiskSettings, FsError> {
    match std::fs::read(file) {
        Ok(bytes) => Ok(parse_settings(&bytes)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(DiskSettings::Missing),
        Err(error) => Err(FsError::io("read", file, error)),
    }
}

fn parse_settings(bytes: &[u8]) -> DiskSettings {
    let corrupt = || DiskSettings::Corrupt {
        fingerprint: fingerprint(bytes),
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return corrupt();
    };
    if let Some(version) = value.get("version").and_then(serde_json::Value::as_u64)
        && version > u64::from(SETTINGS_VERSION)
    {
        return DiskSettings::Newer(version);
    }
    match serde_json::from_value::<Settings>(value) {
        Ok(settings) => DiskSettings::Valid(settings.normalized()),
        Err(_) => corrupt(),
    }
}

fn encode(settings: &Settings) -> Result<Vec<u8>, SettingsError> {
    let mut bytes = serde_json::to_vec_pretty(settings)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn fingerprint(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}
