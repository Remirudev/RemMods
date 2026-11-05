//! Permanent delete with the second confirmation (spec §5.4, §8.2 D1–D15). Data safety first.
#![allow(clippy::expect_used)]

mod mods_support;
mod support;

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use mods_support::DATA;
use mods_support::manage::{HEX, core_with_trash, entry_target, mods_error, restart, tree};
use remmods_core::CoreError;
use remmods_core::mods::{
    DeleteRequest, DeleteTokens, ManageAction, ManageBlock, ModRef, ModTarget, ModsError,
    RepairAction, list_toggle_files, plan_set_aside, repair_items,
};
use remmods_core::safe_fs::{FaultPlan, FsError, Scope};

const COOL: &[(&str, &[u8])] = &[
    ("Mods/Cool/Cool.pak", DATA),
    ("Mods/Cool/Cool.ucas", DATA),
    ("Mods/Cool/Sub/Cool_P.utoc", DATA),
    ("Mods/Cool/readme.txt", DATA),
    ("Mods/Other/Other.pak", DATA),
];

fn request(target: ModTarget, token: &str) -> DeleteRequest {
    DeleteRequest {
        target,
        token: token.to_owned(),
        apply_while_running: false,
    }
}

fn delete(
    fixture: &mods_support::manage::ToggleFixture,
    key: &str,
) -> Result<remmods_core::mods::DeleteOutcome, CoreError> {
    let target = fixture.target(key);
    let prepared = fixture.core.mods_delete_prepare(&target).expect("prepare");
    fixture
        .core
        .mods_delete_permanently(&request(target, &prepared.token))
}

fn assert_token_error<T: std::fmt::Debug>(result: Result<T, CoreError>) {
    assert!(matches!(mods_error(result), ModsError::DeleteToken));
}

#[test]
fn delete_without_prepare_changes_nothing() {
    let fixture = core_with_trash(COOL);
    let before = tree(&fixture);
    let target = fixture.target("Standard:Cool");
    let random = uuid::Uuid::new_v4().simple().to_string();
    assert_token_error(
        fixture
            .core
            .mods_delete_permanently(&request(target.clone(), &random)),
    );
    assert_token_error(fixture.core.mods_delete_permanently(&request(target, "")));
    assert_eq!(tree(&fixture), before);
}

#[test]
fn delete_token_bound_to_ref_and_key() {
    let fixture = core_with_trash(COOL);
    let before = tree(&fixture);
    let cool = fixture.target("Standard:Cool");
    let other = fixture.target("Standard:Other");
    let token = fixture
        .core
        .mods_delete_prepare(&cool)
        .expect("prepare")
        .token;
    assert_token_error(
        fixture
            .core
            .mods_delete_permanently(&request(other.clone(), &token)),
    );
    // The token is spent: even the right target is refused now.
    assert_token_error(
        fixture
            .core
            .mods_delete_permanently(&request(cool.clone(), &token)),
    );
    let token = fixture
        .core
        .mods_delete_prepare(&cool)
        .expect("prepare")
        .token;
    let mixed = ModTarget {
        mod_ref: cool.mod_ref.clone(),
        key: other.key.clone(),
    };
    assert_token_error(
        fixture
            .core
            .mods_delete_permanently(&request(mixed, &token)),
    );
    assert_token_error(fixture.core.mods_delete_permanently(&request(cool, &token)));
    assert_eq!(tree(&fixture), before);
}

#[test]
fn delete_token_expires() {
    let tokens = DeleteTokens::new();
    let mod_ref = ModRef::from_token("1|d|-|g0|Mods/Cool");
    let now = Instant::now();
    let token = tokens.issue(&mod_ref, "Standard:Cool", now);
    let late = now + DeleteTokens::TTL + Duration::from_secs(1);
    assert!(matches!(
        tokens.consume(&token, &mod_ref, "Standard:Cool", late),
        Err(ModsError::DeleteToken)
    ));
    let token = tokens.issue(&mod_ref, "Standard:Cool", now);
    tokens
        .consume(&token, &mod_ref, "Standard:Cool", now + DeleteTokens::TTL)
        .expect("within the TTL");
}

