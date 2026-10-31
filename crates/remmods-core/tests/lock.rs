mod support;

use remmods_core::safe_fs::{AppLock, FsError, LockMode};

#[test]
fn second_exclusive_lock_is_busy() {
    let (_dir, root) = support::temp_root();
    let path = root.join("remmods.lock");
    let first = AppLock::try_exclusive(&path).expect("first exclusive");

    let second = AppLock::try_exclusive(&path);

    assert_eq!(first.mode(), LockMode::Exclusive);
    assert!(matches!(second, Err(FsError::Busy)));
}

#[test]
fn shared_locks_coexist() {
    let (_dir, root) = support::temp_root();
    let path = root.join("remmods.lock");

    let first = AppLock::try_shared(&path).expect("first shared");
    let second = AppLock::try_shared(&path).expect("second shared");

    assert_eq!(first.mode(), LockMode::Shared);
    assert_eq!(second.mode(), LockMode::Shared);
}

#[test]
fn exclusive_lock_blocks_shared_lock() {
    let (_dir, root) = support::temp_root();
    let path = root.join("remmods.lock");
    let _exclusive = AppLock::try_exclusive(&path).expect("exclusive");

    assert!(matches!(AppLock::try_shared(&path), Err(FsError::Busy)));
}

#[test]
fn shared_lock_blocks_exclusive_lock() {
    let (_dir, root) = support::temp_root();
    let path = root.join("remmods.lock");
    let _shared = AppLock::try_shared(&path).expect("shared");

    assert!(matches!(AppLock::try_exclusive(&path), Err(FsError::Busy)));
}

#[test]
fn dropping_the_guard_releases_the_lock() {
    let (_dir, root) = support::temp_root();
    let path = root.join("remmods.lock");
    let first = AppLock::try_exclusive(&path).expect("first exclusive");
    drop(first);

    let again = AppLock::try_exclusive(&path).expect("exclusive after drop");
    drop(again);
    AppLock::try_shared(&path).expect("shared after drop");
}

#[test]
fn lock_file_and_missing_parent_are_created() {
    let (_dir, root) = support::temp_root();
    let path = root.join("data").join("nested").join("remmods.lock");

    let _lock = AppLock::try_exclusive(&path).expect("exclusive");

    assert!(path.is_file());
}
