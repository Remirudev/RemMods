//! `Core::mods_set_metadata`, `modinfo_json` and the meta store's new fields (spec §5.5, §8.2
//! I1, I3–I11). I2 is in `platform_linux_mods.rs`.
#![allow(clippy::expect_used)]

mod mods_support;
mod support;

use std::sync::atomic::Ordering;

use mods_support::DATA;
use mods_support::manage::{core_with_trash, mods_error, tree};
use proptest::prelude::*;
use remmods_core::mods::{
    MODINFO_FIELD_MAX_CHARS, ManageBlock, MetadataRequest, ModInfoEdit, ModMeta, ModsError,
    modinfo_json, parse_modinfo,
};

fn edit(version: &str, category: &str, description: &str, author: &str) -> ModInfoEdit {
    ModInfoEdit {
        version: version.to_owned(),
        category: category.to_owned(),
        description: description.to_owned(),
        author: author.to_owned(),
    }
}

fn set(
    fixture: &mods_support::manage::ToggleFixture,
    key: &str,
    info: ModInfoEdit,
) -> Result<remmods_core::mods::Refreshed, remmods_core::CoreError> {
    fixture.core.mods_set_metadata(&MetadataRequest {
        target: fixture.target(key),
        info,
    })
}

fn read(fixture: &mods_support::manage::ToggleFixture, rel: &str) -> Vec<u8> {
    std::fs::read(support::join_rel(&fixture.paks, rel)).expect("read")
}

fn backups(fixture: &mods_support::manage::ToggleFixture, folder: &str) -> Vec<String> {
    support::names_in(&support::join_rel(&fixture.paks, folder))
        .into_iter()
        .filter(|name| name.contains(".backup."))
        .collect()
}

const EXPECTED: &str = "{\n  \"Version\": \"2.0\",\n  \"Category\": \"Character\",\n  \"Description\": \"New\",\n  \"Author\": \"Me\"\n}";

#[test]
fn metadata_backs_up_invalid_before_write() {
    let fixture = core_with_trash(&[
        ("Mods/Cool/Cool.pak", DATA),
        ("Mods/Cool/modinfo.json", b"{ broken"),
    ]);
    set(
        &fixture,
        "Standard:Cool",
        edit("2.0", "Character", "New", "Me"),
    )
    .expect("write");
    assert_eq!(
        read(&fixture, "Mods/Cool/modinfo.json"),
        EXPECTED.as_bytes()
    );
    let saved = backups(&fixture, "Mods/Cool");
    assert_eq!(saved.len(), 1, "{saved:?}");
    assert_eq!(
        read(&fixture, &format!("Mods/Cool/{}", saved[0])),
        b"{ broken"
    );
}

#[test]
fn metadata_ambiguous_variants_refused() {
    let fixture = core_with_trash(&[
        ("Mods/Cool/Cool.pak", DATA),
        ("Mods/Cool/ModInfo.json", b"{}"),
        ("Mods/Cool/MODINFO.JSON", b"{}"),
    ]);
    if fixture
        .row("Standard:Cool")
        .path
        .join("modinfo.json")
        .exists()
    {
        return; // a case-insensitive file system cannot hold both variants
    }
    let before = tree(&fixture);
    let result = set(
        &fixture,
        "Standard:Cool",
        edit("2.0", "Character", "New", "Me"),
    );
    assert!(matches!(mods_error(result), ModsError::ModinfoAmbiguous));
    assert_eq!(tree(&fixture), before);
}

#[test]
fn metadata_writes_existing_case_variant() {
    let old = br#"{"Version":"1.0","Category":"Misc","Description":"","Author":"A"}"#;
    let fixture = core_with_trash(&[
        ("Mods/Cool/Cool.pak", DATA),
        ("Mods/Cool/ModInfo.json", old),
    ]);
    set(
        &fixture,
        "Standard:Cool",
        edit("2.0", "Character", "New", "Me"),
    )
    .expect("write");
    assert_eq!(
        read(&fixture, "Mods/Cool/ModInfo.json"),
        EXPECTED.as_bytes()
    );
    let names = support::names_in(&support::join_rel(&fixture.paks, "Mods/Cool"));
    assert_eq!(names, ["Cool.pak", "ModInfo.json"]);
}

#[test]
fn metadata_refused_for_loose_linked() {
    let fixture = core_with_trash(&[("Mods/Loose.pak", DATA)]);
    let before = tree(&fixture);
    match mods_error(set(&fixture, "Loose:Mods/Loose", edit("1", "c", "d", "a"))) {
        ModsError::ManageBlocked { reason, .. } => assert_eq!(reason, ManageBlock::Loose),
        other => panic!("expected ManageBlocked, got {other:?}"),
    }
    assert_eq!(tree(&fixture), before);
}

