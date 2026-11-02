//! Self-update from Rust only (SPEC-P5 §5.4): no JS plugin package and no `updater:*` permission.
//! Every refusal comes before any network access or write; only AppImage and NSIS installs
//! replace themselves (the allowlist in `remmods_core::app_update`).

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, PoisonError};
use std::time::{Duration, Instant};

use remmods_core::Core;
use remmods_core::app_update::{
    self, BundleKind, InstallFacts, InstallKind, InstallPermission, UpdateBlock,
};
use remmods_ipc::{
    AppError, AppInstallKind, AppUpdateChannel, AppUpdateCheck, AppUpdateProgress,
    UpdateBlock as UpdateBlockDto, UpdateStage,
};
use tauri::ipc::Channel;
use tauri::utils::config::BundleType;
use tauri::{AppHandle, State};
use tauri_plugin_updater::{Update, Updater, UpdaterExt};

use super::{internal, run_blocking};
use crate::state::{AppState, Relaunch};

const CHECK_TIMEOUT: Duration = Duration::from_secs(30);
/// The plugin downloads without a timeout; a server or proxy that stops sending would keep the
/// non-closable progress modal up forever. A per-read bound fails a stall, never a slow download.
const DOWNLOAD_STALL: Duration = Duration::from_secs(60);
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