#[test]
fn delete_token_single_use() {
    let fixture = core_with_trash(COOL);
    let target = fixture.target("Standard:Cool");
    let token = fixture
        .core
        .mods_delete_prepare(&target)
        .expect("prepare")
        .token;
    fixture
        .core
        .mods_delete_permanently(&request(target.clone(), &token))
        .expect("first delete");
    let before = tree(&fixture);
    assert_token_error(
        fixture
            .core
            .mods_delete_permanently(&request(target, &token)),
    );
    assert_eq!(tree(&fixture), before);
}

#[test]
fn delete_refused_for_loose_archive_linked_unmanaged() {
    let fixture = core_with_trash(&[
        ("Mods/Loose.pak", DATA),
        ("Mods/Pack.zip", DATA),
        ("Mods/Plain/readme.txt", DATA),
        ("Pak.pak", DATA),
    ]);
    let before = tree(&fixture);
    let snapshot = fixture.snapshot();
    let content = &snapshot.content;
    let loose = fixture.target("Loose:Mods/Loose");
    let archive = entry_target(content, "Mods/Pack.zip");
    let unmanaged = entry_target(content, "Pak.pak");
    for (target, reason) in [
        (loose, ManageBlock::Loose),
        (archive, ManageBlock::Archive),
        (unmanaged, ManageBlock::Unmanaged),
    ] {
        let prepared = fixture.core.mods_delete_prepare(&target);
        match mods_error(prepared) {
            ModsError::ManageBlocked {
                action: ManageAction::Delete,
                reason: got,
                ..
            } => assert_eq!(got, reason),
            other => panic!("expected ManageBlocked({reason:?}), got {other:?}"),
        }
        // A token issued for the target (by another path) is still spent by the refusal.
        let token = fixture
            .core
            .delete_tokens
            .issue(&target.mod_ref, &target.key, Instant::now());
        let refused = fixture
            .core
            .mods_delete_permanently(&request(target.clone(), &token));
        assert!(matches!(
            mods_error(refused),
            ModsError::ManageBlocked { .. }
        ));
        assert_token_error(
            fixture
                .core
                .mods_delete_permanently(&request(target, &token)),
        );
    }
    assert_eq!(tree(&fixture), before);
}

#[test]
fn delete_while_running_needs_flag() {
    let fixture = core_with_trash(COOL);
    let before = tree(&fixture);
    let target = fixture.target("Standard:Cool");
    let token = fixture
        .core
        .mods_delete_prepare(&target)
        .expect("prepare")
        .token;
    fixture.running.store(true, Ordering::SeqCst);
    let refused = fixture
        .core
        .mods_delete_permanently(&request(target.clone(), &token));
    assert!(matches!(mods_error(refused), ModsError::GameRunning));
    assert_eq!(tree(&fixture), before);
    let token = fixture
        .core
        .mods_delete_prepare(&target)
        .expect("prepare")
        .token;
    let mut forced = request(target, &token);
    forced.apply_while_running = true;
    fixture
        .core
        .mods_delete_permanently(&forced)
        .expect("forced delete");
    assert!(!fixture.exists("Mods/Cool"));
}

#[test]
fn set_aside_failure_rolls_back_live_mod() {
    let fixture = core_with_trash(COOL);
    let before = tree(&fixture);
    // The folder op, then Cool.pak, Cool.ucas → -x; Cool_P.utoc (index 3) fails.
    fixture.core.inject_batch_fault(FaultPlan {
        fail_before: Some(3),
        ..FaultPlan::default()
    });
    let result = delete(&fixture, "Standard:Cool");
    assert!(
        matches!(result, Err(CoreError::Fs(FsError::Batch(_)))),
        "{result:?}"
    );
    assert_eq!(tree(&fixture), before);
}

#[test]
fn set_aside_crash_recovers_to_live() {
    let mut fixture = core_with_trash(COOL);
    let before = tree(&fixture);
    fixture.core.inject_batch_fault(FaultPlan {
        crash_after: Some(2),
        ..FaultPlan::default()
    });
    assert!(delete(&fixture, "Standard:Cool").is_err());
    assert!(
        !fixture.exists("Mods/Cool"),
        "the crash left the folder set aside"
    );
    restart(&mut fixture);
    assert_eq!(tree(&fixture), before);
    assert_eq!(
        fixture.row("Standard:Cool").state,
        remmods_core::mods::ModState::Enabled
    );
}

