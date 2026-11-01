//! Linux-only scan cases: non-UTF-8 names, case-variant roots, symlinks, unreadable folders,
//! read-only trees and the warm-scan budget (spec §8.1).
#![cfg(target_os = "linux")]
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod mods_support;
mod support;

use std::ffi::OsStr;
use std::fs::{self, Permissions};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::Path;
use std::time::{Duration, Instant};

use mods_support::{
    DATA, exercise_read_only, fixture, input, listing, paks_tree, row, scan, smoke_tree,
};
use remmods_core::mods::{
    ModIssue, ModState, ModsError, ToggleBlock, UnmanagedKind, resolve_ref, scan_paks, shell_form,
};
use remmods_core::platform::OsKind;

fn has(issues: &[ModIssue], wanted: &ModIssue) -> bool {
    issues.contains(wanted)
}

/// Permission bits mean nothing to root; such runs skip the permission tests.
fn permissions_enforced(probe: &Path) -> bool {
    fs::set_permissions(probe, Permissions::from_mode(0o000)).expect("chmod probe");
    let enforced = fs::read_dir(probe).is_err();
    fs::set_permissions(probe, Permissions::from_mode(0o755)).expect("restore probe");
    enforced
}

#[test]
fn non_utf8_names_are_escaped_and_flagged() {
    let (_dir, paks) = fixture();
    let bad = paks.join("Mods").join(OsStr::from_bytes(b"bad\xFFname"));
    fs::create_dir_all(&bad).expect("create");
    fs::write(bad.join("a.pak"), DATA).expect("pak");
    fs::write(bad.join(OsStr::from_bytes(b"odd\xFE.txt")), DATA).expect("odd file");
    let scan_input = input(&paks);
    let content = scan(&paks).content;
    let found = row(&content, r"Standard:bad\xFFname");
    assert!(has(&found.issues, &ModIssue::NonUnicodeName));
    assert!(has(&found.issues, &ModIssue::NonUnicodeFiles { count: 1 }));
    assert_eq!(
        resolve_ref(&scan_input, &found.mod_ref).expect("resolve"),
        bad
    );
    assert!(matches!(
        shell_form(&scan_input, &found.mod_ref, OsKind::Windows),
        Err(ModsError::NotShellSafe)
    ));
}

#[test]
fn case_variant_roots_are_ambiguous() {
    let (_dir, paks) = fixture();
    paks_tree(
        &paks,
        &[("Mods/Cool/a.pak", DATA), ("mods/Cool/a.pak", DATA)],
    );
    let content = scan(&paks).content;
    assert_eq!(content.rows.len(), 2);
    assert!(content.rows.iter().all(|r| r.ambiguous));
    let variant = content
        .roots
        .iter()
        .find(|r| r.name == "mods")
        .expect("mods root");
    assert!(variant.case_variant);
    let literal = content
        .roots
        .iter()
        .find(|r| r.name == "Mods")
        .expect("Mods root");
    assert!(!literal.case_variant);
    let in_variant = content.rows.iter().find(|r| r.root == "mods").expect("row");
    assert!(has(&in_variant.issues, &ModIssue::CaseVariantRoot));
}

#[test]
fn loose_files_in_case_variant_roots_share_a_key() {
    let (_dir, paks) = fixture();
    paks_tree(&paks, &[("Mods/X.pak", DATA), ("mods/X.pak", DATA)]);
    let content = scan(&paks).content;
    assert_eq!(content.rows.len(), 2);
    for loose in &content.rows {
        assert_eq!(loose.key, "Loose:Mods/X");
        assert!(loose.ambiguous);
    }
}

#[test]
fn case_collisions_in_one_root() {
    let (_dir, paks) = fixture();
    paks_tree(
        &paks,
        &[("Mods/Cool/a.pak", DATA), ("Mods/cool/b.pak", DATA)],
    );
    let content = scan(&paks).content;
    assert!(has(
        &row(&content, "Standard:Cool").issues,
        &ModIssue::CaseCollision {
            others: vec!["cool".to_owned()]
        }
    ));
}

#[test]
fn linked_root_and_linked_mod() {
    let (_dir, paks) = fixture();
    let elsewhere = paks.parent().expect("parent").join("elsewhere");
    paks_tree(
        &elsewhere,
        &[("RootMod/a.pak", DATA), ("Target/b.pak-x", DATA)],
    );
    symlink(&elsewhere, paks.join("Mods")).expect("link root");
    paks_tree(&paks, &[("~mods/", b"")]);
    symlink(elsewhere.join("Target"), paks.join("~mods").join("Linked")).expect("link mod");
    let content = scan(&paks).content;
    let mods_root = content
        .roots
        .iter()
        .find(|r| r.name == "Mods")
        .expect("root");
    assert!(mods_root.is_link);
    assert!(has(
        &row(&content, "Standard:RootMod").issues,
        &ModIssue::LinkedRoot
    ));
    let linked = row(&content, "Legacy:Linked");
    assert!(has(&linked.issues, &ModIssue::LinkedMod));
    assert_eq!(linked.state, ModState::Disabled);
    assert_eq!(linked.toggle_block, Some(ToggleBlock::Linked));
    assert_eq!(row(&content, "Standard:RootMod").toggle_block, None);
}

