//! Thin Tauri shell around `remmods-core`: window, plugins, commands and the `--smoke` run.

mod args;
mod commands;
mod os;
mod smoke;
pub mod specta;
mod state;
mod window;

use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::process;

use log::LevelFilter;
use remmods_core::{AppPaths, Core, platform, safe_fs};
use tauri::plugin::{Plugin, TauriPlugin};
use tauri::{App, AppHandle, Manager, RunEvent, Wry};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};
use tauri_plugin_window_state::StateFlags;

use crate::args::LaunchArgs;
use crate::state::{AppState, SmokeState};

const LOG_FILE_NAME: &str = "remmods";
const LOG_MAX_BYTES: u128 = 2 * 1024 * 1024;
const LOG_FILES_KEPT: usize = 5;

pub fn run() {
    let args = LaunchArgs::parse(std::env::args_os().skip(1));
    let smoke = args.smoke;
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
    if let Err(error) = safe_fs::ensure_dir(&paths.log_dir) {
        fail_fast("Could not open the RemMods data folder", &error, smoke);
    }
    let log_plugin = log_plugin(&paths.log_dir);
    let bindings = crate::specta::builder();

    let mut builder = tauri::Builder::default();
    if !smoke {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            log::info!("second instance started with {argv:?}; focusing the main window");
            window::focus_main_window(app);
        }));
    }
    builder = builder.plugin(log_plugin);
    if !smoke {
        builder = builder.plugin(window_state_plugin());
    }
    let app = builder
        .plugin(opener_plugin())
        .invoke_handler(bindings.invoke_handler())
        .setup(move |app| {
            bindings.mount_events(app);
            on_setup(app, &args, paths, smoke_root);
            Ok(())
        })
        .build(tauri::generate_context!())
        .unwrap_or_else(|error| fail_fast("Could not start RemMods", &error, smoke));
    app.run(on_run_event);
}

/// Runs after the plugins are set up, so the log plugin already records what core logs while it
/// opens (settings recovery, backup paths, read errors).
fn on_setup(app: &mut App, args: &LaunchArgs, paths: AppPaths, smoke_root: Option<PathBuf>) {
    log_launch(app, args);
    let core = Core::open(paths).unwrap_or_else(|error| {
        fail_fast("Could not open the RemMods data folder", &error, args.smoke)
    });
    app.manage(AppState::new(core, smoke_root));
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
    if let Some(paks) = &args.paks {
        log::info!("--paks {} accepted (unused until Phase 2)", paks.display());
    }
    if !args.ignored.is_empty() {
        log::debug!("ignored arguments: {:?}", args.ignored);
    }
}

fn on_run_event(app: &AppHandle, event: RunEvent) {
    if let RunEvent::Exit = event {
        if let Some(state) = app.try_state::<AppState>()
            && let Some(smoke_state) = state.smoke.as_ref()
        {
            smoke::on_exit(smoke_state);
        }
        log::logger().flush();
    }
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

/// Restores size, position, maximized and fullscreen. Never VISIBLE (the window stays hidden
/// until `app_ready`) and never DECORATIONS, which would override the title bar setting with
/// the previous session's frame.
fn window_state_plugin() -> TauriPlugin<Wry> {
    let skipped = StateFlags::VISIBLE | StateFlags::DECORATIONS;
    tauri_plugin_window_state::Builder::new()
        .with_state_flags(StateFlags::all().difference(skipped))
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
