//! Linux-only bulk cases (P3b §5.7 L1–L5): real moves into a TempDir trash, a cross-device
//! row, a read-only folder and a linked row.
#![cfg(target_os = "linux")]
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod mods_support;
mod support;

use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use mods_support::DATA;
use mods_support::manage::{ToggleFixture, core_with_trash, trash_dir, tree};
use remmods_core::CoreError;
use remmods_core::mods::{
    BulkDeleteRequest, BulkOutcome, BulkRowResult, BulkTrashRequest, ManageBlock, ModTarget,
    ModsError, RepairKind, repair_items,
};
use remmods_core::platform::{TrashKind, TrashUnavailableReason};
use remmods_core::safe_fs::FaultPlan;

const ABC: &[(&str, &[u8])] = &[
    ("Mods/A/A.pak", DATA),
    ("Mods/B/B.pak", DATA),
    ("Mods/B/Sub/x.txt", DATA),
    ("Mods/C/C.pak", DATA),
];

fn targets(fixture: &ToggleFixture, keys: &[&str]) -> Vec<ModTarget> {
    keys.iter().map(|key| fixture.target(key)).collect()
}

fn trash_many(fixture: &ToggleFixture, targets: Vec<ModTarget>) -> BulkOutcome {
    fixture
        .core
        .mods_trash_many(&BulkTrashRequest {
            targets,
            apply_while_running: false,
        })
        .expect("bulk trash")
}

fn chmod(path: &Path, mode: u32) {
    fs::set_permissions(path, Permissions::from_mode(mode)).expect("chmod");
}

/// Permission bits mean nothing to root; such runs skip the permission test.
fn permissions_enforced(probe: &Path) -> bool {
    chmod(probe, 0o555);
    let enforced = fs::write(probe.join("probe"), b"").is_err();
    let _ = fs::remove_file(probe.join("probe"));
    chmod(probe, 0o755);
    enforced
}

#[test]
fn bulk_trash_moves_rows_to_home_trash() {
    let fixture = core_with_trash(ABC);
    let outcome = trash_many(
        &fixture,
        targets(&fixture, &["Standard:A", "Standard:B", "Standard:C"]),
    );
    for row in &outcome.rows {
        assert!(
            matches!(
                row.result,
                BulkRowResult::Trashed {
                    items: 1,
                    kind: TrashKind::Home
                }
            ),
            "{row:?}"
        );
    }
    let info = trash_dir(&fixture).join("info");
    assert_eq!(
        support::names_in(&info),
        ["A.trashinfo", "B.trashinfo", "C.trashinfo"]
    );
    assert!(
        trash_dir(&fixture)
            .join("files")
            .join("B")
            .join("Sub")
            .join("x.txt")
            .is_file()
    );
    let snapshot = outcome.refreshed.expect("refreshed").snapshot;
    assert!(snapshot.content.rows.is_empty());
}

#[test]
fn bulk_trash_mixed_unavailable_row() {
    let fixture = core_with_trash(ABC);
    let b_before = support::tree_listing(&fixture.paks.join("Mods").join("B"));
    fixture.core.inject_batch_faults(vec![
        None,
        Some(FaultPlan {
            fail_before: Some(0),
            raw_os_error: Some(18), // EXDEV
            ..FaultPlan::default()
        }),
        None,
    ]);
    let outcome = trash_many(
        &fixture,
        targets(&fixture, &["Standard:A", "Standard:B", "Standard:C"]),
    );
    assert!(matches!(
        outcome.rows[0].result,
        BulkRowResult::Trashed { .. }
    ));
    match &outcome.rows[1].result {
        BulkRowResult::TrashUnavailable { reason, size } => {
            assert_eq!(*reason, TrashUnavailableReason::OtherDevice);
            assert!(size.is_some());
        }
        other => panic!("expected TrashUnavailable, got {other:?}"),
    }
    assert!(matches!(
        outcome.rows[2].result,
        BulkRowResult::Trashed { .. }
    ));
    assert_eq!(
        support::tree_listing(&fixture.paks.join("Mods").join("B")),
        b_before
    );
    assert!(!fixture.exists("Mods/A"));
    assert!(!fixture.exists("Mods/C"));
}

