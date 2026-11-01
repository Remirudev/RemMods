//! File-system matrix (CI job `fs-matrix`, SPEC-P2 §9.2). Every test returns early unless its
//! environment variable is set, so a local `cargo test` passes without mounts.
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod support;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, mpsc};
use std::time::{Duration, Instant};

use remmods_core::AppPaths;
use remmods_core::platform::{
    self, OsErrorClass, OsKind, TopdirPolicy, TrashEnv, TrashKind, TrashUnavailableReason,
};
use remmods_core::safe_fs::{
    FsError, OpGate, PaksWatcher, RenameOp, Scope, TrashReport, WatchSignal, placeholder_noreplace,
};

use support::{crash_after, fail_before, join_rel, options, snapshot, write_tree};

const MOD_TREE: &[(&str, &[u8])] = &[
    ("Mods/Cool/Cool.pak", b"pak"),
    ("Mods/Cool/Cool.ucas", b"ucas"),
    ("LogicMods/X/X.pak", b"xpak"),
    ("LogicMods/X/X.ucas", b"xucas"),
    ("Mods/Case/foo.pak", b"foo"),
];

/// `REMMODS_FS_MATRIX=<label>=<path>;…`
fn labelled_mounts() -> Vec<(String, PathBuf)> {
    let Ok(spec) = env::var("REMMODS_FS_MATRIX") else {
        return Vec::new();
    };
    spec.split(';')
        .filter_map(|item| item.split_once('='))
        .map(|(label, path)| (label.to_owned(), PathBuf::from(path)))
        .collect()
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name).map(PathBuf::from)
}

/// A Paks tree and app folders inside `base` (one file system).
struct Site {
    _dir: tempfile::TempDir,
    paks: PathBuf,
    app: AppPaths,
}

fn site_in(base: &Path) -> Site {
    let dir = tempfile::tempdir_in(base).expect("temp dir on the mount");
    let root = dunce::canonicalize(dir.path()).expect("canonical");
    let paks = root.join("Paks");
    write_tree(
        &paks,
        &[("Mods/", b""), ("~mods/", b""), ("LogicMods/", b"")],
    );
    write_tree(&paks, MOD_TREE);
    let app = AppPaths::under(&root.join("app"));
    fs::create_dir_all(&app.data_dir).expect("data dir");
    Site {
        _dir: dir,
        paks,
        app,
    }
}

fn scope_of(paks: &Path, app: &AppPaths) -> Scope {
    Scope::for_paks(paks, &[&app.data_dir]).expect("scope")
}

fn op(paks: &Path, from: &str, to: &str, is_dir: bool) -> RenameOp {
    RenameOp {
        from: join_rel(paks, from),
        to: join_rel(paks, to),
        is_dir,
    }
}

fn disable_both(paks: &Path) -> Vec<RenameOp> {
    vec![
        op(paks, "Mods/Cool/Cool.pak", "Mods/Cool/Cool.pak-x", false),
        op(paks, "Mods/Cool/Cool.ucas", "Mods/Cool/Cool.ucas-x", false),
        op(paks, "LogicMods/X", "LogicMods/.disabled_X", true),
        op(
            paks,
            "LogicMods/.disabled_X/X.pak",
            "LogicMods/.disabled_X/X.pak-x",
            false,
        ),
    ]
}

fn enable_both(paks: &Path) -> Vec<RenameOp> {
    vec![
        op(paks, "Mods/Cool/Cool.pak-x", "Mods/Cool/Cool.pak", false),
        op(paks, "Mods/Cool/Cool.ucas-x", "Mods/Cool/Cool.ucas", false),
        op(paks, "LogicMods/.disabled_X", "LogicMods/X", true),
        op(paks, "LogicMods/X/X.pak-x", "LogicMods/X/X.pak", false),
    ]
}