#[test]
fn links_inside_mods_are_reported_not_followed() {
    let (_dir, paks) = fixture();
    let elsewhere = paks.parent().expect("parent").join("elsewhere");
    paks_tree(&elsewhere, &[("Extra/x.pak", DATA)]);
    paks_tree(
        &paks,
        &[
            ("Mods/Mixed/a.pak", DATA),
            ("Mods/OnlyLinked/readme.txt", DATA),
        ],
    );
    symlink(
        elsewhere.join("Extra"),
        paks.join("Mods").join("Mixed").join("Extra"),
    )
    .expect("link");
    symlink(
        elsewhere.join("Extra"),
        paks.join("Mods").join("OnlyLinked").join("Extra"),
    )
    .expect("link");
    let content = scan(&paks).content;
    let mixed = row(&content, "Standard:Mixed");
    assert!(has(&mixed.issues, &ModIssue::ContainsLink));
    assert_eq!(mixed.payload.live_pak, 1);
    assert_eq!(mixed.state, ModState::Enabled);
    assert_eq!(mixed.toggle_block, None);
    let only_linked = row(&content, "Standard:OnlyLinked");
    assert_eq!(only_linked.state, ModState::Unknown);
    assert_eq!(only_linked.toggle_block, Some(ToggleBlock::UnknownState));
    let linked: Vec<(&str, u32)> = content
        .unmanaged
        .iter()
        .filter(|e| e.kind == UnmanagedKind::LinkedFolder)
        .map(|e| (e.rel_path.as_str(), e.live_files))
        .collect();
    assert_eq!(
        linked,
        [("Mods/Mixed/Extra", 0), ("Mods/OnlyLinked/Extra", 0)]
    );
}

#[test]
fn unreadable_subfolder_is_reported() {
    let (_dir, paks) = fixture();
    paks_tree(
        &paks,
        &[("Mods/Locked/a.pak", DATA), ("Mods/Locked/sub/b.pak", DATA)],
    );
    let sub = paks.join("Mods").join("Locked").join("sub");
    if !permissions_enforced(&sub) {
        return;
    }
    fs::set_permissions(&sub, Permissions::from_mode(0o000)).expect("chmod");
    let content = scan_paks(&input(&paks)).map(|out| out.content);
    fs::set_permissions(&sub, Permissions::from_mode(0o755)).expect("restore");
    let content = content.expect("scan");
    let locked = row(&content, "Standard:Locked");
    assert_eq!(locked.size, None);
    assert!(
        locked
            .issues
            .iter()
            .any(|issue| matches!(issue, ModIssue::Unreadable { .. }))
    );
    assert_eq!(locked.state, ModState::Enabled);
}

/// Restores writable modes so the TempDir can be removed.
struct ModeGuard<'a>(&'a Path);

impl Drop for ModeGuard<'_> {
    fn drop(&mut self) {
        set_modes(self.0, 0o755, 0o644);
    }
}

fn set_modes(root: &Path, dir_mode: u32, file_mode: u32) {
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        // Directories are opened before they are locked, so a 0555 tree stays walkable.
        let _ = fs::set_permissions(&dir, Permissions::from_mode(0o755));
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                match entry.file_type() {
                    Ok(kind) if kind.is_dir() => dirs.push(path),
                    Ok(kind) if kind.is_file() => {
                        let _ = fs::set_permissions(&path, Permissions::from_mode(file_mode));
                    }
                    _ => {}
                }
            }
        }
        let _ = fs::set_permissions(&dir, Permissions::from_mode(dir_mode));
    }
}

#[test]
fn read_only_tree_scans_without_writes() {
    let (_home_dir, home) = support::temp_root();
    let (_app_dir, app_root) = support::temp_root();
    smoke_tree(&home.join(".local").join("share").join("Steam"));
    if !permissions_enforced(&home.join(".local")) {
        return;
    }
    let before = listing(&home);
    let _guard = ModeGuard(&home);
    set_modes(&home, 0o555, 0o444);
    exercise_read_only(&home, &app_root);
    assert_eq!(listing(&home), before);
}

#[test]
fn perf_warm_scan_300_mods() {
    let (_dir, paks) = fixture();
    for index in 0..300 {
        let name = format!("Mod{index:03}");
        let files: Vec<(String, &[u8])> = ["pak", "ucas", "utoc"]
            .iter()
            .map(|ext| (format!("Mods/{name}/{name}.{ext}"), DATA))
            .chain([(
                format!("Mods/{name}/modinfo.json"),
                br#"{"Version":"1.2","Author":"Perf"}"#.as_slice(),
            )])
            .collect();
        let files: Vec<(&str, &[u8])> = files.iter().map(|(p, d)| (p.as_str(), *d)).collect();
        paks_tree(&paks, &files);
    }
    let scan_input = input(&paks);
    let rows = scan_paks(&scan_input)
        .expect("cold scan")
        .content
        .rows
        .len();
    assert_eq!(rows, 300);
    let best = (0..3)
        .map(|_| {
            let started = Instant::now();
            scan_paks(&scan_input).expect("warm scan");
            started.elapsed()
        })
        .min()
        .unwrap_or(Duration::MAX);
    let limit_ms = if cfg!(debug_assertions) { 250 } else { 50 };
    assert!(
        best < Duration::from_millis(limit_ms),
        "warm scan took {best:?}, limit {limit_ms} ms"
    );
}
