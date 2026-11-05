//! Host programs get no descriptor RemMods inherited without close-on-exec (the AppImage
//! runtime's keepalive pipe and fd 1023), while RemMods keeps them open itself.
#![cfg(target_os = "linux")]
#![allow(clippy::disallowed_methods, clippy::expect_used)]

use std::fs;
use std::io;
use std::os::fd::{AsFd, AsRawFd, OwnedFd};
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use remmods_core::platform::{host_command, spawn_detached};
use rustix::io::{FdFlags, fcntl_dupfd_cloexec, fcntl_getfd, fcntl_setfd};

/// Above anything the test harness opens, so the child's own descriptors cannot collide.
const HIGH_FD: i32 = 700;

/// A pipe end at a high number with close-on-exec cleared, like the runtime's `pipe()` + `dup2`.
fn inherited_fd(fd: impl AsFd) -> OwnedFd {
    let high = fcntl_dupfd_cloexec(fd, HIGH_FD).expect("dup to a high fd");
    fcntl_setfd(&high, FdFlags::empty()).expect("clear close-on-exec");
    high
}

/// `ls /proc/self/fd` of a child started by `command`, written to `out`.
fn listed_by_child(out: &Path, start: impl FnOnce(Command) -> io::Result<()>) -> Vec<i32> {
    let mut command = host_command("sh");
    command
        .arg("-c")
        .arg("ls /proc/self/fd > \"$1\"")
        .arg("sh")
        .arg(out);
    start(command).expect("start sh");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(text) = fs::read_to_string(out)
            && text.ends_with('\n')
        {
            return text
                .lines()
                .map(|line| line.trim().parse().expect("fd number"))
                .collect();
        }
        assert!(Instant::now() < deadline, "the child wrote no listing");
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn host_programs_do_not_inherit_descriptors_without_cloexec() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (reader, writer) = io::pipe().expect("pipe");
    let reader = inherited_fd(reader);
    let writer = inherited_fd(writer);

    // Control: a plain spawn passes both ends on, so the listing can see them.
    let plain = listed_by_child(&dir.path().join("plain.txt"), |mut command| {
        command.spawn()?.wait().map(drop)
    });
    assert!(
        plain.contains(&reader.as_raw_fd()),
        "plain child: {plain:?}"
    );
    assert!(
        plain.contains(&writer.as_raw_fd()),
        "plain child: {plain:?}"
    );

    let detached = listed_by_child(&dir.path().join("host.txt"), spawn_detached);
    assert!(
        !detached.contains(&reader.as_raw_fd()),
        "host program: {detached:?}"
    );
    assert!(
        !detached.contains(&writer.as_raw_fd()),
        "host program: {detached:?}"
    );

    // RemMods itself keeps them open and unchanged.
    assert_eq!(fcntl_getfd(&reader).expect("reader open"), FdFlags::empty());
    assert_eq!(fcntl_getfd(&writer).expect("writer open"), FdFlags::empty());
}
