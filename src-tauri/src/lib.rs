//! Thin Tauri shell around `remmods-core`: window, plugins, commands and the `--smoke` run.

mod args;
mod base_index;
mod commands;
mod drop;
mod events;
mod focus;
mod install_sources;
mod monitor;
mod os;
mod smoke;
pub mod specta;
mod state;
mod watcher;
mod window;

use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::Ordering;
use std::sync::{Arc, PoisonError};
use std::time::Duration;

use log::LevelFilter;
use remmods_core::app_update::{self, InstallKind};
use remmods_core::game::GameOptions;
use remmods_core::graphics::GraphicsMode;
use remmods_core::platform::TrashEnv;
use remmods_core::{AppPaths, Core, CoreOptions, paths, platform, safe_fs};
use tauri::plugin::{Plugin, TauriPlugin};
use tauri::{App, AppHandle, Manager, RunEvent, Wry};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};
use tauri_plugin_window_state::StateFlags;

use crate::args::LaunchArgs;
use crate::smoke::SmokeFixture;
use crate::state::{AppState, Relaunch, SmokeState, UpdateSetup};

const LOG_FILE_NAME: &str = "remmods";
const LOG_MAX_BYTES: u128 = 2 * 1024 * 1024;
const LOG_FILES_KEPT: usize = 5;
const SAFE_GRAPHICS_ARG: &str = "--safe-graphics";
/// Read and removed by the type-2 AppImage runtime when it is the first argument: extract and run
/// instead of a FUSE mount.
const APPIMAGE_EXTRACT_AND_RUN_ARG: &str = "--appimage-extract-and-run";
/// How long the exit waits for an update that is rewriting the AppImage (about 100 MB).
const EXIT_INSTALL_WAIT: Duration = Duration::from_secs(60);
/// Restores size, position, maximized and fullscreen. Never VISIBLE (the window stays hidden
/// until `app_ready`) and never DECORATIONS, which would override the title bar setting with
/// the previous session's frame.
pub(crate) const WINDOW_STATE_FLAGS: StateFlags =
    StateFlags::all().difference(StateFlags::VISIBLE.union(StateFlags::DECORATIONS));

pub fn run() {
    let args = LaunchArgs::from_env();
    let smoke = args.smoke;
    let fixture = smoke_fixture(&args);
    let smoke_root = smoke.then(|| {
        safe_fs::create_smoke_root().unwrap_or_else(|error| {
            fail_fast("Could not create the smoke test folder", &error, smoke)
        })
    });
    let paths = match &smoke_root {
        Some(root) => AppPaths::under(root),
        None => AppPaths::for_user()
            .unwrap_or_else(|error| fail_fast("Could not find your user folders", &error, smoke)),
    };
    // Sets WebKit's environment, so it runs before the logger, any thread and GTK (A4).
    os::graphics::prepare(&paths, &args);
    let smoke_state = smoke_root.map(|root| SmokeState::new(root, fixture));
    if let Err(error) = safe_fs::ensure_dir(&paths.log_dir) {
        fail_fast("Could not open the RemMods data folder", &error, smoke);
    }
    let log_plugin = log_plugin(&paths.log_dir);
    let bindings = crate::specta::builder();
    let context = tauri::generate_context!();
    // Only the release config carries `plugins.updater`; without it the plugin's setup fails.
    let updater = !smoke && context.config().plugins.0.contains_key("updater");

    let mut builder = tauri::Builder::default();
    if !smoke {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            on_second_instance(app, &argv);
        }));
    }
    builder = builder.plugin(log_plugin);
    if !smoke {
        builder = builder.plugin(window_state_plugin());
    }
    // No dialog:* or fs:* permission is granted: the folder picker runs from Rust only.
    builder = builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(opener_plugin());
    // Rust-only updater: no JS package and no updater:* permission.
    if updater {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }
    let app = builder
        .invoke_handler(bindings.invoke_handler())
        .setup(move |app| {
            bindings.mount_events(app);
            on_setup(app, &args, paths, smoke_state, updater);
            Ok(())
        })
        .build(context)
        .unwrap_or_else(|error| fail_fast("Could not start RemMods", &error, smoke));
    app.run(on_run_event);
}

