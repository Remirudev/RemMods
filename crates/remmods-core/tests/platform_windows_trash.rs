//! P3a has no Recycle Bin mover on Windows (WG-5).
#![cfg(windows)]
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod support;

use remmods_core::platform::{self, OsErrorClass, TrashUnavailableReason};
use remmods_core::safe_fs::FsError;

use support::tree_listing;

#[test]
fn trash_is_unavailable_and_changes_nothing() {
    let fixture = support::fs_fixture();
    support::write_tree(&fixture.paks, &[("Mods/Cool/Cool.pak", b"pak")]);
    let env = support::trash_env(&fixture.root);
    // Only the Paks tree: the app dirs hold lock files with byte-range locks that Windows refuses
    // to read while the gate is alive.
    let before = tree_listing(&fixture.paks);
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("write guard");

    let result = guard.trash(&env, &[fixture.path("Mods/Cool")], chrono::Local::now());
    drop(guard);

    match result {
        Err(FsError::TrashUnavailable { reason }) => {
            assert_eq!(reason, TrashUnavailableReason::NotSupported);
        }
        other => panic!("expected TrashUnavailable, got {other:?}"),
    }
    assert_eq!(tree_listing(&fixture.paks), before);
}

#[test]
fn mount_identity_is_unsupported() {
    let (_dir, root) = support::temp_root();
    let error = platform::mount_identity(&root).expect_err("unsupported");
    assert_eq!(platform::classify(&error), OsErrorClass::Unsupported);
}
