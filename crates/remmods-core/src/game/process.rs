//! Decides from process records whether TEKKEN 8 (or the original mod manager) runs. Pure, so
//! fake `/proc` fixtures drive it on every OS. Exact rules only: never a substring of the whole
//! command line, never the `GameThread` name, never `/proc/<pid>/exe` (the Wine preloader).

use super::STEAM_APP_ID;
use crate::platform::ProcessRecord;

pub const GAME_EXECUTABLES: [&str; 2] = ["TEKKEN 8.exe", "Polaris-Win64-Shipping.exe"];
pub const ORIGINAL_APP_EXECUTABLE: &str = "ModManager.exe";
/// TASK_COMM_LEN 16 minus the NUL: Linux shows `Polaris-Win64-Shipping.exe` as `Polaris-Win64-S`.
pub const COMM_MAX_BYTES: usize = 15;

const WINE_LOADERS: [&str; 4] = ["wine", "wine64", "wine-preloader", "wine64-preloader"];
const REAPER: &str = "reaper";
const STEAM_LAUNCH: &str = "SteamLaunch";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameSignal {
    /// Steam's `reaper SteamLaunch AppId=1778820 [Install=1] -- …`, alive for the whole session.
    SteamReaper,
    /// argv[0] or the process name is one of the game's executables.
    GameExecutable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameMatch {
    pub pid: u32,
    pub signal: GameSignal,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProcessVerdict {
    pub game: Option<GameMatch>,
    pub original_app: bool,
}

/// A reaper match is preferred, else the first match by pid.
pub fn classify(records: &[ProcessRecord]) -> ProcessVerdict {
    let matches = game_matches(records);
    let game = matches
        .iter()
        .find(|found| found.signal == GameSignal::SteamReaper)
        .or_else(|| matches.first())
        .copied();
    let original_app = records
        .iter()
        .any(|record| names_executable(record, ORIGINAL_APP_EXECUTABLE));
    ProcessVerdict { game, original_app }
}

/// Every record that shows the game, ordered by pid.
pub fn game_matches(records: &[ProcessRecord]) -> Vec<GameMatch> {
    let mut matches: Vec<GameMatch> = records
        .iter()
        .filter_map(|record| {
            let signal = if is_steam_reaper(record) {
                GameSignal::SteamReaper
            } else if is_game_executable(record) {
                GameSignal::GameExecutable
            } else {
                return None;
            };
            Some(GameMatch {
                pid: record.pid,
                signal,
            })
        })
        .collect();
    matches.sort_by_key(|found| found.pid);
    matches
}

/// `reaper` with `SteamLaunch` and `AppId=1778820` among its own arguments (before the first
/// `--`; what follows belongs to the launched command).
fn is_steam_reaper(record: &ProcessRecord) -> bool {
    let is_reaper = record.name == REAPER
        || record
            .argv
            .first()
            .is_some_and(|arg0| file_name(arg0) == REAPER);
    if !is_reaper {
        return false;
    }
    let app_id = format!("AppId={STEAM_APP_ID}");
    let own_args = || record.argv.iter().skip(1).take_while(|arg| *arg != "--");
    own_args().any(|arg| arg == STEAM_LAUNCH) && own_args().any(|arg| *arg == app_id)
}

fn is_game_executable(record: &ProcessRecord) -> bool {
    GAME_EXECUTABLES
        .iter()
        .any(|exe| names_executable(record, exe))
}

fn names_executable(record: &ProcessRecord, exe: &str) -> bool {
    let is_exe = |arg: &String| file_name(arg).eq_ignore_ascii_case(exe);
    let by_argv0 = record.argv.first().is_some_and(is_exe);
    // Wine shifts its loader out of argv; should it ever stay, the exe follows the loader.
    let by_loader = record
        .argv
        .first()
        .is_some_and(|arg0| WINE_LOADERS.contains(&file_name(arg0)))
        && record.argv.iter().skip(1).take(2).any(is_exe);
    by_argv0 || by_loader || name_is(&record.name, exe)
}

/// The full name, or the kernel's 15-byte cut of a longer one.
fn name_is(name: &str, exe: &str) -> bool {
    name.eq_ignore_ascii_case(exe)
        || (exe.len() > COMM_MAX_BYTES
            && name.len() == COMM_MAX_BYTES
            && exe.as_bytes()[..COMM_MAX_BYTES].eq_ignore_ascii_case(name.as_bytes()))
}

/// Text after the last `/` or `\`: Wine passes Windows paths, so both count on every OS.
fn file_name(arg: &str) -> &str {
    arg.rsplit(['/', '\\']).next().unwrap_or(arg)
}
