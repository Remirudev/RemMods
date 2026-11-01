//! The only module allowed to mutate the filesystem; clippy's `disallowed-methods` bans the
//! `std::fs` mutators everywhere else.
#![allow(clippy::disallowed_methods)]

mod atomic;
mod backup;
mod batch;
mod dirs;
mod error;
mod gate;
mod journal;
mod lock;
pub mod names;
mod preflight;
mod recovery;
mod remove;
mod rename;
mod retry;
mod scope;
mod sessions;
mod watch;

pub use atomic::write_atomic;
pub use backup::backup_file;
pub use batch::{BatchError, BatchOptions, BatchReport, FaultPlan, RenameOp, RetryMode, StuckOp};
pub use dirs::{SMOKE_ROOT_PREFIX, create_smoke_root, ensure_dir, remove_smoke_root};
pub use error::FsError;
pub use gate::{OpGate, ReadGuard, WriteGuard};
pub use journal::{JournalProblem, RecoveryReport};
pub use lock::{AppLock, LockMode};
pub use rename::{RenameMethod, link_unlink_noreplace, placeholder_noreplace};
pub use scope::{Scope, ScopeViolation};
pub use watch::{Debounce, PaksWatcher, WatchError, WatchSignal, signal_for};

/// notify's event types, for the shell's watcher thread and tests.
pub mod notify_types {
    pub use notify::{Error, ErrorKind, Event, EventKind, event};
}
