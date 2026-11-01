use proptest::prelude::*;
use remmods_core::game::{GameSignal, classify, game_matches};
use remmods_core::platform::{ProcessRecord, parse_cmdline, parse_comm};

fn record(pid: u32, name: &str, argv: &[&str]) -> ProcessRecord {
    ProcessRecord {
        pid,
        name: name.to_owned(),
        argv: argv.iter().map(|arg| (*arg).to_owned()).collect(),
    }
}

const REAPER: &str = "/home/user/.local/share/Steam/ubuntu12_32/reaper";

#[test]
fn windows_names_without_argv() {
    for exe in ["TEKKEN 8.exe", "Polaris-Win64-Shipping.exe", "tekken 8.EXE"] {
        let verdict = classify(&[record(10, exe, &[])]);
        assert_eq!(
            verdict.game.map(|found| (found.pid, found.signal)),
            Some((10, GameSignal::GameExecutable)),
            "{exe}"
        );
    }
    let original = classify(&[record(11, "ModManager.exe", &[])]);
    assert!(original.game.is_none());
    assert!(original.original_app);
}

#[test]
fn reaper_is_preferred_over_an_earlier_exe() {
    let records = [
        record(5, "TEKKEN 8.exe", &[]),
        record(
            9,
            "reaper",
            &[REAPER, "SteamLaunch", "AppId=1778820", "--", "x"],
        ),
    ];
    let found = classify(&records).game.expect("match");
    assert_eq!((found.pid, found.signal), (9, GameSignal::SteamReaper));
    assert_eq!(game_matches(&records).len(), 2);
}

#[test]
fn shipping_exe_truncated_name_counts_but_others_do_not() {
    assert!(
        classify(&[record(1, "Polaris-Win64-S", &[])])
            .game
            .is_some()
    );
    assert!(
        classify(&[record(1, "polaris-win64-s", &[])])
            .game
            .is_some()
    );
    assert!(
        classify(&[record(1, "Polaris-Win64-X", &[])])
            .game
            .is_none()
    );
    assert!(classify(&[record(1, "TEKKEN 8.ex", &[])]).game.is_none());
    assert!(classify(&[record(1, "GameThread", &[])]).game.is_none());
}

#[test]
fn parse_cmdline_rules() {
    assert_eq!(parse_cmdline(b"a\0b\0\0\0"), ["a", "b"]);
    assert_eq!(parse_cmdline(b"a\0\0b\0"), ["a", "", "b"]);
    assert!(parse_cmdline(b"\0\0\0").is_empty());
    assert!(parse_cmdline(b"").is_empty());
    assert_eq!(parse_cmdline(b"no-nul"), ["no-nul"]);
}

#[test]
fn parse_comm_strips_one_newline() {
    assert_eq!(parse_comm(b"reaper\n"), "reaper");
    assert_eq!(parse_comm(b"x\n\n"), "x\n");
    assert_eq!(parse_comm(b"plain"), "plain");
}

fn arg() -> impl Strategy<Value = String> {
    "[A-Za-z0-9=/:._ \\\\-]{0,20}"
}

proptest! {
    #![proptest_config(ProptestConfig { failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn arguments_after_the_separator_never_change_the_reaper_verdict(
        own in proptest::collection::vec(prop_oneof![
            Just("SteamLaunch".to_owned()),
            Just("AppId=1778820".to_owned()),
            Just("Install=1".to_owned()),
            arg(),
        ], 0..5),
        tail_a in proptest::collection::vec(arg(), 0..6),
        tail_b in proptest::collection::vec(prop_oneof![
            Just("SteamLaunch".to_owned()),
            Just("AppId=1778820".to_owned()),
            arg(),
        ], 0..6),
    ) {
        let own: Vec<String> = own.into_iter().filter(|text| text != "--").collect();
        let build = |tail: &[String]| {
            let mut argv = vec![REAPER.to_owned()];
            argv.extend(own.iter().cloned());
            argv.push("--".to_owned());
            argv.extend(tail.iter().cloned());
            ProcessRecord { pid: 1, name: "reaper".to_owned(), argv }
        };
        let left = classify(&[build(&tail_a)]).game.map(|found| found.signal);
        let right = classify(&[build(&tail_b)]).game.map(|found| found.signal);
        prop_assert_eq!(left, right);
    }

    #[test]
    fn argv_without_a_game_basename_never_matches(
        name in "[a-z]{1,15}",
        argv in proptest::collection::vec(arg(), 0..6),
    ) {
        let mentions_game = argv.iter().any(|arg| {
            let base = arg.rsplit(['/', '\\']).next().unwrap_or(arg).to_ascii_lowercase();
            base == "tekken 8.exe" || base == "polaris-win64-shipping.exe" || base == "reaper"
        });
        prop_assume!(!mentions_game && name != "reaper");
        let records = [ProcessRecord { pid: 1, name, argv }];
        prop_assert!(classify(&records).game.is_none());
    }
}
