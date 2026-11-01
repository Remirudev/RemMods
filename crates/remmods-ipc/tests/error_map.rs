//! One assertion per row of the P3 core → AppError table (SPEC-P3 §5.12).
#![allow(clippy::expect_used)]

use std::path::PathBuf;

use remmods_core::CoreError;
use remmods_core::mods::{ManageAction as CoreAction, ManageBlock as CoreBlock, ModsError};
use remmods_core::platform::TrashUnavailableReason as CoreReason;
use remmods_core::profiles::{ImportProblem as CoreProblem, ProfileError};
use remmods_core::safe_fs::{FsError, ScopeViolation};
use remmods_ipc::{AppError, ImportProblem, ManageAction, ManageBlock, TrashUnavailableReason};
use serde_json::{Value, json};

fn mods(error: ModsError) -> AppError {
    AppError::from(CoreError::Mods(error))
}

fn fs(error: FsError) -> AppError {
    AppError::from(CoreError::Fs(error))
}

fn profiles(error: ProfileError) -> AppError {
    AppError::from(CoreError::Profiles(error))
}

fn aside() -> PathBuf {
    PathBuf::from("Paks")
        .join("Mods")
        .join("~remmods-set_aside-1")
}

fn to_json(error: &AppError) -> Value {
    serde_json::to_value(error).expect("serialize")
}

#[test]
fn rename_unchanged_is_invalid_name_input() {
    assert_eq!(
        mods(ModsError::RenameUnchanged),
        AppError::invalid_input("name")
    );
}

#[test]
fn invalid_name_carries_the_issue_code() {
    assert_eq!(
        mods(ModsError::InvalidName {
            code: "reserved_device_name"
        }),
        AppError::InvalidName {
            issue: "reserved_device_name".into()
        }
    );
}

#[test]
fn target_exists_from_mods_and_fs() {
    assert_eq!(mods(ModsError::TargetExists), AppError::TargetExists);
    assert_eq!(
        fs(FsError::TargetExists { path: aside() }),
        AppError::TargetExists
    );
}

#[test]
fn manage_blocked_keeps_action_and_reason() {
    assert_eq!(
        mods(ModsError::ManageBlocked {
            key: "Mods/X".into(),
            action: CoreAction::Trash,
            reason: CoreBlock::Linked,
        }),
        AppError::ManageBlocked {
            action: ManageAction::Trash,
            reason: ManageBlock::Linked,
        }
    );
}

#[test]
fn trash_unavailable_size_is_a_number_or_null() {
    let from_mods = mods(ModsError::TrashUnavailable {
        reason: CoreReason::OtherDevice,
        size: Some(1024),
    });
    assert_eq!(
        from_mods,
        AppError::TrashUnavailable {
            reason: TrashUnavailableReason::OtherDevice,
            size: Some(1024.0),
        }
    );
    assert_eq!(
        to_json(&from_mods),
        json!({ "kind": "trashUnavailable", "reason": "otherDevice", "size": 1024.0 })
    );
    let from_fs = fs(FsError::TrashUnavailable {
        reason: CoreReason::NotSupported,
    });
    assert_eq!(
        to_json(&from_fs),
        json!({ "kind": "trashUnavailable", "reason": "notSupported", "size": null })
    );
}

#[test]
fn delete_token_is_invalid() {
    assert_eq!(mods(ModsError::DeleteToken), AppError::DeleteTokenInvalid);
    assert_eq!(
        to_json(&AppError::DeleteTokenInvalid),
        json!({ "kind": "deleteTokenInvalid" })
    );
}

#[test]
fn modinfo_ambiguous() {
    assert_eq!(
        mods(ModsError::ModinfoAmbiguous),
        AppError::ModinfoAmbiguous
    );
}

#[test]
fn field_too_long_names_the_field() {
    assert_eq!(
        mods(ModsError::FieldTooLong {
            field: "description"
        }),
        AppError::invalid_input("description")
    );
}

#[test]
fn changed_leftovers_are_stale() {
    assert_eq!(fs(FsError::NotAnAside { path: aside() }), AppError::Stale);
    assert_eq!(
        fs(FsError::AsideNotInert { path: aside() }),
        AppError::Stale
    );
}

#[test]
fn journaled_aside_blocks_the_delete() {
    assert_eq!(
        fs(FsError::AsideJournaled { path: aside() }),
        AppError::ManageBlocked {
            action: ManageAction::Delete,
            reason: ManageBlock::JournalPending,
        }
    );
}

#[test]
fn export_out_of_scope() {
    assert_eq!(
        fs(FsError::OutOfScope {
            path: PathBuf::from("profiles.json"),
            reason: ScopeViolation::OutsideRoots,
        }),
        AppError::OutOfScope
    );
}

#[test]
fn profile_not_found() {
    assert_eq!(profiles(ProfileError::NotFound), AppError::ProfileNotFound);
}

#[test]
fn profile_invalid_name() {
    assert_eq!(
        profiles(ProfileError::InvalidName),
        AppError::invalid_input("profileName")
    );
}

#[test]
fn profile_name_taken() {
    assert_eq!(
        profiles(ProfileError::NameTaken),
        AppError::ProfileNameTaken
    );
}

#[test]
fn profile_store_read_only() {
    assert_eq!(profiles(ProfileError::ReadOnly), AppError::ReadOnly);
}

#[test]
fn unparsable_import_names_the_problem() {
    let error = profiles(ProfileError::Unparsable(CoreProblem::WrongShape));
    assert_eq!(
        error,
        AppError::ImportFailed {
            problem: ImportProblem::WrongShape,
        }
    );
    assert_eq!(
        to_json(&error),
        json!({ "kind": "importFailed", "problem": "wrongShape" })
    );
}

#[test]
fn changed_diff_is_stale() {
    assert_eq!(profiles(ProfileError::DiffChanged), AppError::Stale);
}

#[test]
fn profile_fs_errors_map_like_fs_errors() {
    assert_eq!(profiles(ProfileError::Fs(FsError::Busy)), AppError::Busy);
}
