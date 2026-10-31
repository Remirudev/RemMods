use std::io;
use std::path::Path;
use std::thread;
use std::time::Instant;

use super::FsError;
use crate::platform::{self, OsErrorClass, RetryPolicy};

/// Runs `attempt` until it succeeds or the policy for its error class is used up.
pub(super) fn with_retry<T>(
    op: &'static str,
    path: &Path,
    policy_for: fn(OsErrorClass) -> RetryPolicy,
    mut attempt: impl FnMut() -> io::Result<T>,
) -> Result<T, FsError> {
    let started = Instant::now();
    let mut tries = 0_u32;
    loop {
        tries += 1;
        let source = match attempt() {
            Ok(value) => return Ok(value),
            Err(source) => source,
        };
        let policy = policy_for(platform::classify(&source));
        if tries >= policy.attempts || started.elapsed() >= policy.total {
            return Err(FsError::io(op, path, source));
        }
        log::debug!(
            "{op} {} failed (try {tries}), retrying: {source}",
            path.display()
        );
        thread::sleep(policy.pause());
    }
}
