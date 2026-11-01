//! `--smoke`: the real UI runs its checks and reports through `smoke_report`; a watchdog fails
//! the run if no report arrives (e.g. the page navigated away or never loaded). With
//! `--paks <fixture Paks>` the last scan is also compared with the fixture's marker file.

use std::collections::BTreeSet;
use std::fmt::Debug;
use std::io::Write;
use std::path::Path;
use std::process;
use std::thread;
use std::time::Duration;

use remmods_core::game::GameLocation;
use remmods_core::{Core, platform, safe_fs};
use remmods_ipc::{ModsSnapshot, SmokeCheck, SmokeReport};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::state::{AppState, SmokeState};

const WATCHDOG_TIMEOUT: Duration = Duration::from_secs(30);
/// Hard exit if the event loop does not finish the requested exit in time.
const EXIT_GRACE: Duration = Duration::from_secs(5);
const EXIT_PASSED: i32 = 0;
const EXIT_FAILED: i32 = 1;
const EXIT_TIMEOUT: i32 = 2;

/// Written into the fixture's Paks folder by `scripts/make-fixtures.py`.
pub const FIXTURE_MARKER: &str = "remmods-smoke-fixture";
const FIXTURE_NAME: &str = "remmods-smoke";
const FIXTURE_VERSION: u32 = 1;
const FIXTURE_MAX_BYTES: u64 = 64 * 1024;
/// Differences listed in the `scanFixture` detail; the rest are only counted.
const SHOWN_DIFFERENCES: usize = 5;

/// The marker's expectations: rows, archives, Unmanaged entries, leftovers and the build id.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmokeFixture {
    version: u32,
    fixture: String,
    rows: Vec<FixtureRow>,
    archives: Vec<String>,
    unmanaged: Vec<FixtureUnmanaged>,
    leftovers: Vec<String>,
    build_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureRow {
    key: String,
    state: String,
    conflicts: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureUnmanaged {
    rel_path: String,
    kind: String,
}

/// Reads `<paks>/remmods-smoke-fixture`, which must be a regular file holding a version 1 marker:
/// `--smoke --paks` only ever runs against a generated fixture, never a real game folder.
pub fn load_fixture(paks: &Path) -> Result<SmokeFixture, String> {
    let marker = platform::fs_path(&paks.join(FIXTURE_MARKER))
        .map_err(|error| format!("{}: {error}", paks.display()))?;
    let failed = |error: &dyn std::fmt::Display| format!("{}: {error}", marker.display());
    let metadata = std::fs::symlink_metadata(&marker).map_err(|error| failed(&error))?;
    if !metadata.is_file() {
        return Err(failed(&"not a regular file"));
    }
    if metadata.len() > FIXTURE_MAX_BYTES {
        return Err(failed(&"larger than a marker"));
    }
    let bytes = std::fs::read(&marker).map_err(|error| failed(&error))?;
    let fixture: SmokeFixture = serde_json::from_slice(&bytes).map_err(|error| failed(&error))?;
    if fixture.version != FIXTURE_VERSION || fixture.fixture != FIXTURE_NAME {
        return Err(failed(&format!(
            "not a version {FIXTURE_VERSION} \"{FIXTURE_NAME}\" marker"
        )));
    }
    Ok(fixture)
}

pub fn finish(app: &AppHandle, core: &Core, smoke: &SmokeState, report: SmokeReport) {
    let mut report = with_navigation_check(report, smoke);
    if let Some(fixture) = &smoke.fixture {
        report.checks.push(scan_fixture_check(core, fixture));
    }
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

/// The frontend opened the mods table, so a snapshot must exist and match the marker as sets.
fn scan_fixture_check(core: &Core, fixture: &SmokeFixture) -> SmokeCheck {
    let differences = match core.mods_last() {
        Some(snapshot) => {
            let build_id = installed_build(core);
            fixture_differences(
                fixture,
                &ModsSnapshot::from(&*snapshot),
                build_id.as_deref(),
            )
        }
        None => vec!["no scan ran".to_owned()],
    };
    let detail = if differences.is_empty() {
        "scan matches the fixture marker".to_owned()
    } else {
        summarize(&differences)
    };
    SmokeCheck {
        name: "scanFixture".into(),
        ok: differences.is_empty(),
        detail: Some(detail),
    }
}

fn installed_build(core: &Core) -> Option<String> {
    match core.game_status().location {
        GameLocation::Ready(install) => install.build_id,
        GameLocation::NotSet | GameLocation::Missing { .. } => None,
    }
}

fn fixture_differences(
    fixture: &SmokeFixture,
    snapshot: &ModsSnapshot,
    build_id: Option<&str>,
) -> Vec<String> {
    let mut differences = Vec::new();
    let expected_rows = fixture
        .rows
        .iter()
        .map(|row| (row.key.clone(), row.state.clone(), row.conflicts));
    let actual_rows = snapshot.rows.iter().map(|row| {
        let state = wire_name(&row.state);
        (row.key.clone(), state, row.conflicts_with.len())
    });
    compare_sets("row", expected_rows, actual_rows, &mut differences);
    compare_sets(
        "archive",
        fixture.archives.iter().cloned(),
        snapshot.archives.iter().map(|entry| entry.rel_path.clone()),
        &mut differences,
    );
    compare_sets(
        "unmanaged",
        fixture
            .unmanaged
            .iter()
            .map(|entry| (entry.rel_path.clone(), entry.kind.clone())),
        snapshot
            .unmanaged
            .iter()
            .map(|entry| (entry.rel_path.clone(), wire_name(&entry.kind))),
        &mut differences,
    );
    compare_sets(
        "leftover",
        fixture.leftovers.iter().cloned(),
        snapshot
            .leftovers
            .iter()
            .map(|entry| entry.rel_path.clone()),
        &mut differences,
    );
    if build_id != Some(fixture.build_id.as_str()) {
        differences.push(format!(
            "build id {build_id:?} instead of {:?}",
            fixture.build_id
        ));
    }
    differences
}

fn compare_sets<T: Ord + Debug>(
    what: &str,
    expected: impl Iterator<Item = T>,
    actual: impl Iterator<Item = T>,
    differences: &mut Vec<String>,
) {
    let expected: BTreeSet<T> = expected.collect();
    let actual: BTreeSet<T> = actual.collect();
    for missing in expected.difference(&actual) {
        differences.push(format!("missing {what} {missing:?}"));
    }
    for unexpected in actual.difference(&expected) {
        differences.push(format!("unexpected {what} {unexpected:?}"));
    }
}

/// The IPC spelling of a unit enum (`noPakFiles`, `paksFile`), as the marker writes it.
fn wire_name<T: Serialize + Debug>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(name)) => name,
        _ => format!("{value:?}"),
    }
}

fn summarize(differences: &[String]) -> String {
    let shown = differences
        .iter()
        .take(SHOWN_DIFFERENCES)
        .cloned()
        .collect::<Vec<_>>()
        .join("; ");
    match differences.len().saturating_sub(SHOWN_DIFFERENCES) {
        0 => shown,
        more => format!("{shown}; and {more} more"),
    }
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
