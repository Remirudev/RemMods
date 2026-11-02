//! Spec P5 §9.4 H1–H9, H11: the environment of host programs. Cfg-free: AppImage values are judged
//! by their first byte, so these run on Windows too.

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use remmods_core::platform::{
    APPIMAGE_RUNTIME_VARS, APPRUN_VALUE_VARS, FALLBACK_PATH, HostEnvPlan, host_command_in,
    host_env_plan, resolve_cli_path,
};

const APPDIR: &str = "/tmp/.mount_x";

fn env(pairs: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
    pairs
        .iter()
        .map(|(name, value)| (OsString::from(name), OsString::from(value)))
        .collect()
}

fn os(list: &[&str]) -> Vec<OsString> {
    list.iter().map(OsString::from).collect()
}

fn set(pairs: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
    env(pairs)
}

/// The value the plan gives `name`: Some(Some) set, Some(None) removed, None untouched.
fn planned(plan: &HostEnvPlan, name: &str) -> Option<Option<OsString>> {
    if plan.remove.iter().any(|removed| removed == name) {
        return Some(None);
    }
    plan.set
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| Some(value.clone()))
}

#[test]
fn h1_without_appdir_only_own_vars_go() {
    let inherited = env(&[
        ("WEBKIT_DMABUF_RENDERER_FORCE_SHM", "1"),
        ("WEBKIT_DISABLE_DMABUF_RENDERER", "1"),
        ("PATH", "/usr/bin::/usr/bin"),
        ("APPIMAGE", "/home/u/RemMods.AppImage"),
    ]);
    let plan = host_env_plan(
        &inherited,
        &[
            "WEBKIT_DMABUF_RENDERER_FORCE_SHM",
            "__NV_DISABLE_EXPLICIT_SYNC",
        ],
    );
    assert_eq!(
        plan,
        HostEnvPlan {
            remove: os(&["WEBKIT_DMABUF_RENDERER_FORCE_SHM"]),
            set: Vec::new(),
        }
    );
}

#[test]
fn h2_appimage_runtime_and_value_vars_are_removed() {
    let mut pairs = vec![("APPDIR", APPDIR), ("HOME", "/home/u")];
    for name in APPIMAGE_RUNTIME_VARS.iter().chain(APPRUN_VALUE_VARS) {
        if *name != "APPDIR" {
            pairs.push((name, "/tmp/.mount_x/usr/lib"));
        }
    }
    let plan = host_env_plan(&env(&pairs), &[]);
    let mut expected: Vec<&str> = APPIMAGE_RUNTIME_VARS.to_vec();
    expected.extend_from_slice(APPRUN_VALUE_VARS);
    assert_eq!(plan.remove, os(&expected));
    assert!(plan.set.is_empty());

    let partial = host_env_plan(&env(&[("APPDIR", APPDIR), ("OWD", "/home/u")]), &[]);
    assert_eq!(partial.remove, os(&["APPDIR", "OWD"]));
    assert!(partial.set.is_empty());
}

#[test]
fn h3_path_loses_mount_and_empty_entries() {
    let plan = host_env_plan(
        &env(&[
            ("APPDIR", APPDIR),
            (
                "PATH",
                "/tmp/.mount_x/usr/bin:/usr/bin:/tmp/.mount_x/usr/sbin::/bin",
            ),
        ]),
        &[],
    );
    assert_eq!(planned(&plan, "PATH"), Some(Some("/usr/bin:/bin".into())));
}

#[test]
fn h4_prefix_siblings_stay_and_appdir_itself_goes() {
    let plan = host_env_plan(
        &env(&[
            ("APPDIR", APPDIR),
            ("PATH", "/tmp/.mount_x:/tmp/.mount_xy/bin:/usr/bin"),
        ]),
        &[],
    );
    assert_eq!(
        planned(&plan, "PATH"),
        Some(Some("/tmp/.mount_xy/bin:/usr/bin".into()))
    );
}

#[test]
fn h5_list_of_mount_entries_only_is_removed() {
    let plan = host_env_plan(
        &env(&[
            ("APPDIR", APPDIR),
            ("LD_LIBRARY_PATH", "/tmp/.mount_x/usr/lib:/tmp/.mount_x/lib"),
        ]),
        &[],
    );
    assert_eq!(planned(&plan, "LD_LIBRARY_PATH"), Some(None));
}

