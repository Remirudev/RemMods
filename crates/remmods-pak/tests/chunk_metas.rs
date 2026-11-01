#![allow(clippy::unwrap_used)]

use remmods_pak::fixture::TocBuilder;
use remmods_pak::{chunk_meta_size, read_toc_bytes};

const HASH_A: [u8; 20] = [0xA1; 20];
const HASH_B: [u8; 20] = [0xB2; 20];

fn hashes(bytes: &[u8]) -> Vec<(String, Option<[u8; 20]>)> {
    read_toc_bytes(bytes)
        .unwrap()
        .files
        .into_iter()
        .map(|f| (f.path, f.content_hash))
        .collect()
}

fn two_files(version: u8) -> TocBuilder {
    TocBuilder::new("../../../Polaris/Content/")
        .version(version)
        .file_with_hash("B/Two.uasset", HASH_B)
        .file_with_hash("A/One.uasset", HASH_A)
        .file("C/Zero.uasset")
}

#[test]
fn meta_size_is_33_before_v8_and_24_from_v8() {
    for version in 2..=7 {
        assert_eq!(chunk_meta_size(version), 33);
    }
    assert_eq!(chunk_meta_size(8), 24);
}

#[test]
fn hashes_follow_the_chunk_index_in_every_version() {
    for version in 2..=8 {
        assert_eq!(
            hashes(&two_files(version).build()),
            vec![
                ("Polaris/Content/A/One.uasset".to_owned(), Some(HASH_A)),
                ("Polaris/Content/B/Two.uasset".to_owned(), Some(HASH_B)),
                ("Polaris/Content/C/Zero.uasset".to_owned(), None),
            ],
            "v{version}"
        );
    }
}

#[test]
fn metas_take_exactly_entry_count_times_meta_size() {
    for version in [5, 8] {
        let builder = two_files(version).extra_chunks(2);
        let bytes = builder.build();
        let index = builder.build_index().len();
        let listing = read_toc_bytes(&bytes).unwrap();
        let metas = u64::from(listing.header.entry_count) * chunk_meta_size(version);
        let start = bytes.len() - usize::try_from(metas).unwrap();
        assert_eq!(
            &bytes[start - index..start],
            builder.build_index().as_slice()
        );
    }
}

#[test]
fn signed_container_hashes_are_found() {
    let bytes = two_files(5).signed(20).build();
    assert_eq!(hashes(&bytes)[0].1, Some(HASH_A));
}

#[test]
fn truncated_metas_give_no_hashes_but_still_list() {
    let mut bytes = two_files(5).build();
    bytes.truncate(bytes.len() - 1);
    let found = hashes(&bytes);
    assert_eq!(found.len(), 3);
    assert!(found.iter().all(|(_, hash)| hash.is_none()));
}

#[test]
fn all_zero_hash_is_unknown() {
    let bytes = TocBuilder::new("/")
        .file_with_hash("X.uasset", [0; 20])
        .build();
    assert_eq!(hashes(&bytes)[0].1, None);
}
