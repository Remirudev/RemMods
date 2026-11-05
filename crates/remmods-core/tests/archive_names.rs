//! P4 §8.1 N-tests: name decoding and normalization.
#![allow(clippy::expect_used)]

mod archive_support;

use archive_support::{CraftEntry, ZipCraft, sevenz_fixture, temp_dir};
use remmods_core::archive::{
    self, ArchiveFormat, EntryKind, Limits, Listing, NameEncoding, RawEntry, RawEntryName,
    decode_name, normalize,
};

fn listing(format: ArchiveFormat, name: RawEntryName) -> Listing {
    Listing {
        format,
        entries: vec![RawEntry {
            index: 0,
            name,
            kind: EntryKind::File,
            size: 1,
            crc32: None,
            mtime: None,
            encrypted: false,
            anti: false,
        }],
        duplicate_raw: false,
        solid: false,
    }
}

fn paths_of(listing: &Listing) -> Vec<Vec<String>> {
    normalize(listing, NameEncoding::Auto, &Limits::default())
        .into_iter()
        .map(|entry| entry.path)
        .collect()
}

#[test]
fn n1_backslash_separates_in_every_format() {
    let expected = vec![vec!["Cool".to_owned(), "x.pak".to_owned()]];
    let (_dir, root) = temp_dir();
    let zip = ZipCraft::new()
        .with(CraftEntry::file("Cool\\x.pak", b"x"))
        .write(&root, "x.zip");
    let reader = archive::open(&zip, &Limits::default()).expect("open zip");
    assert_eq!(paths_of(reader.listing()), expected);

    let sevenz = sevenz_fixture(&root, "x.7z", &[("Cool\\x.pak", b"x")]);
    let reader = archive::open(&sevenz, &Limits::default()).expect("open 7z");
    assert_eq!(
        reader.listing().entries[0].name,
        RawEntryName::Unicode("Cool\\x.pak".to_owned())
    );
    assert_eq!(paths_of(reader.listing()), expected);

    let rar4 = listing(
        ArchiveFormat::Rar,
        RawEntryName::Bytes(b"Cool\\x.pak".to_vec()),
    );
    assert_eq!(paths_of(&rar4), expected);
}

#[test]
fn n2_utf8_first_then_cp437() {
    let utf8 = decode_name(
        &RawEntryName::Bytes("Ä/ü.pak".as_bytes().to_vec()),
        NameEncoding::Auto,
    );
    assert_eq!(utf8.text, "Ä/ü.pak");
    assert!(!utf8.legacy && !utf8.undecodable);
    let legacy = decode_name(
        &RawEntryName::Bytes(vec![0x81, 0xE1, 0x9B]),
        NameEncoding::Auto,
    );
    assert_eq!(legacy.text, "üß¢");
    assert!(legacy.legacy && !legacy.undecodable);
}

#[test]
fn n3_shift_jis_override() {
    let sjis = vec![0x83, 0x65, 0x83, 0x58, 0x83, 0x67];
    let decoded = decode_name(&RawEntryName::Bytes(sjis), NameEncoding::ShiftJis);
    assert_eq!(decoded.text, "テスト");
    assert!(decoded.legacy && !decoded.undecodable);
    let utf8 = decode_name(
        &RawEntryName::Bytes("Ä.pak".as_bytes().to_vec()),
        NameEncoding::ShiftJis,
    );
    assert_eq!(utf8.text, "Ä.pak");
    assert!(!utf8.legacy);
    let unicode = decode_name(
        &RawEntryName::Unicode("テスト.pak".to_owned()),
        NameEncoding::ShiftJis,
    );
    assert_eq!(unicode.text, "テスト.pak");
    assert!(!unicode.legacy);
}

#[test]
fn n4_other_overrides() {
    let cases: [(NameEncoding, &[u8], &str); 5] = [
        (NameEncoding::Gbk, &[0xD6, 0xD0, 0xCE, 0xC4], "中文"),
        (NameEncoding::Big5, &[0xA4, 0xA4, 0xA4, 0xE5], "中文"),
        (NameEncoding::EucKr, &[0xC7, 0xD1, 0xB1, 0xB9], "한국"),
        (
            NameEncoding::Ibm866,
            &[0x8F, 0xE0, 0xA8, 0xA2, 0xA5, 0xE2],
            "Привет",
        ),
        (
            NameEncoding::Windows1251,
            &[0xCF, 0xF0, 0xE8, 0xE2, 0xE5, 0xF2],
            "Привет",
        ),
    ];
    for (encoding, bytes, text) in cases {
        let decoded = decode_name(&RawEntryName::Bytes(bytes.to_vec()), encoding);
        assert_eq!(decoded.text, text, "{encoding:?}");
        assert!(decoded.legacy && !decoded.undecodable, "{encoding:?}");
    }
}

#[test]
fn n5_undecodable_under_the_override() {
    let decoded = decode_name(
        &RawEntryName::Bytes(vec![0x80, 0xFF]),
        NameEncoding::ShiftJis,
    );
    assert!(decoded.undecodable && decoded.legacy);
    assert_eq!(decoded.text.chars().count(), 2);
    let replacement = decode_name(
        &RawEntryName::Unicode("bad\u{FFFD}.pak".to_owned()),
        NameEncoding::Auto,
    );
    assert!(replacement.undecodable && !replacement.legacy);
}

#[test]
fn n6_nfd_becomes_nfc() {
    let nfd = listing(
        ArchiveFormat::SevenZip,
        RawEntryName::Unicode("e\u{301}.pak".to_owned()),
    );
    let entries = normalize(&nfd, NameEncoding::Auto, &Limits::default());
    assert_eq!(entries[0].path, ["\u{E9}.pak"]);
    assert_eq!(entries[0].display, "e\u{301}.pak");
}

#[test]
fn zip_trailing_slash_with_no_data_is_a_directory() {
    let entries = normalize(
        &listing(ArchiveFormat::Zip, RawEntryName::Bytes(b"Cool\\".to_vec())),
        NameEncoding::Auto,
        &Limits::default(),
    );
    assert_eq!(entries[0].kind, EntryKind::File, "size 1 stays a file");
    let mut empty = listing(ArchiveFormat::Zip, RawEntryName::Bytes(b"Cool\\".to_vec()));
    empty.entries[0].size = 0;
    let entries = normalize(&empty, NameEncoding::Auto, &Limits::default());
    assert_eq!(entries[0].kind, EntryKind::Dir);
    assert_eq!(entries[0].path, ["Cool"]);
}
