//! Install data safety (P4 spec §8.2 D1–D8, D13–D17, D21, D23): refusals change nothing, staging
//! is inert and journaled, the commit is all-or-none.
#![allow(clippy::expect_used, clippy::disallowed_methods)]

mod archive_support;
mod install_support;
mod mods_support;
mod support;

use std::fs;
use std::sync::{Arc, Mutex};

use archive_support::{CraftEntry, ZipCraft};
use install_support::{
    InstallFixture, default_request, fixture, fixture_without_trash, has_issue, install_error,
    journal_entries,
};
use mods_support::manage::{HEX, mods_error};
use remmods_core::archive::ArchiveError;
use remmods_core::install::{
    InstallBlock, InstallError, InstallHooks, IssueCode, IssueSeverity, KeptReason, ReplacedOld,
};
use remmods_core::mods::{ModState, ModsError, RepairKind};
use remmods_core::platform;
use remmods_core::safe_fs::{FaultPlan, Scope};

const COOL: &[(&str, &[u8])] = &[("Cool/x.pak", b"new pak"), ("Cool/x.ucas", b"new ucas")];

fn assert_no_staging(fixture: &InstallFixture) {
    for root in ["Mods", "~mods", "LogicMods"] {
        assert!(
            fixture.temp_folders(root).is_empty(),
            "temp folder left in {root}"
        );
    }
    assert!(journal_entries(&fixture.fx.paths.data_dir).is_empty());
}

#[test]
fn d1_inspect_writes_nothing() {
    let fixture = fixture(&[("Mods/Other/o.pak", b"o")]);
    let good = fixture.zip("Cool.zip", COOL);
    let bad = fixture.zip("Evil.zip", &[("../evil.pak", b"evil")]);
    let before = fixture.state();
    let inspection = fixture.inspect(&good);
    assert!(!inspection.blocked);
    let refused = fixture.inspect(&bad);
    assert!(refused.blocked);
    assert!(refused.candidates.is_empty());
    assert!(has_issue(
        &refused,
        IssueSeverity::Block,
        IssueCode::UnsafePath
    ));
    assert_eq!(fixture.state(), before);
    assert_no_staging(&fixture);
}

#[test]
fn d2_unsafe_and_unsupported_archives_are_refused() {
    let fixture = fixture(&[("Mods/Other/o.pak", b"o")]);
    let before = fixture.state();
    let traversal = fixture.zip("Evil.zip", &[("a/../../evil.pak", b"evil")]);
    let inspection = fixture.inspect(&traversal);
    let error = install_error(fixture.start(&inspection));
    assert!(matches!(
        error,
        InstallError::Blocked {
            reason: InstallBlock::Issues
        }
    ));
    let script = fixture.zip("Script.zip", &[("Scripts/main.lua", b"print()")]);
    let inspection = fixture.inspect(&script);
    assert!(has_issue(
        &inspection,
        IssueSeverity::Block,
        IssueCode::ScriptMod
    ));
    let error = install_error(fixture.start(&inspection));
    assert!(matches!(
        error,
        InstallError::Blocked {
            reason: InstallBlock::Issues
        }
    ));
    let encrypted = ZipCraft::new()
        .with(CraftEntry {
            encrypted_flag: true,
            ..CraftEntry::file("Cool/x.pak", b"secret")
        })
        .write(&fixture.src, "Locked.zip");
    let multi = ZipCraft::new()
        .with(CraftEntry {
            disk: 1,
            ..CraftEntry::file("Cool/x.pak", b"part")
        })
        .write(&fixture.src, "Split.zip");
    for (path, expect_encrypted) in [(encrypted, true), (multi, false)] {
        let result = fixture.fx.core.install_inspect(
            &remmods_core::install::InstallSource::Path(path),
            &Default::default(),
        );
        match install_error(result) {
            InstallError::Archive(ArchiveError::Encrypted) => assert!(expect_encrypted),
            InstallError::Archive(ArchiveError::Unsupported { .. }) => assert!(!expect_encrypted),
            other => panic!("unexpected {other:?}"),
        }
    }
    assert_eq!(fixture.state(), before);
    assert_no_staging(&fixture);
}

