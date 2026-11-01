use std::time::{Duration, Instant};

use remmods_core::game::{GameMatch, GameSignal, ProcessVerdict, RunningState, RunningTracker};

const POSITIVE: Result<ProcessVerdict, ()> = Ok(ProcessVerdict {
    game: Some(GameMatch {
        pid: 1,
        signal: GameSignal::SteamReaper,
    }),
    original_app: false,
});
const NEGATIVE: Result<ProcessVerdict, ()> = Ok(ProcessVerdict {
    game: None,
    original_app: false,
});

fn at(t0: Instant, millis: u64) -> Instant {
    t0 + Duration::from_millis(millis)
}

#[test]
fn starts_unknown_and_first_negative_is_not_running() {
    let t0 = Instant::now();
    let mut tracker = RunningTracker::new();
    assert_eq!(
        (tracker.state(), tracker.session()),
        (RunningState::Unknown, 0)
    );
    assert!(tracker.observe(NEGATIVE, t0));
    assert_eq!(tracker.state(), RunningState::NotRunning);
    assert!(tracker.observe(POSITIVE, at(t0, 100)));
    assert_eq!(
        (tracker.state(), tracker.session()),
        (RunningState::Running, 1)
    );
}

#[test]
fn short_gap_keeps_running() {
    let t0 = Instant::now();
    let mut tracker = RunningTracker::new();
    tracker.observe(POSITIVE, t0);
    assert!(!tracker.observe(NEGATIVE, at(t0, 1_000)));
    assert!(!tracker.observe(NEGATIVE, at(t0, 2_000)));
    assert!(!tracker.observe(POSITIVE, at(t0, 3_000)));
    assert_eq!(
        (tracker.state(), tracker.session()),
        (RunningState::Running, 1)
    );
}

#[test]
fn not_running_after_three_seconds_of_negatives() {
    let t0 = Instant::now();
    let mut tracker = RunningTracker::new();
    tracker.observe(POSITIVE, t0);
    for second in 1..=3 {
        assert!(!tracker.observe(NEGATIVE, at(t0, second * 1_000)));
        assert_eq!(tracker.state(), RunningState::Running);
    }
    assert!(tracker.observe(NEGATIVE, at(t0, 4_000)));
    assert_eq!(tracker.state(), RunningState::NotRunning);
}

#[test]
fn two_scans_spanning_three_seconds_suffice() {
    let t0 = Instant::now();
    let mut tracker = RunningTracker::new();
    tracker.observe(POSITIVE, t0);
    assert!(!tracker.observe(NEGATIVE, at(t0, 500)));
    assert!(tracker.observe(NEGATIVE, at(t0, 5_000)));
    assert_eq!(tracker.state(), RunningState::NotRunning);
}

#[test]
fn a_new_run_is_a_new_session() {
    let t0 = Instant::now();
    let mut tracker = RunningTracker::new();
    tracker.observe(POSITIVE, t0);
    for second in 1..=4 {
        tracker.observe(NEGATIVE, at(t0, second * 1_000));
    }
    assert_eq!(tracker.state(), RunningState::NotRunning);
    assert!(tracker.observe(POSITIVE, at(t0, 5_000)));
    assert_eq!(
        (tracker.state(), tracker.session()),
        (RunningState::Running, 2)
    );
}

#[test]
fn scan_error_is_unknown_and_recovery_starts_a_session() {
    let t0 = Instant::now();
    let mut tracker = RunningTracker::new();
    tracker.observe(POSITIVE, t0);
    assert!(tracker.observe(Err(()), at(t0, 1_000)));
    assert_eq!(tracker.state(), RunningState::Unknown);
    assert!(tracker.observe(POSITIVE, at(t0, 2_000)));
    assert_eq!(
        (tracker.state(), tracker.session()),
        (RunningState::Running, 2)
    );
}

#[test]
fn original_app_flip_alone_is_a_change() {
    let t0 = Instant::now();
    let mut tracker = RunningTracker::new();
    tracker.observe(NEGATIVE, t0);
    let with_original = Ok(ProcessVerdict {
        game: None,
        original_app: true,
    });
    assert!(tracker.observe(with_original, at(t0, 1_000)));
    assert!(tracker.original_app());
    assert!(!tracker.observe(with_original, at(t0, 2_000)));
    assert!(tracker.observe(NEGATIVE, at(t0, 3_000)));
    assert!(!tracker.original_app());
}