#[test]
fn bulk_trash_loose_group_and_archive() {
    let fixture = core_with_trash(&[
        ("Mods/Loose.pak", DATA),
        ("Mods/Loose.ucas", DATA),
        ("Mods/Loose.utoc", DATA),
        ("Mods/Pack.zip", DATA),
    ]);
    let content = fixture.snapshot().content.clone();
    let archive = mods_support::manage::entry_target(&content, "Mods/Pack.zip");
    let loose = fixture.target("Loose:Mods/Loose");
    let outcome = trash_many(&fixture, vec![loose, archive]);
    let items: Vec<u32> = outcome
        .rows
        .iter()
        .map(|row| match row.result {
            BulkRowResult::Trashed {
                items,
                kind: TrashKind::Home,
            } => items,
            ref other => panic!("expected Trashed, got {other:?}"),
        })
        .collect();
    assert_eq!(items, [3, 1]);
    assert_eq!(
        support::names_in(&trash_dir(&fixture).join("files")),
        ["Loose.pak", "Loose.ucas", "Loose.utoc", "Pack.zip"]
    );
    assert_eq!(
        support::names_in(&fixture.paks.join("Mods")),
        Vec::<String>::new()
    );
}

#[test]
fn bulk_delete_removal_failure_leaves_leftover() {
    let fixture = core_with_trash(ABC);
    let sub = fixture.paks.join("Mods").join("B").join("Sub");
    if !permissions_enforced(&sub) {
        return;
    }
    chmod(&sub, 0o555);
    let rows = targets(&fixture, &["Standard:A", "Standard:B", "Standard:C"]);
    let token = fixture
        .core
        .mods_delete_prepare_many(&rows)
        .expect("prepare")
        .token
        .expect("token");
    let outcome = fixture.core.mods_delete_many(&BulkDeleteRequest {
        targets: rows,
        token,
        apply_while_running: false,
    });
    let snapshot = fixture.snapshot();
    let leftover = snapshot
        .content
        .leftovers
        .first()
        .expect("a leftover")
        .rel_path
        .clone();
    chmod(&fixture.paks.join(&leftover).join("Sub"), 0o755);
    let outcome = outcome.expect("bulk delete");
    assert!(matches!(
        outcome.rows[0].result,
        BulkRowResult::Deleted { leftover: None }
    ));
    match &outcome.rows[1].result {
        BulkRowResult::Deleted {
            leftover: Some(reported),
        } => assert_eq!(*reported, leftover),
        other => panic!("expected Deleted with a leftover, got {other:?}"),
    }
    assert!(matches!(
        outcome.rows[2].result,
        BulkRowResult::Deleted { leftover: None }
    ));
    assert!(leftover.starts_with("Mods/_B_tmp_"), "{leftover}");
    assert!(snapshot.content.rows.is_empty());
    assert!(snapshot.content.unmanaged.is_empty(), "no live payload");
    let repair = repair_items(&snapshot.content, snapshot.revision);
    assert!(
        repair
            .iter()
            .any(|item| item.label == leftover && item.kind == RepairKind::Leftover)
    );
}

#[test]
fn bulk_rows_refuse_linked_row() {
    let fixture = core_with_trash(&[("Mods/Keep/Keep.pak", DATA)]);
    support::write_tree(&fixture.root, &[("elsewhere/Linked/Linked.pak", DATA)]);
    std::os::unix::fs::symlink(
        fixture.root.join("elsewhere").join("Linked"),
        fixture.paks.join("Mods").join("Linked"),
    )
    .expect("symlink");
    let before = tree(&fixture);
    let linked = fixture.target("Standard:Linked");
    let outcome = fixture
        .core
        .mods_delete_prepare_many(std::slice::from_ref(&linked))
        .expect("prepare");
    assert!(outcome.token.is_none());
    assert!(matches!(
        outcome.rows[0].refusal,
        Some(ModsError::ManageBlocked {
            reason: ManageBlock::Linked,
            ..
        })
    ));
    let outcome = trash_many(&fixture, vec![linked]);
    match &outcome.rows[0].result {
        BulkRowResult::Failed(CoreError::Mods(ModsError::ManageBlocked {
            reason: ManageBlock::Linked,
            ..
        })) => {}
        other => panic!("expected ManageBlocked(Linked), got {other:?}"),
    }
    assert_eq!(tree(&fixture), before);
}
