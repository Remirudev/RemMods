//! The only module allowed to mutate the filesystem; clippy's `disallowed-methods` bans the
//! `std::fs` mutators everywhere else.
#![allow(clippy::disallowed_methods)]

mod app_files;
mod aside;
mod atomic;
mod backup;
mod batch;
mod busy;
mod dirs;
mod error;
mod export;
mod gate;
mod journal;
mod lock;
mod mod_files;
pub mod names;
mod preflight;
mod recovery;
mod remove;
mod rename;
mod retry;
mod scope;
mod sessions;
mod skin_saves;
mod staging;
mod trash;
mod watch;

pub use app_files::{live_session_exists, remove_app_dir, remove_app_file};
pub use aside::set_aside_name;
pub use atomic::write_atomic;
pub use backup::backup_file;
pub use batch::{
    BatchError, BatchOptions, BatchReport, BusyFault, FaultPlan, RenameOp, RetryMode, StuckOp,
};
pub use busy::{BUSY_NOTICE_AFTER, BusyCanceller, BusyEvent, BusyPhase, BusySink, BusyWaits};
pub use dirs::{SMOKE_ROOT_PREFIX, create_smoke_root, ensure_dir, remove_smoke_root};
pub use error::FsError;
pub use gate::{OpGate, ReadGuard, WriteGuard};
pub use journal::{JournalProblem, RecoveryReport};
pub use lock::{AppLock, LockMode};
pub use rename::{RenameMethod, link_unlink_noreplace, placeholder_noreplace};
pub use scope::{Scope, ScopeViolation};
pub(crate) use scope::{is_within, resolve_existing};
pub use skin_saves::{SKIN_SAVE_FILES, SkinSaves};
pub use staging::{StagedFile, StagingDir};
pub use trash::{TrashReport, TrashedItem, percent_encode_path, trashinfo_text};
pub use watch::{Debounce, PaksWatcher, WatchError, WatchSignal, signal_for};

/// notify's event types, for the shell's watcher thread and tests.
pub mod notify_types {
    pub use notify::{Error, ErrorKind, Event, EventKind, event};
}
