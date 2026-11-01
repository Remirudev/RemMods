mod support;

use std::path::{Path, PathBuf};

use remmods_core::safe_fs::{FsError, Scope, ScopeViolation};

fn violation(result: Result<(), FsError>) -> Option<ScopeViolation> {
    match result {
        Ok(()) => None,
        Err(FsError::OutOfScope { reason, .. }) => Some(reason),
        Err(other) => panic!("unexpected error {other}"),
    }
}

#[test]
fn relative_paths_are_rejected() {
    let fixture = support::fs_fixture();

    let result = fixture.scope().check(Path::new("Mods/Cool/Cool.pak"));

    assert_eq!(violation(result), Some(ScopeViolation::NotAbsolute));
}

#[test]
fn dot_components_are_rejected() {
    let fixture = support::fs_fixture();
    let scope = fixture.scope();
    let mods = fixture.path("Mods");

    let mut dotted = mods.as_os_str().to_os_string();
    dotted.push("/./Cool.pak");
    let mut dotdot = mods.as_os_str().to_os_string();
    dotdot.push("/../Mods/Cool.pak");

    assert_eq!(
        violation(scope.check(&PathBuf::from(dotted))),
        Some(ScopeViolation::DotComponent)
    );
    assert_eq!(
        violation(scope.check(&PathBuf::from(dotdot))),
        Some(ScopeViolation::DotComponent)
    );
}

#[test]
fn paths_outside_the_roots_are_rejected() {
    let fixture = support::fs_fixture();
    let scope = fixture.scope();

    assert_eq!(
        violation(scope.check(&fixture.paks.join("pakchunk0-Windows.pak"))),
        Some(ScopeViolation::OutsideRoots)
    );
    assert_eq!(
        violation(scope.check(&fixture.root.join("elsewhere.pak"))),
        Some(ScopeViolation::OutsideRoots)
    );
}

#[test]
fn a_root_itself_is_not_a_mod_path() {
    let fixture = support::fs_fixture();

    let result = fixture.scope().check(&fixture.path("LogicMods"));

    assert_eq!(violation(result), Some(ScopeViolation::IsRoot));
}

#[test]
fn paths_below_each_root_are_in_scope() {
    let fixture = support::fs_fixture();
    let scope = fixture.scope();

    for rel in [
        "Mods/Cool/Cool.pak",
        "~mods/a.pak",
        "LogicMods/X/X.pak",
        "Mods/new.pak",
    ] {
        assert!(scope.check(&fixture.path(rel)).is_ok(), "{rel}");
    }
}

#[test]
fn for_paks_picks_every_case_variant_of_a_root() {
    let (_dir, root) = support::temp_root();
    let paks = root.join("Paks");
    support::write_tree(
        &paks,
        &[("mods/", b""), ("LOGICMODS/", b""), ("Other/", b"")],
    );
    let scope = Scope::for_paks(&paks, &[]).expect("scope");

    assert!(scope.check(&paks.join("mods").join("a.pak")).is_ok());
    assert!(scope.check(&paks.join("LOGICMODS").join("X")).is_ok());
    assert_eq!(
        violation(scope.check(&paks.join("Other").join("a.pak"))),
        Some(ScopeViolation::OutsideRoots)
    );
}

#[test]
fn app_dirs_pass_check_app_only_and_mod_paths_check_only() {
    let fixture = support::fs_fixture();
    let scope = fixture.scope();
    let meta = fixture.app.data_dir.join("mod-meta.json");
    let pak = fixture.path("Mods/Cool/Cool.pak");

    assert!(scope.check_app(&meta).is_ok());
    assert!(
        scope
            .check_app(&fixture.app.config_dir.join("x.json"))
            .is_ok()
    );
    assert_eq!(
        violation(scope.check(&meta)),
        Some(ScopeViolation::OutsideRoots)
    );
    assert_eq!(
        violation(scope.check_app(&pak)),
        Some(ScopeViolation::OutsideRoots)
    );
}

#[test]
fn a_missing_root_is_skipped() {
    let fixture = support::fs_fixture();
    let missing = fixture.root.join("NoSuchRoot");
    let scope = Scope::from_roots(&[missing.clone(), fixture.path("Mods")], &[]).expect("scope");

    assert_eq!(
        violation(scope.check(&missing.join("a.pak"))),
        Some(ScopeViolation::OutsideRoots)
    );
    assert!(scope.check(&fixture.path("Mods/a.pak")).is_ok());
}

#[test]
fn a_missing_paks_folder_has_no_mod_roots() {
    let (_dir, root) = support::temp_root();
    let paks = root.join("Paks");

    let scope = Scope::for_paks(&paks, &[]).expect("scope");

    assert_eq!(
        violation(scope.check(&paks.join("Mods").join("a.pak"))),
        Some(ScopeViolation::OutsideRoots)
    );
}
