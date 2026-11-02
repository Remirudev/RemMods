//! Windows-only install cases (P4 spec §8.2 W-I1, W-I2): a name Win32 would rewrite is fixed
//! before it is created; a case-only replace on a case-insensitive file system.
#![cfg(windows)]
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod archive_support;
mod install_support;
mod mods_support;
mod support;

use install_support::fixture;
use remmods_core::install::ReplacedOld;

#[test]
fn w_i1_trailing_dot_is_fixed() {
    let fixture = fixture(&[]);
    let zip = fixture.zip("Dot.zip", &[("Cool./x.pak", b"p")]);
    let inspection = fixture.inspect(&zip);
    assert_eq!(inspection.candidates[0].fixed_name.as_deref(), Some("Cool"));
    fixture.start(&inspection).expect("install");
    assert_eq!(
        support::names_in(&fixture.fx.paks.join("Mods")),
        vec!["Cool"]
    );
}

#[test]
fn w_i2_case_only_replace() {
    let fixture = fixture(&[("Mods/cool/a.pak", b"old")]);
    let zip = fixture.zip("Cool.zip", &[("Cool/x.pak", b"new")]);
    let inspection = fixture.inspect(&zip);
    assert!(inspection.candidates[0].existing.is_some());
    let outcome = fixture.start(&inspection).expect("replace");
    assert!(matches!(
        outcome.installed[0].replaced,
        Some(ReplacedOld::KeptInRepair { .. })
    ));
    let names: Vec<String> = support::names_in(&fixture.fx.paks.join("Mods"))
        .into_iter()
        .filter(|name| !name.starts_with('_'))
        .collect();
    assert_eq!(names, vec!["Cool"]);
    assert_eq!(fixture.read("Mods/Cool/x.pak"), b"new");
}