#[test]
fn d3_d4_staging_is_inert_and_journaled() {
    let fixture = fixture(&[("Mods/Other/o.pak", b"o")]);
    let zip = fixture.zip(
        "Cool.zip",
        &[
            ("Cool/x.pak", b"pak"),
            ("Cool/x.ucas", b"ucas"),
            ("Cool/opt.pak-x", b"opt"),
        ],
    );
    let inspection = fixture.inspect(&zip);
    let seen: Arc<Mutex<Vec<String>>> = Arc::default();
    let record = Arc::clone(&seen);
    let paks = fixture.fx.paks.clone();
    let data_dir = fixture.fx.paths.data_dir.clone();
    fixture.fx.core.inject_install_hooks(InstallHooks {
        after_stage: Some(Arc::new(move |folders| {
            let mut out = record.lock().expect("lock");
            for folder in folders {
                let name = folder
                    .file_name()
                    .expect("name")
                    .to_string_lossy()
                    .into_owned();
                out.push(format!("folder {name}"));
                out.extend(support::tree_listing(folder));
            }
            out.extend(
                support::tree_listing(&paks)
                    .into_iter()
                    .map(|line| format!("paks {line}")),
            );
            let journal = journal_entries(&data_dir);
            out.push(format!("journal {}", serde_json::Value::Array(journal)));
        })),
        ..InstallHooks::default()
    });
    fixture.start(&inspection).expect("install");
    let seen = seen.lock().expect("lock").clone();
    let folder = seen[0].strip_prefix("folder ").expect("folder line");
    let suffix = folder.strip_prefix("_install_tmp_").expect("staging name");
    assert!(suffix.len() == 32 && suffix.bytes().all(|b| b.is_ascii_hexdigit()));
    let staged: Vec<&String> = seen.iter().filter(|line| line.starts_with("f ")).collect();
    assert_eq!(staged.len(), 3, "{seen:?}");
    assert!(staged.iter().any(|line| line.starts_with("f x.pak-x ")));
    assert!(staged.iter().any(|line| line.starts_with("f x.ucas-x ")));
    let opt = support::tree_listing(&fixture.fx.paks.join("Mods").join("Cool"))
        .into_iter()
        .find(|line| line.starts_with("f opt.pak-x "))
        .expect("opt.pak-x installed");
    assert!(staged.contains(&&opt), "shipped -x file is byte-identical");
    let live: Vec<&String> = seen
        .iter()
        .filter(|line| line.starts_with("paks f ") && line.contains(".pak "))
        .collect();
    assert_eq!(live.len(), 1, "only the pre-existing live pak: {live:?}");
    let journal = seen
        .iter()
        .find(|line| line.starts_with("journal "))
        .expect("journal");
    let entries: serde_json::Value =
        serde_json::from_str(journal.strip_prefix("journal ").expect("json")).expect("json");
    let entries = entries.as_array().expect("array");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["label"], "install-stage");
    assert_eq!(entries[0]["ops"][0]["kind"], "staging");
    assert!(
        entries[0]["ops"][0]["path"]["text"]
            .as_str()
            .expect("text")
            .contains(folder)
    );
    assert_no_staging(&fixture);
}

#[test]
fn d5_new_install_is_live_with_meta() {
    let fixture = fixture(&[]);
    let zip = fixture.zip(
        "Cool v1.zip",
        &[("Cool/x.pak", b"pak"), ("Cool/opt.pak-x", b"o")],
    );
    let inspection = fixture.inspect(&zip);
    let outcome = fixture.start(&inspection).expect("install");
    assert_eq!(fixture.read("Mods/Cool/x.pak"), b"pak");
    assert!(fixture.exists("Mods/Cool/opt.pak-x"));
    assert!(!fixture.exists("Mods/Cool/opt.pak"));
    assert_eq!(fixture.fx.row("Standard:Cool").state, ModState::Partial);
    let meta = fixture.meta();
    let entry = &meta["mods"]["Standard:Cool"];
    assert!(entry["added"].is_string());
    assert_eq!(entry["source"], "Cool v1.zip");
    assert_eq!(outcome.installed.len(), 1);
    assert_eq!(outcome.installed[0].key, "Standard:Cool");
    assert!(outcome.installed[0].mod_ref.is_some());
    assert_no_staging(&fixture);
}