#[test]
fn h6_path_of_mount_entries_only_falls_back() {
    let plan = host_env_plan(
        &env(&[("APPDIR", APPDIR), ("PATH", "/tmp/.mount_x/usr/bin:")]),
        &[],
    );
    assert_eq!(planned(&plan, "PATH"), Some(Some(FALLBACK_PATH.into())));
}

#[test]
fn h7_duplicates_after_the_first_are_dropped() {
    let plan = host_env_plan(
        &env(&[
            ("APPDIR", APPDIR),
            (
                "XDG_DATA_DIRS",
                "/tmp/.mount_x/usr/share:/usr/share:/usr/share",
            ),
        ]),
        &[],
    );
    assert_eq!(
        planned(&plan, "XDG_DATA_DIRS"),
        Some(Some("/usr/share".into()))
    );
}

#[test]
fn unchanged_lists_are_left_alone() {
    let plan = host_env_plan(&env(&[("APPDIR", APPDIR), ("PATH", "/usr/bin:/bin")]), &[]);
    assert_eq!(planned(&plan, "PATH"), None);
    assert_eq!(plan.remove, os(&["APPDIR"]));
}

#[test]
fn h8_relative_appdir_is_no_appimage() {
    for appdir in ["squashfs-root", ""] {
        let plan = host_env_plan(
            &env(&[
                ("APPDIR", appdir),
                ("APPIMAGE", "/home/u/RemMods.AppImage"),
                ("PATH", "squashfs-root/usr/bin:/usr/bin"),
            ]),
            &[],
        );
        assert_eq!(plan, HostEnvPlan::default(), "APPDIR={appdir:?}");
    }
}

#[test]
fn h9_host_command_in_applies_the_plan_and_home() {
    let home = PathBuf::from("/home/u");
    let inherited = env(&[
        ("APPDIR", APPDIR),
        ("APPIMAGE", "/home/u/RemMods.AppImage"),
        ("PATH", "/tmp/.mount_x/usr/bin:/usr/bin"),
        ("WEBKIT_DISABLE_DMABUF_RENDERER", "1"),
        ("LANG", "C"),
    ]);
    let own = ["WEBKIT_DISABLE_DMABUF_RENDERER"];
    let plan = host_env_plan(&inherited, &own);
    let command = host_command_in("xdg-open", &inherited, &own, Some(&home));
    assert_eq!(command.get_current_dir(), Some(home.as_path()));

    let actual: BTreeMap<&OsStr, Option<&OsStr>> = command.get_envs().collect();
    let mut expected: BTreeMap<&OsStr, Option<&OsStr>> = BTreeMap::new();
    for name in &plan.remove {
        expected.insert(name.as_os_str(), None);
    }
    for (name, value) in &plan.set {
        expected.insert(name.as_os_str(), Some(value.as_os_str()));
    }
    assert_eq!(actual, expected);
    assert_eq!(
        plan,
        HostEnvPlan {
            remove: os(&["WEBKIT_DISABLE_DMABUF_RENDERER", "APPDIR", "APPIMAGE"]),
            set: set(&[("PATH", "/usr/bin")]),
        }
    );

    let homeless = host_command_in("true", &[], &[], None);
    assert_eq!(homeless.get_current_dir(), None);
    assert_eq!(homeless.get_envs().count(), 0);
}

#[test]
fn h11_resolve_cli_path_against_owd() {
    let appdir = Some(OsStr::new(APPDIR));
    let owd = Some(OsStr::new("/home/u"));
    assert_eq!(
        resolve_cli_path(Path::new("fixture"), owd, appdir),
        Path::new("/home/u").join("fixture")
    );
    let rooted = Path::new("/srv/fixture");
    assert_eq!(resolve_cli_path(rooted, owd, appdir), rooted);
    assert_eq!(
        resolve_cli_path(Path::new("fixture"), None, appdir),
        Path::new("fixture")
    );
    assert_eq!(
        resolve_cli_path(Path::new("fixture"), owd, None),
        Path::new("fixture")
    );
    assert_eq!(
        resolve_cli_path(Path::new("fixture"), owd, Some(OsStr::new("squashfs-root"))),
        Path::new("fixture")
    );
}
