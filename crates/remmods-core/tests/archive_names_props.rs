//! P4 §8.1 N7: normalized names never carry a separator, dot or drive component.

use std::collections::HashSet;

use proptest::prelude::*;
use remmods_core::archive::{
    ArchiveFormat, EntryKind, Limits, Listing, NameEncoding, RawEntry, RawEntryName, decode_name,
    entry_is_unsafe, normalize,
};

/// Bytes biased towards the characters the checks care about.
fn name_bytes() -> impl Strategy<Value = Vec<u8>> {
    let byte = prop_oneof![
        3 => prop::sample::select(vec![b'/', b'\\', b'.', b' ', b':', b'C', b'a', 0u8]),
        2 => any::<u8>(),
    ];
    prop::collection::vec(byte, 0..48)
}

fn single(name: Vec<u8>) -> Listing {
    Listing {
        format: ArchiveFormat::Zip,
        entries: vec![RawEntry {
            index: 0,
            name: RawEntryName::Bytes(name),
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

proptest! {
    #![proptest_config(ProptestConfig { failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn safe_paths_have_only_plain_components(name in name_bytes()) {
        let entries = normalize(&single(name), NameEncoding::Auto, &Limits::default());
        let entry = &entries[0];
        if !entry_is_unsafe(entry) && entry.problem.is_none() {
            prop_assert!(!entry.path.is_empty());
            for part in &entry.path {
                prop_assert!(!part.is_empty());
                prop_assert!(part != "." && part != "..");
                prop_assert!(!part.chars().all(|c| c == '.' || c == ' '));
                prop_assert!(!part.contains('/') && !part.contains('\\') && !part.contains('\0'));
            }
            let first = entry.path[0].as_bytes();
            prop_assert!(!(first.len() >= 2 && first[0].is_ascii_alphabetic() && first[1] == b':'));
        }
    }

    #[test]
    fn decoding_is_total(name in prop::collection::vec(any::<u8>(), 0..64)) {
        for encoding in [NameEncoding::Auto, NameEncoding::ShiftJis, NameEncoding::Gbk] {
            let decoded = decode_name(&RawEntryName::Bytes(name.clone()), encoding);
            if std::str::from_utf8(&name).is_err() {
                prop_assert!(decoded.legacy);
            }
        }
    }
}

#[test]
fn cp437_is_total_and_injective_over_the_high_half() {
    let mut seen = HashSet::new();
    for byte in 0x80..=0xFFu8 {
        let decoded = decode_name(&RawEntryName::Bytes(vec![byte]), NameEncoding::Auto);
        assert!(decoded.legacy);
        let mut chars = decoded.text.chars();
        let only = chars.next().expect("one char");
        assert!(chars.next().is_none());
        assert!(!only.is_ascii(), "{byte:#x}");
        assert!(seen.insert(only), "{byte:#x} repeats {only}");
    }
    assert_eq!(seen.len(), 128);
}
