//! ModRef tokens, resolve_ref, read_details and shell_form (spec §4.7, §4.8).
#![allow(clippy::disallowed_methods)]

mod mods_support;
mod support;

use std::fs;

use mods_support::{DATA, fixture, input, paks_tree, row, scan};
use remmods_core::mods::{
    DETAILS_MAX_FILES, ModRef, ModsError, PayloadKind, RefKind, read_details, resolve_ref,
    shell_form,
};
use remmods_core::platform::{self, OsKind};

#[test]
fn tokens_parse_into_five_fields() {
    let parsed = ModRef::parse("1|d|803-1a2b3c|803-77|Mods/Cool|Skin").expect("parse");
    assert_eq!(parsed.kind, RefKind::Dir);
    assert_eq!(parsed.file_id.as_deref(), Some("803-1a2b3c"));
    assert_eq!(parsed.paks_id, "803-77");
    assert_eq!(parsed.components, ["Mods", "Cool|Skin"]);
    let loose = ModRef::parse("1|l|-|g2|Mods/Loose").expect("loose");
    assert_eq!(loose.kind, RefKind::Loose);
    assert_eq!(loose.file_id, None);
}

#[test]
fn malformed_tokens_are_bad_refs() {
    for token in [
        "1|d|-|803-77",
        "1|d|-||Mods/Cool",
        "2|d|-|803-77|Mods/Cool",
        "1|q|-|803-77|Mods/Cool",
        "1|d|-|803-77|Mods//Cool",
    ] {
        assert!(
            matches!(ModRef::parse(token), Err(ModsError::BadRef)),
            "{token}"
        );
    }
}

#[test]
fn from_token_keeps_the_string() {
    assert_eq!(
        ModRef::from_token("anything|at all").as_str(),
        "anything|at all"
    );
}

#[test]
fn scan_tokens_round_trip_and_resolve() {
    let (_dir, paks) = fixture();
    paks_tree(
        &paks,
        &[
            ("Mods/Cool/a.pak", DATA),
            ("Mods/Loose.pak", DATA),
            ("Mods/Pack.zip", DATA),
            ("Stray.pak", DATA),
        ],
    );
    let scan_input = input(&paks);
    let content = scan(&paks).content;
    let cool = row(&content, "Standard:Cool");
    let parsed = ModRef::parse(cool.mod_ref.as_str()).expect("parse");
    assert_eq!(parsed.components, ["Mods", "Cool"]);
    assert_eq!(parsed.paks_id, scan_input.paks_id);
    let expected = platform::fs_path(&paks.join("Mods").join("Cool")).expect("fs path");
    assert_eq!(
        resolve_ref(&scan_input, &cool.mod_ref).expect("row"),
        expected
    );
    let loose = row(&content, "Loose:Mods/Loose");
    let loose_file = platform::fs_path(&paks.join("Mods").join("Loose.pak")).expect("fs path");
    assert_eq!(
        resolve_ref(&scan_input, &loose.mod_ref).expect("loose"),
        loose_file
    );
    resolve_ref(&scan_input, &content.archives[0].mod_ref).expect("archive");
    resolve_ref(&scan_input, &content.unmanaged[0].mod_ref).expect("unmanaged");
}

#[test]
fn renamed_or_removed_entries_are_stale() {
    let (_dir, paks) = fixture();
    paks_tree(
        &paks,
        &[("Mods/Cool/a.pak", DATA), ("Mods/Gone/a.pak", DATA)],
    );
    let scan_input = input(&paks);
    let content = scan(&paks).content;
    let mods = paks.join("Mods");
    fs::rename(mods.join("Gone"), mods.join("Moved")).expect("rename");
    let gone = row(&content, "Standard:Gone");
    assert!(matches!(
        resolve_ref(&scan_input, &gone.mod_ref),
        Err(ModsError::Stale)
    ));
    // A replaced folder keeps its name but not its identity. Renaming (never delete + recreate)
    // keeps the old inode alive, so the new folder cannot get it back.
    fs::rename(mods.join("Cool"), mods.join("Cool-old")).expect("rename");
    paks_tree(&paks, &[("Mods/Cool/b.pak", DATA)]);
    let cool = row(&content, "Standard:Cool");
    assert!(matches!(
        resolve_ref(&scan_input, &cool.mod_ref),
        Err(ModsError::Stale)
    ));
}