fn run_label(label: &str, base: &Path) {
    let site = site_in(base);
    let start = snapshot(&site.paks);
    let scope = || scope_of(&site.paks, &site.app);

    {
        let gate = OpGate::open(&site.app).expect("gate");
        let guard = gate.write(scope()).expect("write guard");
        let off = guard
            .rename_batch(&disable_both(&site.paks), &options(None))
            .expect("disable");
        println!("{label}: disable methods {:?}", off.methods);
        guard
            .rename_batch(&enable_both(&site.paks), &options(None))
            .expect("enable");
        assert_eq!(snapshot(&site.paks), start, "{label}: round trip");

        let exists = [op(
            &site.paks,
            "Mods/Cool/Cool.pak",
            "Mods/Cool/Cool.ucas",
            false,
        )];
        let refused = guard.rename_batch(&exists, &options(None));
        assert!(matches!(
            refused,
            Err(FsError::InvalidBatch { .. } | FsError::TargetExists { .. })
        ));
        let placeholder = placeholder_noreplace(
            guard.scope(),
            &join_rel(&site.paks, "Mods/Cool"),
            &join_rel(&site.paks, "Mods/Case"),
        );
        assert!(
            placeholder.is_err(),
            "{label}: placeholder over an existing folder"
        );
        assert_eq!(snapshot(&site.paks), start, "{label}: never overwrite");

        let case = [op(
            &site.paks,
            "Mods/Case/foo.pak",
            "Mods/Case/Foo.pak",
            false,
        )];
        match guard.rename_batch(&case, &options(None)) {
            Ok(_) => assert_eq!(
                support::names_in(&site.paks.join("Mods").join("Case")),
                ["Foo.pak"]
            ),
            Err(error) => {
                println!("{label}: case-only rename refused: {error}");
                assert_eq!(error.class(), Some(OsErrorClass::Unsupported), "{label}");
                assert_eq!(
                    support::names_in(&site.paks.join("Mods").join("Case")),
                    ["foo.pak"]
                );
            }
        }
        let start = snapshot(&site.paks);
        let failed = guard.rename_batch(&disable_both(&site.paks), &options(fail_before(3)));
        assert!(failed.is_err());
        assert_eq!(snapshot(&site.paks), start, "{label}: rollback");

        let crashed = guard.rename_batch(&disable_both(&site.paks), &options(crash_after(3)));
        assert!(crashed.is_err());
        assert_ne!(snapshot(&site.paks), start);
    }
    let before_recovery = snapshot(&site.paks);
    let gate = OpGate::open(&site.app).expect("gate");
    let guard = gate.write(scope()).expect("write guard");
    assert_eq!(guard.recovery().rolled_back, 3, "{label}: recovery");
    assert_ne!(snapshot(&site.paks), before_recovery);
}

#[test]
fn every_labelled_mount_passes_the_batch_checks() {
    for (label, base) in labelled_mounts() {
        println!("fs-matrix: {label} at {}", base.display());
        run_label(&label, &base);
    }
}

/// A Paks folder prepared by the mount script, app folders in the normal temp dir.
fn toggle_prepared(paks: &Path, rel_pak: &str) -> (Result<(), FsError>, bool) {
    let (_dir, root) = support::temp_root();
    let app = AppPaths::under(&root);
    let before = snapshot(paks);
    let gate = OpGate::open(&app).expect("gate");
    let guard = gate.write(scope_of(paks, &app)).expect("write guard");
    let ops = [op(paks, rel_pak, &format!("{rel_pak}-x"), false)];
    let result = guard.rename_batch(&ops, &options(None)).map(drop);
    (result, snapshot(paks) == before)
}

#[test]
fn a_read_only_mount_fails_read_only_before_any_change() {
    let Some(paks) = env_path("REMMODS_FS_MATRIX_RO") else {
        return;
    };

    let (result, unchanged) = toggle_prepared(&paks, "Mods/Ro/Ro.pak");

    assert_eq!(
        result.expect_err("refused").class(),
        Some(OsErrorClass::ReadOnlyFs)
    );
    assert!(unchanged);
}

#[test]
fn a_root_owned_mod_fails_access_denied() {
    let Some(paks) = env_path("REMMODS_FS_MATRIX_DENIED") else {
        return;
    };

    let (result, unchanged) = toggle_prepared(&paks, "Mods/R/R.pak");

    assert_eq!(
        result.expect_err("refused").class(),
        Some(OsErrorClass::AccessDenied)
    );
    assert!(unchanged);
}

#[test]
fn a_full_data_dir_fails_the_journal_write_and_renames_nothing() {
    let Some(full) = env_path("REMMODS_FS_MATRIX_FULL") else {
        return;
    };
    let site = site_in(&std::env::temp_dir());
    let data_dir = if full.join("data").is_dir() {
        full.join("data")
    } else {
        full
    };
    let app = AppPaths {
        lock_file: data_dir.join("remmods.lock"),
        log_dir: data_dir.join("logs"),
        settings_file: site.app.settings_file.clone(),
        config_dir: site.app.config_dir.clone(),
        data_dir,
    };
    let before = snapshot(&site.paks);

    let error = match OpGate::open(&app) {
        Err(error) => error,
        Ok(gate) => {
            let guard = gate.write(scope_of(&site.paks, &app));
            match guard {
                Err(error) => error,
                Ok(guard) => guard
                    .rename_batch(&disable_both(&site.paks), &options(None))
                    .expect_err("journal write fails"),
            }
        }
    };

    assert_eq!(error.class(), Some(OsErrorClass::NoSpace), "{error}");
    assert_eq!(snapshot(&site.paks), before);
}