#[test]
fn d6_replace_trashes_the_old_version() {
    let fixture = fixture(&[("Mods/Cool/a.pak", b"old")]);
    fixture
        .fx
        .toggle(&["Standard:Cool"], false)
        .expect("disable");
    assert!(fixture.meta()["mods"]["Standard:Cool"]["disabled"].is_object());
    let zip = fixture.zip("Cool.zip", COOL);
    let inspection = fixture.inspect(&zip);
    assert!(has_issue(
        &inspection,
        IssueSeverity::Warn,
        IssueCode::Replaces
    ));
    let outcome = fixture.start(&inspection).expect("replace");
    // Windows has no Trash before P3b: the old version stays as a Repair leftover there.
    let windows = platform::os_kind() == platform::OsKind::Windows;
    let expected = if windows {
        ReplacedOld::KeptInRepair {
            reason: KeptReason::TrashUnavailable,
        }
    } else {
        ReplacedOld::Trashed
    };
    assert_eq!(outcome.installed[0].replaced, Some(expected));
    assert_eq!(fixture.read("Mods/Cool/x.pak"), b"new pak");
    assert!(!fixture.exists("Mods/Cool/a.pak") && !fixture.exists("Mods/Cool/a.pak-x"));
    let state = fixture.state();
    if !windows {
        let trashed = |line: &String| line.starts_with("trash ") && line.contains("a.pak");
        assert!(state.tree.iter().any(trashed));
        assert_no_staging(&fixture);
    }
    assert!(fixture.meta()["mods"]["Standard:Cool"]["disabled"].is_null());
    assert_eq!(fixture.fx.row("Standard:Cool").state, ModState::Enabled);
    assert!(journal_entries(&fixture.fx.paths.data_dir).is_empty());
}

#[test]
fn d7_replace_without_trash_keeps_an_inert_aside() {
    let fixture = fixture_without_trash(&[("Mods/Cool/a.pak", b"old")]);
    let zip = fixture.zip("Cool.zip", COOL);
    let inspection = fixture.inspect(&zip);
    let outcome = fixture.start(&inspection).expect("replace");
    assert_eq!(
        outcome.installed[0].replaced,
        Some(ReplacedOld::KeptInRepair {
            reason: KeptReason::TrashUnavailable
        })
    );
    let asides = fixture.temp_folders("Mods");
    assert_eq!(asides.len(), 1);
    assert!(asides[0].starts_with("_Cool_tmp_"));
    let aside = fixture.fx.paks.join("Mods").join(&asides[0]);
    assert_eq!(support::names_in(&aside), vec!["a.pak-x"]);
    let repair = fixture.fx.core.repair_list().expect("repair_list");
    assert!(
        repair
            .items
            .iter()
            .any(|item| item.kind == RepairKind::Leftover)
    );
}

#[test]
fn d8_commit_failure_at_every_op_restores_everything() {
    let fixture = fixture(&[("Mods/Cool/a.pak", b"old")]);
    let zip = fixture.zip("Cool.zip", &[("Cool/x.pak", b"pak")]);
    let inspection = fixture.inspect(&zip);
    let before = fixture.state();
    // Set-aside folder, a.pak → -x, staging → Cool, x.pak-x → x.pak.
    for k in 0..4 {
        fixture.fx.core.inject_batch_fault(FaultPlan {
            fail_before: Some(k),
            ..FaultPlan::default()
        });
        let result = fixture.start(&inspection);
        assert!(result.is_err(), "op {k}");
        assert_eq!(fixture.state(), before, "op {k}");
        assert!(fixture.temp_folders("Mods").is_empty(), "op {k}");
    }
}

