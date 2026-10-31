//! `--smoke`: the real UI runs its checks and reports through `smoke_report`; a watchdog fails
//! the run if no report arrives (e.g. the page navigated away or never loaded).

use std::io::Write;
use std::process;
use std::thread;
use std::time::Duration;

use remmods_core::safe_fs;
use remmods_ipc::{SmokeCheck, SmokeReport};
use tauri::{AppHandle, Manager};

use crate::state::{AppState, SmokeState};

const WATCHDOG_TIMEOUT: Duration = Duration::from_secs(30);
/// Hard exit if the event loop does not finish the requested exit in time.
const EXIT_GRACE: Duration = Duration::from_secs(5);
const EXIT_PASSED: i32 = 0;
const EXIT_FAILED: i32 = 1;
const EXIT_TIMEOUT: i32 = 2;

pub fn finish(app: &AppHandle, smoke: &SmokeState, report: SmokeReport) {
    let report = with_navigation_check(report, smoke);
    let passed = report.passed();
    if !smoke.finish(passed) {
        return;
    }
    for check in &report.checks {
        let status = if check.ok { "ok" } else { "FAILED" };
        let detail = check.detail.as_deref().unwrap_or("");
        log::info!("smoke check {}: {status} {detail}", check.name);
    }
    print_report_line(&SmokeReport {
        ok: passed,
        checks: report.checks,
    });
    exit_soon(app, if passed { EXIT_PASSED } else { EXIT_FAILED });
}

/// Called by the navigation filter for every navigation it refuses.
pub fn record_blocked_navigation(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>()
        && let Some(smoke) = state.smoke.as_ref()
    {
        smoke.record_blocked_navigation();
    }
}

/// The page's own check only sees that it is still there a second later, which a slow network
/// would fake; this one needs the filter to have refused the page's `example.com` attempt.
fn with_navigation_check(mut report: SmokeReport, smoke: &SmokeState) -> SmokeReport {
    let blocked = smoke.blocked_navigations();
    report.checks.push(SmokeCheck {
        name: "navigationBlocked".into(),
        ok: blocked > 0,
        detail: Some(format!("{blocked} blocked by the navigation filter")),
    });
    report
}

pub fn start_watchdog(app: AppHandle) -> std::io::Result<()> {
    thread::Builder::new()
        .name("smoke-watchdog".into())
        .spawn(move || {
            thread::sleep(WATCHDOG_TIMEOUT);
            let state = app.state::<AppState>();
            let Some(smoke) = state.smoke.as_ref() else {
                return;
            };
            if !smoke.finish(false) {
                return;
            }
            log::error!("no smoke report within {WATCHDOG_TIMEOUT:?}");
            print_report_line(&SmokeReport {
                ok: false,
                checks: vec![SmokeCheck {
                    name: "timeout".into(),
                    ok: false,
                    detail: Some(format!("no smoke report within {WATCHDOG_TIMEOUT:?}")),
                }],
            });
            exit_soon(&app, EXIT_TIMEOUT);
        })
        .map(drop)
}

/// Runs on `RunEvent::Exit`. An exit before any report (window closed, webview gone) must not
/// look like a pass, so it prints a failed report and exits 1 itself.
pub fn on_exit(smoke: &SmokeState) {
    let unreported = smoke.finish(false);
    if unreported {
        log::error!("the app exited before the smoke report arrived");
        print_report_line(&SmokeReport {
            ok: false,
            checks: vec![SmokeCheck {
                name: "exit".into(),
                ok: false,
                detail: Some("app exited before the smoke report".into()),
            }],
        });
    }
    clean_up(smoke);
    if unreported {
        log::logger().flush();
        process::exit(EXIT_FAILED);
    }
}

/// Removes the temp root after a passed run; a failed run keeps it so CI can upload the logs.
fn clean_up(smoke: &SmokeState) {
    if !smoke.passed() {
        let root = smoke.root.display();
        log::warn!("smoke failed; keeping {root} for inspection");
        eprintln!("REMMODS_SMOKE_ROOT {root}");
        return;
    }
    // On Windows the WebView2 browser process can still hold the webview folder at exit.
    if let Err(error) = safe_fs::remove_smoke_root(&smoke.root) {
        log::warn!("smoke temp root not fully removed: {error}");
        eprintln!("REMMODS_SMOKE_ROOT {}", smoke.root.display());
    }
}

fn print_report_line(report: &SmokeReport) {
    let json = serde_json::to_string(report).unwrap_or_else(|error| {
        log::error!("serializing the smoke report failed: {error}");
        r#"{"ok":false,"checks":[]}"#.to_owned()
    });
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "REMMODS_SMOKE {json}");
    let _ = stdout.flush();
}

fn exit_soon(app: &AppHandle, code: i32) {
    app.exit(code);
    let app = app.clone();
    let spawned = thread::Builder::new()
        .name("smoke-exit".into())
        .spawn(move || {
            thread::sleep(EXIT_GRACE);
            log::error!("event loop did not exit within {EXIT_GRACE:?}; exiting hard");
            if let Some(smoke) = app.state::<AppState>().smoke.as_ref() {
                clean_up(smoke);
            }
            log::logger().flush();
            process::exit(code);
        });
    if let Err(error) = spawned {
        log::error!("starting the exit guard failed: {error}");
    }
}
