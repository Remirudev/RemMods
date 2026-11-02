//! Sources (P4 spec §8.2 F1–F3): a folder installs like its zip, an archive row inside Paks
//! installs and stays, 7z and RAR install end to end.
#![allow(clippy::expect_used, clippy::disallowed_methods)]

mod archive_support;
mod install_support;
mod mods_support;
mod support;

use install_support::{fixture, zip_bytes};
use remmods_core::install::{InspectOptions, InstallSource};

const FILES: &[(&str, &[u8])] = &[
    ("x.pak", b"pak"),
    ("x.ucas", b"ucas"),
    ("opt.pak-x", b"opt"),
    ("sub/readme.txt", b"hi"),
];

fn mods_tree(fixture: &install_support::InstallFixture) -> Vec<String> {
    support::tree_listing(&fixture.fx.paks.join("Mods"))
}

#[test]
fn f1_folder_installs_like_its_zip() {
    let from_folder = fixture(&[]);
    let folder = from_folder.src.join("Cool");
    support::write_tree(&folder, FILES);
    let inspection = from_folder.inspect(&folder);
    assert_eq!(inspection.candidates[0].name, "Cool");
    from_folder.start(&inspection).expect("folder install");
    assert!(
        folder.join("x.pak").exists(),
        "the source folder is untouched"
    );

    let from_zip = fixture(&[]);
    let zip = from_zip.zip("Cool.zip", FILES);
    let inspection = from_zip.inspect(&zip);
    from_zip.start(&inspection).expect("zip install");
    assert_eq!(mods_tree(&from_folder), mods_tree(&from_zip));
}

#[test]
fn f2_archive_row_inside_paks() {
    let fixture = fixture(&[]);
    let bytes = zip_bytes(&[("Cool/x.pak", b"pak")]);
    support::write_tree(&fixture.fx.paks, &[("~mods/Cool.zip", &bytes)]);
    let snapshot = fixture.fx.snapshot();
    let row = snapshot
        .content
        .archives
        .iter()
        .find(|entry| entry.rel_path.ends_with("Cool.zip"))
        .expect("archive row");
    let inspection = fixture
        .fx
        .core
        .install_inspect(
            &InstallSource::ArchiveRow {
                mod_ref: row.mod_ref.as_str().to_owned(),
            },
            &InspectOptions::default(),
        )
        .expect("inspect archive row");
    fixture.start(&inspection).expect("install");
    assert!(fixture.exists("Mods/Cool/x.pak"));
    assert!(
        fixture.exists("~mods/Cool.zip"),
        "the archive stays in place"
    );
}

#[test]
fn f3_sevenz_and_rar_install() {
    let fixture = fixture(&[]);
    let sevenz =
        archive_support::sevenz_fixture(&fixture.src, "Seven.7z", &[("Cool\\x.pak", b"seven")]);
    let inspection = fixture.inspect(&sevenz);
    fixture.start(&inspection).expect("7z install");
    assert_eq!(fixture.read("Mods/Cool/x.pak"), b"seven");
    #[cfg(feature = "rar-builtin")]
    {
        let rar = archive_support::rar_fixture(
            &fixture.src,
            "Rar.rar",
            &[("Other\\y.pak", b"rar")],
            None,
        );
        let inspection = fixture.inspect(&rar);
        fixture.start(&inspection).expect("rar install");
        assert_eq!(fixture.read("Mods/Other/y.pak"), b"rar");
    }
}