#[test]
fn modinfo_json_golden() {
    let cases: [(ModInfoEdit, &str); 6] = [
        (
            edit("1.2", "Character", "Hello", "Me"),
            "{\n  \"Version\": \"1.2\",\n  \"Category\": \"Character\",\n  \"Description\": \"Hello\",\n  \"Author\": \"Me\"\n}",
        ),
        (
            edit(" ", "", "\t", ""),
            "{\n  \"Version\": \"1.0\",\n  \"Category\": \"Misc\",\n  \"Description\": \"\",\n  \"Author\": \"Unknown\"\n}",
        ),
        (
            edit("<>&'+`", "\"q\"", "a\\b", "x"),
            "{\n  \"Version\": \"\\u003C\\u003E\\u0026\\u0027\\u002B\\u0060\",\n  \"Category\": \"\\u0022q\\u0022\",\n  \"Description\": \"a\\\\b\",\n  \"Author\": \"x\"\n}",
        ),
        (
            edit("a\u{8}\t\n\u{C}\r\u{1}b", "v", "d", "a"),
            "{\n  \"Version\": \"a\\b\\t\\n\\f\\r\\u0001b\",\n  \"Category\": \"v\",\n  \"Description\": \"d\",\n  \"Author\": \"a\"\n}",
        ),
        (
            edit("\u{e9}", "\u{65E5}\u{672C}", "\u{1F600}", "a"),
            "{\n  \"Version\": \"\\u00E9\",\n  \"Category\": \"\\u65E5\\u672C\",\n  \"Description\": \"\\uD83D\\uDE00\",\n  \"Author\": \"a\"\n}",
        ),
        (
            edit("\u{7F}", "x\u{2028}", "d", "a"),
            "{\n  \"Version\": \"\\u007F\",\n  \"Category\": \"x\\u2028\",\n  \"Description\": \"d\",\n  \"Author\": \"a\"\n}",
        ),
    ];
    for (info, expected) in cases {
        assert_eq!(modinfo_json(&info), expected, "{info:?}");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn modinfo_round_trip(
        version in "\\PC*[a-z]\\PC*",
        category in "\\PC*[a-z]\\PC*",
        description in "\\PC*[a-z]\\PC*",
        author in "\\PC*[a-z]\\PC*",
    ) {
        let info = edit(&version, &category, &description, &author);
        let parsed = parse_modinfo(modinfo_json(&info).as_bytes());
        prop_assert_eq!(parsed.version, version);
        prop_assert_eq!(parsed.category, category);
        prop_assert_eq!(parsed.description, description);
        prop_assert_eq!(parsed.author, author);
    }
}

#[test]
fn field_too_long_refused() {
    let fixture = core_with_trash(&[("Mods/Cool/Cool.pak", DATA)]);
    let before = tree(&fixture);
    let long = "\u{e9}".repeat(MODINFO_FIELD_MAX_CHARS + 1);
    match mods_error(set(&fixture, "Standard:Cool", edit("1", "c", &long, "a"))) {
        ModsError::FieldTooLong { field } => assert_eq!(field, "description"),
        other => panic!("expected FieldTooLong, got {other:?}"),
    }
    assert_eq!(tree(&fixture), before);
    let fits = "\u{e9}".repeat(MODINFO_FIELD_MAX_CHARS);
    set(&fixture, "Standard:Cool", edit("1", "c", &fits, "a")).expect("at the limit");
}

#[test]
fn metadata_allowed_while_running() {
    let fixture = core_with_trash(&[("Mods/Cool/Cool.pak", DATA)]);
    fixture.running.store(true, Ordering::SeqCst);
    set(
        &fixture,
        "Standard:Cool",
        edit("2.0", "Character", "New", "Me"),
    )
    .expect("write");
    assert_eq!(
        read(&fixture, "Mods/Cool/modinfo.json"),
        EXPECTED.as_bytes()
    );
    assert_eq!(fixture.row("Standard:Cool").info.author, "Me");
}

#[test]
fn mod_meta_added_source_round_trip() {
    let text = r#"{"version":1,"revision":3,"mods":{"Standard:A":{"added":"2026-11-01T12:00:00.000Z","source":"A.zip","future":{"x":1}}},"top":true}"#;
    let mut meta: ModMeta = serde_json::from_str(text).expect("parse");
    let entry = &meta.mods["Standard:A"];
    assert_eq!(entry.added.as_deref(), Some("2026-11-01T12:00:00.000Z"));
    assert_eq!(entry.source.as_deref(), Some("A.zip"));
    let value: serde_json::Value = serde_json::to_value(&meta).expect("encode");
    assert_eq!(value["mods"]["Standard:A"]["future"]["x"], 1);
    assert_eq!(value["top"], true);
    assert!(meta.rekey("Standard:A", "Standard:B"));
    assert!(meta.mods.contains_key("Standard:B") && !meta.mods.contains_key("Standard:A"));
    meta.remove("Standard:B");
    assert!(meta.mods.is_empty());
}

#[test]
fn metadata_backs_up_extra_keys() {
    let old =
        br#"{"Version":"1.0","Category":"Misc","Description":"","Author":"A","Url":"https://x"}"#;
    let fixture = core_with_trash(&[
        ("Mods/Cool/Cool.pak", DATA),
        ("Mods/Cool/modinfo.json", old),
    ]);
    set(
        &fixture,
        "Standard:Cool",
        edit("2.0", "Character", "New", "Me"),
    )
    .expect("write");
    assert_eq!(
        read(&fixture, "Mods/Cool/modinfo.json"),
        EXPECTED.as_bytes()
    );
    let saved = backups(&fixture, "Mods/Cool");
    assert_eq!(saved.len(), 1, "{saved:?}");
    assert_eq!(read(&fixture, &format!("Mods/Cool/{}", saved[0])), old);
}
