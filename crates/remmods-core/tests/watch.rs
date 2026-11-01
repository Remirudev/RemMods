#![allow(clippy::disallowed_methods)]

mod support;

use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use remmods_core::safe_fs::notify_types::event::{
    AccessKind, CreateKind, Flag, ModifyKind, RemoveKind, RenameMode,
};
use remmods_core::safe_fs::notify_types::{Error, ErrorKind, Event, EventKind};
use remmods_core::safe_fs::{Debounce, PaksWatcher, WatchError, WatchSignal, signal_for};

const TEMP_DIR: &str = "_tmp_0123456789abcdef0123456789abcdef";

fn event(kind: EventKind, paths: &[&str]) -> Result<Event, Error> {
    let event = paths.iter().fold(Event::new(kind), |event, path| {
        event.add_path(PathBuf::from(path))
    });
    Ok(event)
}

#[test]
fn one_burst_is_due_quiet_after_its_last_event() {
    let start = Instant::now();
    let mut debounce = Debounce::standard();
    debounce.push(start);
    debounce.push(start + Duration::from_millis(100));

    let due = start + Duration::from_millis(500);

    assert_eq!(debounce.deadline(), Some(due));
    assert!(!debounce.take_if_due(due - Duration::from_millis(1)));
    assert!(debounce.take_if_due(due));
    assert_eq!(debounce.deadline(), None);
    assert!(!debounce.take_if_due(due + Duration::from_secs(10)));
}

#[test]
fn a_steady_stream_fires_max_after_its_first_event() {
    let start = Instant::now();
    let mut debounce = Debounce::standard();
    let mut fired_at = None;
    for step in 0..40 {
        let now = start + Duration::from_millis(100 * step);
        if debounce.take_if_due(now) {
            fired_at = Some(now);
            break;
        }
        debounce.push(now);
    }

    assert_eq!(fired_at, Some(start + Debounce::MAX));
}

#[test]
fn an_idle_debounce_has_no_deadline() {
    let mut debounce = Debounce::new(Duration::from_millis(10), Duration::from_millis(50));

    assert_eq!(debounce.deadline(), None);
    assert!(!debounce.take_if_due(Instant::now()));
}

#[test]
fn access_events_are_ignored() {
    let open = event(EventKind::Access(AccessKind::Any), &["/p/Mods/a.pak"]);

    assert_eq!(signal_for(&open), None);
}

#[test]
fn events_only_under_install_temp_folders_are_ignored() {
    let temp = format!("/p/Mods/{TEMP_DIR}/a.pak");
    let mixed = event(
        EventKind::Create(CreateKind::File),
        &[temp.as_str(), "/p/Mods/b.pak"],
    );

    assert_eq!(
        signal_for(&event(EventKind::Create(CreateKind::File), &[&temp])),
        None
    );
    assert_eq!(signal_for(&mixed), Some(WatchSignal::Changed));
}

#[test]
fn a_rescan_flag_asks_for_a_rescan() {
    let flagged = Event::new(EventKind::Other).set_flag(Flag::Rescan);

    assert_eq!(signal_for(&Ok(flagged)), Some(WatchSignal::Rescan));
}

#[test]
fn errors_map_to_limit_reached_or_rescan() {
    let limit: Result<Event, Error> = Err(Error::new(ErrorKind::MaxFilesWatch));
    let other: Result<Event, Error> = Err(Error::generic("boom"));

    assert_eq!(signal_for(&limit), Some(WatchSignal::LimitReached));
    assert_eq!(signal_for(&other), Some(WatchSignal::Rescan));
}

#[test]
fn creates_removes_and_renames_are_changes() {
    let kinds = [
        EventKind::Create(CreateKind::Folder),
        EventKind::Remove(RemoveKind::File),
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
    ];
    for kind in kinds {
        assert_eq!(
            signal_for(&event(kind, &["/p/Mods/a.pak"])),
            Some(WatchSignal::Changed)
        );
    }
}

#[test]
fn a_real_watcher_reports_a_new_file() {
    let fixture = support::fs_fixture();
    let (sender, receiver) = mpsc::channel();
    let _watcher = PaksWatcher::start(&fixture.paks, move |signal| {
        let _ = sender.send(signal);
    })
    .expect("start");

    fs::write(fixture.path("Mods/new.pak"), b"pak").expect("write");

    let signal = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("a signal");
    assert_eq!(signal, WatchSignal::Changed);
}

#[test]
fn start_on_a_missing_folder_is_missing() {
    let (_dir, root) = support::temp_root();

    let result = PaksWatcher::start(&root.join("Paks"), |_| {});

    assert!(matches!(result, Err(WatchError::Missing)));
}

#[test]
fn a_dropped_watcher_sends_nothing() {
    let fixture = support::fs_fixture();
    let (sender, receiver) = mpsc::channel();
    let watcher = PaksWatcher::start(&fixture.paks, move |signal| {
        let _ = sender.send(signal);
    })
    .expect("start");
    drop(watcher);
    // notify stops its event thread asynchronously; wait until it has dropped the handler.
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline
        && !matches!(
            receiver.recv_timeout(Duration::from_millis(100)),
            Err(mpsc::RecvTimeoutError::Disconnected)
        )
    {}

    fs::write(fixture.path("Mods/late.pak"), b"pak").expect("write");

    assert!(receiver.recv_timeout(Duration::from_millis(500)).is_err());
}