#[tauri::command]
#[specta::specta]
pub async fn update_channel(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AppUpdateChannel, AppError> {
    let kind = state.install_kind;
    Ok(AppUpdateChannel {
        install: kind.into(),
        self_update: app_update::install_permission(kind) == InstallPermission::SelfUpdate,
        configured: state.updater_configured,
        current: app.package_info().version.to_string(),
    })
}

/// Allowed for every install kind: a package user may still see that a version exists.
#[tauri::command]
#[specta::specta]
pub async fn update_check(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AppUpdateCheck, AppError> {
    if !state.updater_configured {
        return Err(AppError::UpdateNotConfigured);
    }
    let checked = updater(&app)?.check().await.map_err(|error| {
        log::warn!("update check failed: {error}");
        failed(UpdateStage::Check)
    })?;
    let result = match &checked {
        None => AppUpdateCheck::UpToDate,
        Some(update) => {
            log::info!("update {} available", update.version);
            AppUpdateCheck::Available {
                version: update.version.clone(),
                date: update.raw_json["pub_date"].as_str().map(str::to_owned),
                notes: update.body.clone(),
            }
        }
    };
    *state.update.lock().unwrap_or_else(PoisonError::into_inner) = checked;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn update_install(
    app: AppHandle,
    state: State<'_, AppState>,
    version: String,
    progress: Channel<AppUpdateProgress>,
) -> Result<(), AppError> {
    let update = allowed_update(&state, &version)?;
    let updating = UpdatingFlag::claim(&state.updating)?;
    run_blocking(state.core.clone(), Core::update_preflight).await?;
    let bytes = download(&update, &progress).await?;
    send(&progress, AppUpdateProgress::Installing);
    install(state.core.clone(), update, bytes).await?;
    updating.keep();
    after_install(&app, &state, &progress);
    Ok(())
}

/// The process's install kind, computed once at startup.
pub(crate) fn detect_install_kind() -> InstallKind {
    let exe = std::env::current_exe().ok();
    let facts = InstallFacts {
        bundle: current_bundle(),
        marker: exe.as_deref().and_then(app_update::read_marker),
        appimage_ok: app_update::appimage_target_ok(
            std::env::var_os("APPIMAGE").as_deref(),
            std::env::var_os("APPDIR").as_deref(),
            exe.as_deref(),
            Path::is_file,
        ),
    };
    app_update::classify_install(&facts)
}

/// What tauri-bundler patched into this binary; `None` for dev, `--no-bundle` and loose builds.
fn current_bundle() -> Option<BundleKind> {
    tauri::utils::platform::bundle_type().map(|bundle| match bundle {
        BundleType::Deb => BundleKind::Deb,
        BundleType::Rpm => BundleKind::Rpm,
        BundleType::AppImage => BundleKind::AppImage,
        BundleType::Nsis => BundleKind::Nsis,
        BundleType::Msi => BundleKind::Msi,
        BundleType::App | BundleType::Dmg => BundleKind::Other,
    })
}

/// Refusals 1–4 of SPEC-P5 §5.4, in order, before any network access or write.
fn allowed_update(state: &AppState, version: &str) -> Result<Update, AppError> {
    if !state.updater_configured {
        return Err(AppError::UpdateNotConfigured);
    }
    let kind = state.install_kind;
    if app_update::install_permission(kind) != InstallPermission::SelfUpdate {
        return Err(AppError::UpdateNotAllowed {
            install: kind.into(),
        });
    }
    if !app_update::may_self_install(kind, current_bundle()) {
        return Err(AppError::UpdateNotAllowed {
            install: AppInstallKind::Unknown,
        });
    }
    state
        .update
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .filter(|update| update.version == version)
        .cloned()
        .ok_or(AppError::Stale)
}

/// Only called when the plugin is registered (`updater_builder` panics otherwise).
fn updater(app: &AppHandle) -> Result<Updater, AppError> {
    app.updater_builder()
        .version_comparator(|current, release| {
            app_update::is_offered(&current.to_string(), &release.version.to_string())
        })
        .on_before_exit(before_exit())
        .timeout(CHECK_TIMEOUT)
        .configure_client(|client| client.read_timeout(DOWNLOAD_STALL))
        .build()
        .map_err(|error| {
            log::warn!("building the updater failed: {error}");
            failed(UpdateStage::Check)
        })
}

/// Windows only: runs after the installer was extracted, right before the plugin starts it.
/// It replaces the default `cleanup_before_exit()`, which hides every window and releases the
/// single-instance mutex: the plugin can still return `Err` after this hook (when the installer
/// does not start) and the app must then stay fully usable. On success `process::exit(0)` follows
/// and the OS releases everything, so flushing the log is all that is needed.
fn before_exit() -> impl Fn() + Send + Sync + 'static {
    move || {
        log::info!("starting the update installer; RemMods exits now");
        log::logger().flush();
    }
}

async fn download(
    update: &Update,
    progress: &Channel<AppUpdateProgress>,
) -> Result<Vec<u8>, AppError> {
    let mut meter = DownloadMeter::default();
    let downloaded = update
        .download(
            |chunk, total| meter.add(progress, chunk, total),
            || log::info!("update {} downloaded", update.version),
        )
        .await;
    meter.finish(progress);
    downloaded.map_err(|error| {
        log::warn!("downloading update {} failed: {error}", update.version);
        failed(stage_of(&error, UpdateStage::Download))
    })
}

/// Seals the operation gate, then lets the plugin replace the app. Windows never returns on
/// success (the installer restarts RemMods); on Linux the AppImage file was rewritten.
async fn install(core: Arc<Core>, update: Update, bytes: Vec<u8>) -> Result<(), AppError> {
    tauri::async_runtime::spawn_blocking(move || install_sealed(&core, &update, &bytes))
        .await
        .map_err(|error| internal("update install task", error))?
}

fn install_sealed(core: &Core, update: &Update, bytes: &[u8]) -> Result<(), AppError> {
    core.seal_for_update().map_err(AppError::from)?;
    log::info!("installing update {}", update.version);
    update.install(bytes).map_err(|error| {
        core.unseal_after_update();
        log::error!("installing update {} failed: {error}", update.version);
        failed(stage_of(&error, UpdateStage::Install))
    })
}

/// Linux AppImage: the file at `$APPIMAGE` is the new version. RemMods exits and relaunches it
/// through `host_command` on `RunEvent::Exit`. The gate stays sealed until the process exits.
fn after_install(app: &AppHandle, state: &AppState, progress: &Channel<AppUpdateProgress>) {
    send(progress, AppUpdateProgress::Restarting);
    match std::env::var_os("APPIMAGE") {
        Some(program) => {
            *state
                .relaunch
                .lock()
                .unwrap_or_else(PoisonError::into_inner) = Some(Relaunch {
                program: program.into(),
                args: Vec::new(),
                extract_and_run: app_update::appimage_extracted(
                    std::env::var_os("APPDIR").as_deref(),
                ),
            });
        }
        None => log::error!("APPIMAGE is unset after an update; not relaunching"),
    }
    log::info!("update installed; exiting to relaunch");
    app.exit(0);
}

fn stage_of(error: &tauri_plugin_updater::Error, otherwise: UpdateStage) -> UpdateStage {
    use tauri_plugin_updater::Error;
    match error {
        Error::Minisign(_)
        | Error::SignedVersionMismatch { .. }
        | Error::MissingSignedVersion
        | Error::Base64(_)
        | Error::SignatureUtf8(_) => UpdateStage::Verify,
        _ => otherwise,
    }
}

fn failed(stage: UpdateStage) -> AppError {
    AppError::UpdateFailed { stage }
}

fn send(progress: &Channel<AppUpdateProgress>, message: AppUpdateProgress) {
    if let Err(error) = progress.send(message) {
        log::debug!("update progress not delivered: {error}");
    }
}

/// Sums downloaded bytes and reports at most every 100 ms, plus once at the end.
#[derive(Default)]
struct DownloadMeter {
    downloaded: u64,
    total: Option<u64>,
    last_sent: Option<Instant>,
}

impl DownloadMeter {
    fn add(&mut self, progress: &Channel<AppUpdateProgress>, chunk: usize, total: Option<u64>) {
        self.downloaded = self
            .downloaded
            .saturating_add(u64::try_from(chunk).unwrap_or(u64::MAX));
        self.total = total;
        let due = self
            .last_sent
            .is_none_or(|sent| sent.elapsed() >= PROGRESS_INTERVAL);
        if due {
            self.last_sent = Some(Instant::now());
            self.report(progress);
        }
    }

    fn finish(&self, progress: &Channel<AppUpdateProgress>) {
        self.report(progress);
    }

    fn report(&self, progress: &Channel<AppUpdateProgress>) {
        send(
            progress,
            AppUpdateProgress::Downloading {
                downloaded: self.downloaded as f64,
                total: self.total.map(|total| total as f64),
            },
        );
    }
}

/// One `update_install` at a time. Every error return clears the flag; success keeps it set
/// (the app exits or, on Windows, the process ends inside the plugin).
struct UpdatingFlag<'a> {
    flag: &'a AtomicBool,
    keep: bool,
}

impl<'a> UpdatingFlag<'a> {
    fn claim(flag: &'a AtomicBool) -> Result<Self, AppError> {
        flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| AppError::UpdateBlocked {
                reason: UpdateBlockDto::from(UpdateBlock::Busy),
            })?;
        Ok(Self { flag, keep: false })
    }

    fn keep(mut self) {
        self.keep = true;
    }
}

impl Drop for UpdatingFlag<'_> {
    fn drop(&mut self) {
        if !self.keep {
            self.flag.store(false, Ordering::SeqCst);
        }
    }
}