/// Runs after the plugins are set up, so the log plugin already records what core logs while it
/// opens (settings recovery, backup paths, read errors).
fn on_setup(
    app: &mut App,
    args: &LaunchArgs,
    paths: AppPaths,
    smoke: Option<SmokeState>,
    updater: bool,
) {
    log_launch(app, args);
    os::graphics::log_startup();
    let install_kind = commands::update::detect_install_kind();
    log::info!("install kind {install_kind:?}, updater configured {updater}");
    let update = UpdateSetup {
        install_kind,
        updater_configured: updater,
    };
    let core = Core::open_with(paths, core_options(args)).unwrap_or_else(|error| {
        fail_fast("Could not open the RemMods data folder", &error, args.smoke)
    });
    let core = Arc::new(core);
    core.ops
        .busy()
        .set_sink(events::folder_busy_sink(app.handle().clone()));
    let monitor = monitor::start(app.handle().clone(), Arc::clone(&core))
        .unwrap_or_else(|error| fail_fast("Could not start the game monitor", &error, args.smoke));
    let watcher = watcher::start(app.handle().clone(), Arc::clone(&core))
        .unwrap_or_else(|error| fail_fast("Could not start the Paks watcher", &error, args.smoke));
    app.manage(AppState::new(core, monitor, watcher, smoke, update));
    os::webview::ensure_runtime(args.smoke);

    let handle = app.handle();
    let state = app.state::<AppState>();
    let settings = state.core.settings();
    if settings.read_only || settings.recovered {
        log::warn!(
            "settings loaded with read_only={} recovered={}",
            settings.read_only,
            settings.recovered
        );
    }
    let webview_data_dir = state.smoke.as_ref().map(SmokeState::webview_data_dir);
    let appearance = settings.settings.appearance;
    if let Err(error) = window::create_main_window(handle, &appearance, webview_data_dir) {
        fail_fast("Could not create the main window", &error, args.smoke);
    }
    if let Err(error) = window::start_show_fallback(handle.clone()) {
        fail_fast("Could not start the window fallback", &error, args.smoke);
    }
    if state.smoke.is_some()
        && let Err(error) = smoke::start_watchdog(handle.clone())
    {
        fail_fast("Could not start the smoke watchdog", &error, args.smoke);
    }
}

fn log_launch(app: &App, args: &LaunchArgs) {
    let version = app.package_info().version.to_string();
    let mode = if args.smoke {
        " in smoke test mode"
    } else {
        ""
    };
    log::info!(
        "RemMods {version} starting on {:?}{mode}",
        platform::os_kind()
    );
    match (&args.paks, args.smoke) {
        (Some(paks), true) => log::info!("smoke fixture Paks: {}", paks.display()),
        (Some(paks), false) => log::warn!("--paks {} ignored without --smoke", paks.display()),
        (None, _) => {}
    }
    if !args.ignored.is_empty() {
        log::debug!("ignored arguments: {:?}", args.ignored);
    }
}

/// `--smoke --paks P` runs only against a generated fixture (`P/remmods-smoke-fixture`).
fn smoke_fixture(args: &LaunchArgs) -> Option<SmokeFixture> {
    let paks = args.paks.as_deref().filter(|_| args.smoke)?;
    let fixture = smoke::load_fixture(paks).unwrap_or_else(|error| {
        fail_fast(
            "--paks needs a folder marked remmods-smoke-fixture",
            &error,
            true,
        )
    });
    Some(fixture)
}

/// The smoke run never reads the machine's Steam roots: no detection, and the fixture (if any)
/// is the fixed game folder, and mutations only run inside a marked fixture. It also keeps the
/// default trash env, which refuses every trash.
fn core_options(args: &LaunchArgs) -> CoreOptions {
    if !args.smoke {
        return CoreOptions {
            trash: TrashEnv::from_env(),
            tmm_dir: paths::tmm_profiles_dir(),
            ..CoreOptions::default()
        };
    }
    CoreOptions {
        game: GameOptions {
            steam_detection: false,
            fixed_location: args.paks.clone(),
            steam_hints: None,
            processes: None,
        },
        require_marker: Some(smoke::FIXTURE_MARKER.to_owned()),
        ..CoreOptions::default()
    }
}

fn on_run_event(app: &AppHandle, event: RunEvent) {
    // While sealed, `CloseRequested` (window.rs) refuses the close. An exit accepted before the
    // seal waits here for the install; `ExitRequested(None)` only follows the last window's
    // `Destroyed`, so it is not refused.
    if let RunEvent::Exit = event {
        if let Some(state) = app.try_state::<AppState>() {
            if !state.close_for_exit(EXIT_INSTALL_WAIT) {
                log::error!("the update install did not finish within {EXIT_INSTALL_WAIT:?}");
            }
            state.monitor.stop();
            state.watcher.stop();
            if let Some(smoke_state) = state.smoke.as_ref() {
                smoke::on_exit(smoke_state);
            }
            os::graphics::on_exit();
            relaunch_on_exit(app, &state);
        }
        log::logger().flush();
    }
}

