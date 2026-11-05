//! `StagingDir` and `StagedFile` (P4 spec §8.2 D19, D4): journaled before creation, refusals
//! for bad components and existing files, no overwrite.
#![allow(clippy::expect_used)]

mod support;

use std::io::Write;

use support::{fs_fixture, journal_entries, journal_json};

const SUFFIX: &str = "0123456789abcdef0123456789abcdef";

fn parts(rel: &[&str]) -> Vec<String> {
    rel.iter().map(|part| (*part).to_owned()).collect()
}

#[test]
fn d19_create_file_refusals() {
    let fixture = fs_fixture();
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("guard");
    let staging = guard
        .create_staging(&fixture.path("Mods"), SUFFIX)
        .expect("staging");
    for bad in [
        parts(&["..", "x.pak-x"]),
        parts(&[".", "x.pak-x"]),
        parts(&["", "x.pak-x"]),
        parts(&["a:b", "x.pak-x"]),
        parts(&["CON.pak-x"]),
        parts(&[]),
    ] {
        assert!(staging.create_file(&bad).is_err(), "{bad:?}");
    }
    let mut file = staging
        .create_file(&parts(&["Cool", "x.pak-x"]))
        .expect("file");
    file.write_all(b"data").expect("write");
    file.finish(None).expect("finish");
    assert!(
        staging.create_file(&parts(&["Cool", "x.pak-x"])).is_err(),
        "no overwrite"
    );
    assert_eq!(
        std::fs::read(staging.path().join("Cool").join("x.pak-x")).expect("read"),
        b"data"
    );
    staging
        .create_file(&parts(&["Cool", "y.pak-x"]))
        .expect("a parent this staging made is reused");
    guard.discard_staging(staging).expect("discard");
    assert!(support::names_in(&fixture.path("Mods")).is_empty());
    assert_eq!(journal_entries(&fixture.journal_path()), 0);
}

#[test]
fn d4_staging_is_journaled_before_it_exists() {
    let fixture = fs_fixture();
    let gate = fixture.gate();
    let guard = gate.write(fixture.scope()).expect("guard");
    assert!(
        guard
            .create_staging(&fixture.path("Mods"), "not-hex")
            .is_err()
    );
    assert!(
        guard
            .create_staging(&fixture.path("Mods/x"), SUFFIX)
            .is_err()
    );
    let staging = guard
        .create_staging(&fixture.path("Mods"), SUFFIX)
        .expect("staging");
    let json = journal_json(&fixture.journal_path()).expect("journal");
    assert_eq!(json["entries"][0]["label"], "install-stage");
    assert_eq!(json["entries"][0]["ops"][0]["kind"], "staging");
    assert!(guard.is_journaled(staging.path()).expect("journal"));
    let inert = staging.path().join("x.pak-x");
    guard
        .record_inert(&staging, std::slice::from_ref(&inert))
        .expect("record");
    let json = journal_json(&fixture.journal_path()).expect("journal");
    assert_eq!(
        json["entries"][0]["ops"][0]["inert"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert!(
        guard
            .record_inert(&staging, &[fixture.path("Mods/elsewhere")])
            .is_err()
    );
    assert!(
        guard.finish_staging(staging).is_err(),
        "the folder still exists"
    );
}