#[test]
fn a_different_file_id_is_stale() {
    let (_dir, paks) = fixture();
    paks_tree(&paks, &[("Mods/Cool/a.pak", DATA)]);
    let scan_input = input(&paks);
    let forged = ModRef::from_token(format!("1|d|ffff-ffff|{}|Mods/Cool", scan_input.paks_id));
    assert!(matches!(
        resolve_ref(&scan_input, &forged),
        Err(ModsError::Stale)
    ));
}

#[test]
fn tokens_do_not_cross_paks_folders() {
    let (_dir_a, paks_a) = fixture();
    let (_dir_b, paks_b) = fixture();
    paks_tree(&paks_a, &[("Mods/Loose.pak", DATA)]);
    paks_tree(&paks_b, &[("Mods/Loose.pak", DATA)]);
    let content = scan(&paks_a).content;
    let token = &row(&content, "Loose:Mods/Loose").mod_ref;
    assert!(matches!(
        resolve_ref(&input(&paks_b), token),
        Err(ModsError::Stale)
    ));
}

#[test]
fn details_list_payload_sorted_and_capped() {
    let (_dir, paks) = fixture();
    paks_tree(
        &paks,
        &[
            ("Mods/M/b.pak", DATA),
            ("Mods/M/A.ucas-x", DATA),
            ("Mods/M/sub/c.sig", DATA),
            ("Mods/M/readme.txt", DATA),
            ("Mods/M/modinfo.json", b"{}"),
        ],
    );
    let scan_input = input(&paks);
    let content = scan(&paks).content;
    let details = read_details(&scan_input, &row(&content, "Standard:M").mod_ref).expect("details");
    let files: Vec<(&str, PayloadKind, bool)> = details
        .files
        .iter()
        .map(|f| (f.rel_path.as_str(), f.kind, f.live))
        .collect();
    assert_eq!(
        files,
        [
            ("A.ucas-x", PayloadKind::Ucas, false),
            ("b.pak", PayloadKind::Pak, true),
            ("sub/c.sig", PayloadKind::Sig, true),
        ]
    );
    assert_eq!(details.other_files, 2);
    assert!(!details.truncated);

    let many: Vec<(String, &[u8])> = (0..=DETAILS_MAX_FILES)
        .map(|i| (format!("Mods/Many/f{i:04}.pak"), DATA))
        .collect();
    let many: Vec<(&str, &[u8])> = many.iter().map(|(p, d)| (p.as_str(), *d)).collect();
    paks_tree(&paks, &many);
    let content = scan(&paks).content;
    let details =
        read_details(&scan_input, &row(&content, "Standard:Many").mod_ref).expect("details");
    assert!(details.truncated);
    assert_eq!(details.files.len(), DETAILS_MAX_FILES);
    let archive = &content.archives;
    assert!(archive.is_empty());
    let file_token = ModRef::from_token(format!("1|f|-|{}|Mods/M/b.pak", scan_input.paks_id));
    assert!(matches!(
        read_details(&scan_input, &file_token),
        Err(ModsError::BadRef)
    ));
}

#[test]
fn shell_form_refuses_names_win32_rewrites() {
    let (_dir, paks) = fixture();
    paks_tree(
        &paks,
        &[
            ("Mods/Foo./a.pak", DATA),
            ("Mods/Foo /b.pak", DATA),
            ("Mods/CON.pak/c.pak", DATA),
            ("Mods/Cool/d.pak", DATA),
            ("Mods/Foo/e.pak", DATA),
        ],
    );
    let scan_input = input(&paks);
    let content = scan(&paks).content;
    for key in ["Standard:Foo.", "Standard:Foo ", "Standard:CON.pak"] {
        let token = &row(&content, key).mod_ref;
        assert!(
            matches!(
                shell_form(&scan_input, token, OsKind::Windows),
                Err(ModsError::NotShellSafe)
            ),
            "{key}"
        );
        assert_eq!(
            shell_form(&scan_input, token, OsKind::Linux).expect("linux"),
            resolve_ref(&scan_input, token).expect("resolve")
        );
    }
    shell_form(
        &scan_input,
        &row(&content, "Standard:Cool").mod_ref,
        OsKind::Windows,
    )
    .expect("plain name");
    let dotted =
        read_details(&scan_input, &row(&content, "Standard:Foo.").mod_ref).expect("details");
    let names: Vec<&str> = dotted.files.iter().map(|f| f.rel_path.as_str()).collect();
    assert_eq!(names, ["a.pak"]);
}
