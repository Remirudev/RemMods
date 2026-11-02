#![allow(clippy::expect_used)]

mod game_support;
mod support;

use game_support::{build_proc_tree, proc_fixtures};
use remmods_core::game::{GameSignal, classify};
use remmods_core::platform::read_proc_tree;

fn signal_code(signal: GameSignal) -> &'static str {
    match signal {
        GameSignal::SteamReaper => "steamReaper",
        GameSignal::GameExecutable => "gameExecutable",
    }
}

#[test]
fn every_fixture_classifies_as_expected() {
    let fixtures = proc_fixtures();
    assert!(fixtures.len() >= 10, "proc fixtures missing");
    for fixture in fixtures {
        let (_dir, root) = support::temp_root();
        build_proc_tree(&root, &fixture.processes);
        let records = read_proc_tree(&root).expect("read fake proc tree");
        let verdict = classify(&records);
        let name = &fixture.name;
        assert_eq!(verdict.game.is_some(), fixture.running, "{name}: running");
        if let Some(signal) = &fixture.signal {
            let found = verdict.game.expect("a match");
            assert_eq!(signal_code(found.signal), signal, "{name}: signal");
        }
        if let Some(pid) = fixture.pid {
            assert_eq!(
                verdict.game.map(|found| found.pid),
                Some(pid),
                "{name}: pid"
            );
        }
        if let Some(other_manager) = fixture.other_manager {
            assert_eq!(verdict.other_manager, other_manager, "{name}: otherManager");
        }
    }
}

#[test]
fn fake_tree_records_are_sorted_and_skip_vanished_processes() {
    let fixture = proc_fixtures()
        .into_iter()
        .find(|fixture| fixture.name == "edge-cases.json")
        .expect("edge-cases fixture");
    let (_dir, root) = support::temp_root();
    build_proc_tree(&root, &fixture.processes);
    let records = read_proc_tree(&root).expect("read fake proc tree");
    let pids: Vec<u32> = records.iter().map(|record| record.pid).collect();
    assert_eq!(pids, [701, 702]);
    assert_eq!(records[1].argv, ["C:/Games\\TEKKEN 8/TEKKEN 8.EXE"]);
    assert!(records[0].argv[0].contains('\u{FFFD}'));
}
