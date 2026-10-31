#![allow(clippy::expect_used)]

use std::path::PathBuf;

use tempfile::TempDir;

/// A temp dir and its canonical path. Windows runners hand out 8.3 aliases (`RUNNER~1`), so tests
/// compare against the canonical form only.
pub fn temp_root() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let root = dunce::canonicalize(dir.path()).expect("canonicalize temp dir");
    (dir, root)
}