#[test]
fn delete_removes_folder_and_meta_entry() {
    let fixture = core_with_trash(COOL);
    fixture
        .toggle(&["Standard:Other"], false)
        .expect("disable Other");
    fixture
        .toggle(&["Standard:Cool"], false)
        .expect("disable Cool");
    fixture
        .toggle(&["Standard:Cool"], true)
        .expect("enable Cool");
    fixture
        .toggle(&["Standard:Cool"], false)
        .expect("disable Cool again");
    let meta_path = fixture.paths.data_dir.join("mod-meta.json");
    let meta = std::fs::read_to_string(&meta_path).expect("meta");
    assert!(meta.contains("Standard:Cool"));
    let save = remmods_core::profiles::ProfileSave::New {
        name: "P".to_owned(),
        description: String::new(),
        mode: remmods_core::profiles::ProfileMode::Additive,
    };
    fixture.core.profiles_save(&save).expect("save profile");
    let outcome = delete(&fixture, "Standard:Cool").expect("delete");
    assert_eq!(outcome.leftover, None);
    assert!(!fixture.exists("Mods/Cool"));
    assert!(fixture.exists("Mods/Other/Other.pak-x"));
    let meta = std::fs::read_to_string(&meta_path).expect("meta");
    assert!(!meta.contains("Standard:Cool"), "{meta}");
    assert!(meta.contains("Standard:Other"));
    let view = fixture.core.profiles_list().expect("profiles");
    let profile = &view.profiles[0];
    assert!(profile.profile.mods.contains_key("Standard:Cool"));
    assert_eq!(profile.missing, 1);
    assert!(fixture.snapshot().content.leftovers.is_empty());
}

#[test]
fn delete_logic_prefixed_row() {
    let fixture = core_with_trash(&[
        ("LogicMods/.disabled_X/X.pak-x", DATA),
        ("LogicMods/.disabled_X/X.ucas-x", DATA),
        ("LogicMods/Keep/Keep.pak", DATA),
    ]);
    delete(&fixture, "Logic:X").expect("delete");
    assert!(!fixture.exists("LogicMods/.disabled_X"));
    assert!(fixture.exists("LogicMods/Keep/Keep.pak"));
    assert!(fixture.snapshot().content.leftovers.is_empty());
}

#[test]
fn delete_leftover_directly() {
    let rel = format!("Mods/_Old_tmp_{HEX}");
    let file = format!("{rel}/Old.pak-x");
    let fixture = core_with_trash(&[(&file, DATA), ("Mods/Keep/Keep.pak", DATA)]);
    let target = entry_target(&fixture.snapshot().content, &rel);
    let prepared = fixture.core.mods_delete_prepare(&target).expect("prepare");
    assert_eq!(prepared.key, rel);
    assert_eq!(prepared.expires_in_secs, 120);
    let outcome = fixture
        .core
        .mods_delete_permanently(&request(target, &prepared.token))
        .expect("delete");
    assert_eq!(outcome.leftover, None);
    assert!(!fixture.exists(&rel));
    assert!(fixture.exists("Mods/Keep/Keep.pak"));
}

#[test]
fn plan_set_aside_table() {
    let fixture = core_with_trash(COOL);
    let row = fixture.row("Standard:Cool");
    let input = remmods_core::mods::ScanInput {
        paks: fixture.paks.clone(),
        game_view_len: mods_support::GAME_VIEW_LEN,
        paks_id: fixture.snapshot().content.paks_id.clone(),
    };
    let files = list_toggle_files(&row, &input).expect("files");
    let (ops, aside) = plan_set_aside(&row, &files, HEX).expect("plan");
    let names: Vec<(String, String, bool)> = ops
        .iter()
        .map(|op| {
            let rel = |path: &std::path::Path| {
                path.strip_prefix(&fixture.paks)
                    .expect("below Paks")
                    .to_string_lossy()
                    .replace('\\', "/")
            };
            (rel(&op.from), rel(&op.to), op.is_dir)
        })
        .collect();
    let folder = format!("Mods/_Cool_tmp_{HEX}");
    assert_eq!(
        names,
        vec![
            ("Mods/Cool".into(), folder.clone(), true),
            (
                format!("{folder}/Cool.pak"),
                format!("{folder}/Cool.pak-x"),
                false
            ),
            (
                format!("{folder}/Cool.ucas"),
                format!("{folder}/Cool.ucas-x"),
                false
            ),
            (
                format!("{folder}/Sub/Cool_P.utoc"),
                format!("{folder}/Sub/Cool_P.utoc-x"),
                false
            ),
        ]
    );
    assert_eq!(
        aside,
        fixture.paks.join("Mods").join(format!("_Cool_tmp_{HEX}"))
    );
}