#[test]
fn d13_source_rewritten_after_inspect() {
    let fixture = fixture(&[]);
    let zip = fixture.zip("Cool.zip", COOL);
    let inspection = fixture.inspect(&zip);
    fs::write(
        &zip,
        install_support::zip_bytes(&[("Cool/x.pak", b"other bytes, longer")]),
    )
    .expect("rewrite");
    let before = fixture.state();
    let error = install_error(fixture.start(&inspection));
    assert!(matches!(error, InstallError::SourceChanged));
    assert_eq!(fixture.state(), before);
}

#[test]
fn d14_target_created_after_inspect_or_during_extraction() {
    let fixture = fixture(&[("Mods/Other/o.pak", b"o")]);
    let zip = fixture.zip("Cool.zip", COOL);
    let inspection = fixture.inspect(&zip);
    let cool = fixture.fx.paks.join("Mods").join("Cool");
    fs::create_dir(&cool).expect("plant Cool");
    let before = fixture.state();
    let error = install_error(fixture.start(&inspection));
    assert!(matches!(
        error,
        InstallError::Blocked {
            reason: InstallBlock::TargetChanged
        }
    ));
    assert_eq!(fixture.state(), before);
    fs::remove_dir(&cool).expect("remove Cool");
    let inspection = fixture.inspect(&zip);
    let before = fixture.state();
    let planted = cool.clone();
    fixture.fx.core.inject_install_hooks(InstallHooks {
        after_stage: Some(Arc::new(move |_| fs::create_dir(&planted).expect("plant"))),
        ..InstallHooks::default()
    });
    let error = install_error(fixture.start(&inspection));
    assert!(matches!(
        error,
        InstallError::Blocked {
            reason: InstallBlock::TargetChanged
        }
    ));
    let mut after = fixture.state();
    after.tree.retain(|line| line != "d Mods/Cool");
    assert_eq!(after, before);
    assert_no_staging(&fixture);
}

#[test]
fn d15_game_running() {
    let fixture = fixture(&[]);
    let zip = fixture.zip("Cool.zip", COOL);
    let inspection = fixture.inspect(&zip);
    let before = fixture.state();
    fixture
        .fx
        .running
        .store(true, std::sync::atomic::Ordering::SeqCst);
    assert!(matches!(
        mods_error(fixture.start(&inspection)),
        ModsError::GameRunning
    ));
    assert_eq!(fixture.state(), before);
    let mut request = default_request(&inspection);
    request.apply_while_running = true;
    fixture.start_with(&request).expect("install while running");
    assert!(fixture.exists("Mods/Cool/x.pak"));
}

#[test]
fn d16_no_space() {
    let fixture = fixture(&[]);
    let zip = fixture.zip("Cool.zip", COOL);
    let inspection = fixture.inspect(&zip);
    fixture.fx.core.inject_install_hooks(InstallHooks {
        available_space: Some(1),
        ..InstallHooks::default()
    });
    let before = fixture.state();
    let error = install_error(fixture.start(&inspection));
    assert!(matches!(error, InstallError::NoSpace { available: 1, .. }));
    assert_eq!(fixture.state(), before);
}

