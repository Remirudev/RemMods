//! The only module allowed to mutate the filesystem; clippy's `disallowed-methods` bans the
//! `std::fs` mutators everywhere else.
#![allow(clippy::disallowed_methods)]

mod atomic;
mod backup;
mod dirs;
mod error;
mod lock;
pub mod names;
mod retry;

pub use atomic::write_atomic;
pub use backup::backup_file;
pub use dirs::{SMOKE_ROOT_PREFIX, create_smoke_root, ensure_dir, remove_smoke_root};
pub use error::FsError;
pub use lock::{AppLock, LockMode};