#[test]
fn delete_journaled_leftover_refused() {
    let rel = format!("Mods/_Gone_tmp_{HEX}");
    let fixture = core_with_trash(&[("Mods/Gone/Gone.pak-x", DATA), ("Mods/Gone/b.txt", DATA)]);
    // A stuck journal record whose post path is the set-aside folder.
    {
        let app_dirs = [
            fixture.paths.data_dir.as_path(),
            fixture.paths.config_dir.as_path(),
        ];
        let scope = Scope::for_paks(&fixture.paks, &app_dirs).expect("scope");
        let guard = fixture.core.ops.write(scope).expect("guard");
        let mods = fixture.paks.join("Mods");
        let ops = [
            remmods_core::safe_fs::RenameOp {
                from: mods.join("Gone"),
                to: mods.join(format!("_Gone_tmp_{HEX}")),
                is_dir: true,
            },
            remmods_core::safe_fs::RenameOp {
                from: mods.join(format!("_Gone_tmp_{HEX}")).join("b.txt"),
                to: mods.join(format!("_Gone_tmp_{HEX}")).join("c.txt"),
                is_dir: false,
            },
        ];
        let options = remmods_core::safe_fs::BatchOptions {
            label: "test",
            retry: remmods_core::safe_fs::RetryMode::Never,
            fault: Some(FaultPlan {
                fail_before: Some(1),
                fail_rollback_of: Some(0),
                ..FaultPlan::default()
            }),
        };
        assert!(guard.rename_batch(&ops, &options).is_err());
    }
    let before = tree(&fixture);
    let content = fixture.snapshot().content.clone();
    let target = entry_target(&content, &rel);
    let prepared = fixture.core.mods_delete_prepare(&target).expect("prepare");
    let refused = fixture
        .core
        .mods_delete_permanently(&request(target, &prepared.token));
    match mods_error(refused) {
        ModsError::ManageBlocked {
            reason: ManageBlock::JournalPending,
            ..
        } => {}
        other => panic!("expected JournalPending, got {other:?}"),
    }
    assert_eq!(tree(&fixture), before);
    let items = repair_items(&content, 1);
    let leftover = items
        .iter()
        .find(|item| item.label == rel)
        .expect("leftover item");
    assert!(!leftover.actions.contains(&RepairAction::DeletePermanently));
}

#[test]
fn delete_partial_pair_refused_unchanged() {
    let fixture = core_with_trash(&[("Mods/A/A.pak", DATA), ("Mods/A/A.pak-x", DATA)]);
    let before = tree(&fixture);
    let result = delete(&fixture, "Standard:A");
    let target_exists = matches!(
        result,
        Err(CoreError::Fs(FsError::TargetExists { .. }))
            | Err(CoreError::Mods(ModsError::TargetExists))
            | Err(CoreError::Mods(ModsError::Fs(FsError::TargetExists { .. })))
    );
    assert!(target_exists, "{result:?}");
    assert_eq!(tree(&fixture), before);
    assert!(fixture.exists("Mods/A/A.pak"));
}

#[test]
fn single_token_from_prepare_many_works_for_one_row() {
    let fixture = core_with_trash(COOL);
    let target = fixture.target("Standard:Cool");
    let prepared = fixture
        .core
        .mods_delete_prepare_many(std::slice::from_ref(&target))
        .expect("prepare_many");
    let token = prepared.token.expect("a token");
    let outcome = fixture
        .core
        .mods_delete_permanently(&request(target, &token))
        .expect("delete");
    assert_eq!(outcome.leftover, None);
    assert!(!fixture.exists("Mods/Cool"));
    assert!(fixture.exists("Mods/Other/Other.pak"));
}