#[test]
fn d17_missing_root_is_created_exactly() {
    let fixture = fixture(&[]);
    let zip = fixture.zip("Cool.zip", &[("~mods/Cool/x.pak", b"pak")]);
    let inspection = fixture.inspect(&zip);
    fixture.start(&inspection).expect("install");
    assert_eq!(support::names_in(&fixture.fx.paks), vec!["~mods"]);
    assert!(fixture.exists("~mods/Cool/x.pak"));

    let fixture = install_support::fixture(&[("~MODS/Other/o.pak", b"o")]);
    let app_dirs = [fixture.fx.paths.data_dir.as_path()];
    {
        let scope = Scope::for_paks(&fixture.fx.paks, &app_dirs).expect("scope");
        let mut guard = fixture.fx.core.ops.write(scope).expect("guard");
        assert!(guard.create_mod_root("~mods").is_err());
        assert!(guard.create_mod_root("Other").is_err());
    }
    let zip = fixture.zip("Cool.zip", &[("~mods/Cool/x.pak", b"pak")]);
    let inspection = fixture.inspect(&zip);
    fixture.start(&inspection).expect("install");
    assert_eq!(support::names_in(&fixture.fx.paks), vec!["~MODS"]);
    assert!(fixture.exists("~MODS/Cool/x.pak"));
}

#[test]
fn d21_two_picks_are_all_or_none() {
    let fixture = fixture(&[]);
    let zip = fixture.zip("Pack.zip", &[("A/a.pak", b"a"), ("B/b.pak", b"b")]);
    let inspection = fixture.inspect(&zip);
    assert_eq!(inspection.candidates.len(), 2);
    let before = fixture.state();
    // Pick A: staging → A, a.pak-x → a.pak; pick B: ops 2 and 3.
    fixture.fx.core.inject_batch_fault(FaultPlan {
        fail_before: Some(3),
        ..FaultPlan::default()
    });
    assert!(fixture.start(&inspection).is_err());
    let mut after = fixture.state();
    after.tree.retain(|line| line != "d Mods");
    assert_eq!(after, before);
    assert_no_staging(&fixture);
    fixture.start(&inspection).expect("install both");
    assert!(fixture.exists("Mods/A/a.pak") && fixture.exists("Mods/B/b.pak"));
}

#[test]
fn d23_panic_in_the_target_is_contained() {
    let fixture = fixture(&[("Mods/Other/o.pak", b"o")]);
    let zip = fixture.zip("Cool.zip", COOL);
    let inspection = fixture.inspect(&zip);
    let before = fixture.state();
    fixture.fx.core.inject_install_hooks(InstallHooks {
        panic_in_target: true,
        ..InstallHooks::default()
    });
    let error = install_error(fixture.start(&inspection));
    assert!(
        matches!(error, InstallError::Archive(ArchiveError::Corrupt { .. })),
        "{error:?}"
    );
    assert_eq!(fixture.state(), before);
    assert_no_staging(&fixture);
    fixture
        .fx
        .core
        .inject_install_hooks(InstallHooks::default());
    fixture
        .start(&inspection)
        .expect("the job slot is free again");
}

#[test]
fn d18_journaled_staging_is_protected() {
    let fixture = fixture(&[("Mods/Other/o.pak", b"o")]);
    let app_dirs = [fixture.fx.paths.data_dir.as_path()];
    let scope = Scope::for_paks(&fixture.fx.paks, &app_dirs).expect("scope");
    let guard = fixture.fx.core.ops.write(scope).expect("guard");
    let root = fixture.fx.paks.join("Mods");
    let staging = guard.create_staging(&root, HEX).expect("staging");
    let path = staging.path().to_path_buf();
    let trash = guard.trash(
        &fixture.fx.core.trash_env,
        std::slice::from_ref(&path),
        chrono::Local::now(),
    );
    assert!(matches!(
        trash,
        Err(remmods_core::safe_fs::FsError::AsideJournaled { .. })
    ));
    let removed = guard.remove_aside(&path);
    assert!(matches!(
        removed,
        Err(remmods_core::safe_fs::FsError::AsideJournaled { .. })
    ));
    fs::write(
        platform::fs_path(&path.join("x.pak")).expect("fs path"),
        b"live",
    )
    .expect("plant");
    assert!(guard.discard_staging(staging).is_err());
    assert!(
        path.join("x.pak").exists(),
        "a live payload is never removed"
    );
    assert!(
        !guard.is_journaled(&path).expect("journal"),
        "left unjournaled"
    );
}