#[test]
fn a_small_inotify_queue_overflows_into_a_rescan() {
    if env::var_os("REMMODS_FS_MATRIX_SMALL_QUEUE").is_none() {
        return;
    }
    let (_dir, root) = support::temp_root();
    let (sender, receiver) = mpsc::channel();
    let (release, gate) = mpsc::channel::<()>();
    let gate = Mutex::new(Some(gate));
    let _watcher = PaksWatcher::start(&root, move |signal| {
        // notify calls this on its reader thread; holding the first call stops it from draining
        // the inotify fd, so the 16-event kernel queue overflows instead of racing the writes.
        let first = gate.lock().expect("gate").take();
        if let Some(first) = first {
            let _ = first.recv_timeout(Duration::from_secs(10));
        }
        let _ = sender.send(signal);
    })
    .expect("start");

    for index in 0..300 {
        fs::write(root.join(format!("f{index}.pak")), b"x").expect("write");
    }
    drop(release);

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut rescan = false;
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        match receiver.recv_timeout(left) {
            Ok(WatchSignal::Rescan) => {
                rescan = true;
                break;
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    assert!(rescan, "no Rescan signal after 300 creations");
}

/// Trashes `Mods/Cool` of a fresh site on `base` with `env`; returns the result and whether the
/// site's Paks tree is unchanged.
fn trash_on(base: &Path, env: &TrashEnv) -> (Result<TrashReport, FsError>, bool) {
    let site = site_in(base);
    let before = snapshot(&site.paks);
    let gate = OpGate::open(&site.app).expect("gate");
    let guard = gate
        .write(scope_of(&site.paks, &site.app))
        .expect("write guard");
    let item = join_rel(&site.paks, "Mods/Cool");
    let result = guard.trash(env, &[item], chrono::Local::now());
    (result, snapshot(&site.paks) == before)
}

fn detect_env(home_trash: Option<PathBuf>) -> TrashEnv {
    TrashEnv {
        home_trash,
        uid: platform::current_uid(),
        topdirs: TopdirPolicy::Detect,
    }
}

/// F1: each labelled mount root must be chmod 1777 (CI), so `.Trash-$uid` can be created there.
#[test]
fn trash_on_mount_uses_topdir_user_trash() {
    if platform::os_kind() != OsKind::Linux {
        return;
    }
    for (label, base) in labelled_mounts() {
        let (result, _) = trash_on(&base, &detect_env(None));
        let report = result.unwrap_or_else(|error| panic!("{label}: trash failed: {error}"));
        let uid = platform::current_uid().expect("uid");
        let mount_root = dunce::canonicalize(&base).expect("canonical");
        let trash_root = mount_root.join(format!(".Trash-{uid}"));
        assert_eq!(report.kind, TrashKind::TopdirUser, "{label}");
        assert!(report.items[0].trashed.starts_with(&trash_root), "{label}");
        println!("fs-matrix: {label}: trashed into {}", trash_root.display());
    }
}

/// F2: `REMMODS_FS_MATRIX_BTRFS_SUBVOL` = a nested btrfs subvolume; a home trash in its parent
/// folder (the outer subvolume) has another st_dev and must be refused.
#[test]
fn trash_nested_btrfs_subvolume_is_other_device() {
    let Some(subvolume) = env_path("REMMODS_FS_MATRIX_BTRFS_SUBVOL") else {
        return;
    };
    let outer = subvolume
        .parent()
        .expect("outer subvolume")
        .join("remmods-home");
    fs::create_dir_all(&outer).expect("outer folder");
    let home_trash = outer.join("Trash");
    let env = TrashEnv {
        home_trash: Some(home_trash.clone()),
        uid: platform::current_uid(),
        topdirs: TopdirPolicy::Deny,
    };
    let (result, unchanged) = trash_on(&subvolume, &env);
    assert!(
        matches!(
            result,
            Err(FsError::TrashUnavailable {
                reason: TrashUnavailableReason::OtherDevice
            })
        ),
        "{result:?}"
    );
    assert!(unchanged);
    assert!(!home_trash.exists());
}

/// F3: `REMMODS_FS_MATRIX_NOREPLACE_UNSUPPORTED` = a mount whose renameat2 rejects NOREPLACE.
/// None of today's matrix file systems does, so this normally only logs.
#[test]
fn trash_rename_flag_unsupported_refuses() {
    let Some(base) = env_path("REMMODS_FS_MATRIX_NOREPLACE_UNSUPPORTED") else {
        println!("fs-matrix: no file system without RENAME_NOREPLACE; F3 skipped");
        return;
    };
    let (result, unchanged) = trash_on(&base, &detect_env(None));
    assert!(
        matches!(
            result,
            Err(FsError::TrashUnavailable {
                reason: TrashUnavailableReason::NoUsableTrash
            })
        ),
        "{result:?}"
    );
    assert!(unchanged);
}