/// After an AppImage update or for a safe-graphics restart: releases the single-instance name
/// first so the new process does not exit as a second instance, then starts it with the host's
/// environment (no AppRun values, no RemMods graphics vars) and `$HOME` as its folder (A8).
fn relaunch_on_exit(app: &AppHandle, state: &AppState) {
    let relaunch = state
        .relaunch
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take();
    let Some(relaunch) = relaunch else {
        return;
    };
    tauri_plugin_single_instance::destroy(app);
    let mut command = platform::host_command(&relaunch.program);
    // An argument, not the variable: the runtime removes it, so it reaches no host program.
    if relaunch.extract_and_run {
        command.arg(APPIMAGE_EXTRACT_AND_RUN_ARG);
    }
    command.args(&relaunch.args);
    match platform::spawn_detached(command) {
        Ok(()) => log::info!(
            "relaunched {} with {:?}",
            relaunch.program.display(),
            relaunch.args
        ),
        Err(error) => log::error!("relaunching {} failed: {error}", relaunch.program.display()),
    }
}

/// A second start focuses the running window. `--safe-graphics` (the desktop file's action)
/// instead restarts this instance in software rendering, which must work even when its window
/// stays blank.
fn on_second_instance(app: &AppHandle, argv: &[String]) {
    if argv.iter().any(|arg| arg == SAFE_GRAPHICS_ARG) && restart_in_safe_graphics(app) {
        return;
    }
    log::info!("second instance started with {argv:?}; focusing the main window");
    window::focus_main_window(app);
}

fn restart_in_safe_graphics(app: &AppHandle) -> bool {
    if !os::graphics::supported() {
        return false;
    }
    let Some(state) = app.try_state::<AppState>() else {
        return false;
    };
    if state.updating.load(Ordering::SeqCst) {
        log::info!("--safe-graphics from a second instance ignored while an update runs");
        return false;
    }
    if os::graphics::session().is_some_and(|session| session.effective == GraphicsMode::Software) {
        log::info!("--safe-graphics from a second instance: already in software rendering");
        return false;
    }
    if state.install_kind == InstallKind::AppImage
        && app_update::appimage_extracted(std::env::var_os("APPDIR").as_deref())
    {
        // The same file extracts to the same folder, which this session's runtime deletes when
        // it exits, so a relaunch would run from a folder that is being removed.
        log::warn!(
            "--safe-graphics from a second instance ignored: this session runs from \
             --appimage-extract-and-run; restart it with --safe-graphics instead"
        );
        return false;
    }
    let Some(program) = relaunch_program(state.install_kind) else {
        log::warn!("--safe-graphics from a second instance: no program to relaunch");
        return false;
    };
    log::info!("restarting in safe graphics mode for a second instance's --safe-graphics");
    *state
        .relaunch
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some(Relaunch {
        program,
        args: vec![SAFE_GRAPHICS_ARG.into()],
        extract_and_run: false,
    });
    app.exit(0);
    true
}

/// An existing file only, or the restart would end this instance with nothing to start. The
/// binary path is the one cached before `main`: after a package upgrade replaced the running
/// binary, `std::env::current_exe()` reads `/proc/self/exe` as "<path> (deleted)".
fn relaunch_program(install_kind: InstallKind) -> Option<PathBuf> {
    let program = if install_kind == InstallKind::AppImage {
        std::env::var_os("APPIMAGE").map(PathBuf::from)
    } else {
        tauri::utils::platform::current_exe().ok()
    };
    program.filter(|program| program.is_file())
}

fn log_plugin(log_dir: &Path) -> TauriPlugin<Wry> {
    let mut targets = vec![Target::new(TargetKind::Folder {
        path: log_dir.to_path_buf(),
        file_name: Some(LOG_FILE_NAME.to_owned()),
    })];
    let level = if cfg!(debug_assertions) {
        targets.push(Target::new(TargetKind::Stdout));
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    tauri_plugin_log::Builder::new()
        .targets(targets)
        .max_file_size(LOG_MAX_BYTES)
        .rotation_strategy(RotationStrategy::KeepSome(LOG_FILES_KEPT))
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .level(level)
        .build()
}

fn window_state_plugin() -> TauriPlugin<Wry> {
    tauri_plugin_window_state::Builder::new()
        .with_state_flags(WINDOW_STATE_FLAGS)
        .build()
}

/// Rust-side opening only (`open_logs`). The injected link-click script would call a JS
/// command this app's capability does not grant.
fn opener_plugin() -> impl Plugin<Wry> {
    tauri_plugin_opener::Builder::new()
        .open_js_links_on_click(false)
        .build()
}

/// Startup failure before or while the window is created. `--smoke` never shows a dialog: on a
/// CI runner nobody could close it.
pub(crate) fn fail_fast(what: &str, detail: &dyn Display, smoke: bool) -> ! {
    log::error!("{what}: {detail}");
    log::logger().flush();
    if smoke {
        eprintln!("{what}: {detail}");
    } else {
        platform::fatal_message("RemMods", &format!("{what}.\n\n{detail}"));
    }
    process::exit(1)
}
