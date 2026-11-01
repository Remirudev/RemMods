#![allow(clippy::disallowed_methods)]

mod support;

use std::fs;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use remmods_core::safe_fs::{AppLock, FsError};

const DEAD_SESSION: &str = "0123456789abcdef0123456789abcdef";

fn session_files(fixture: &support::FsFixture) -> Vec<String> {
    support::names_in(&fixture.sessions_dir())
}

#[test]
fn a_second_gate_cannot_write_while_the_first_writes() {
    let fixture = support::fs_fixture();
    let first = fixture.gate();
    let second = fixture.gate();
    let _guard = first.write(fixture.scope()).expect("first write");
    let started = Instant::now();

    let result = second.write(fixture.scope());

    assert!(matches!(result, Err(FsError::Busy)));
    assert!(started.elapsed() >= Duration::from_millis(1900));
}

#[test]
fn a_read_waits_for_a_write_of_another_gate_and_gives_up() {
    let fixture = support::fs_fixture();
    let first = fixture.gate();
    let second = fixture.gate();
    let _guard = first.write(fixture.scope()).expect("first write");

    assert!(matches!(second.read(), Err(FsError::Busy)));
}

#[test]
fn two_reads_coexist() {
    let fixture = support::fs_fixture();
    let first = fixture.gate();
    let second = fixture.gate();

    let _one = first.read().expect("first read");
    let _two = second.read().expect("second read");
    let _three = first.read().expect("same gate, second read");
}

#[test]
fn a_write_excludes_reads_in_the_same_process() {
    let fixture = support::fs_fixture();
    let gate = fixture.gate();
    let (sender, receiver) = mpsc::channel();

    thread::scope(|scope| {
        let guard = gate.write(fixture.scope()).expect("write");
        scope.spawn(|| {
            let read = gate.read();
            sender.send(read.is_ok()).expect("send");
        });
        assert!(receiver.recv_timeout(Duration::from_millis(300)).is_err());
        drop(guard);
        assert_eq!(receiver.recv_timeout(Duration::from_secs(5)), Ok(true));
    });
}

#[test]
fn open_removes_dead_session_files_without_entries() {
    let fixture = support::fs_fixture();
    fs::create_dir_all(fixture.sessions_dir()).expect("sessions dir");
    fs::write(
        fixture.sessions_dir().join(format!("{DEAD_SESSION}.lock")),
        b"",
    )
    .expect("write");

    let gate = fixture.gate();

    assert_eq!(
        session_files(&fixture),
        [format!("{}.lock", gate.session())]
    );
}

#[test]
fn a_live_gates_session_file_survives_another_open() {
    let fixture = support::fs_fixture();
    let first = fixture.gate();
    let own = format!("{}.lock", first.session());

    let second = fixture.gate();

    let files = session_files(&fixture);
    assert!(files.contains(&own));
    assert!(files.contains(&format!("{}.lock", second.session())));
    assert_ne!(first.session(), second.session());
}

#[test]
fn a_settings_style_wait_succeeds_once_a_read_ends() {
    let fixture = support::fs_fixture();
    let gate = fixture.gate();
    let read = gate.read().expect("read");
    let lock_file = fixture.app.lock_file.clone();

    let waiter =
        thread::spawn(move || AppLock::exclusive_within(&lock_file, Duration::from_secs(2)));
    thread::sleep(Duration::from_millis(300));
    drop(read);

    assert!(waiter.join().expect("join").is_ok());
}

#[test]
fn shared_within_gives_up_after_the_wait() {
    let fixture = support::fs_fixture();
    let _held = AppLock::try_exclusive(&fixture.app.lock_file).expect("exclusive");
    let started = Instant::now();

    let result = AppLock::shared_within(&fixture.app.lock_file, Duration::from_millis(200));

    assert!(matches!(result, Err(FsError::Busy)));
    assert!(started.elapsed() >= Duration::from_millis(200));
}
