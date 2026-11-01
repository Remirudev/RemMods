// Writes crafted `.utoc` files into a TempDir; there is no safe_fs in this crate.
#![allow(clippy::disallowed_methods)]
#![allow(clippy::unwrap_used)]

mod support;

use remmods_pak::fixture::TocBuilder;
use remmods_pak::{
    PakError, directory_index_offset, parse_directory_index, parse_header, read_toc, read_toc_bytes,
};
use support::{MOUNT, NONE, latin1, paths, put_u32, raw_index, utf16_units};

fn list(builder: &TocBuilder) -> Vec<String> {
    paths(&read_toc_bytes(&builder.build()).unwrap())
}

fn raw_files(index: &[u8], entry_count: u32) -> Result<Vec<String>, PakError> {
    let index = parse_directory_index(index)?;
    Ok(index
        .files(entry_count)?
        .into_iter()
        .map(|f| f.path)
        .collect())
}

#[test]
fn lists_nested_paths_sorted() {
    let builder = TocBuilder::new(MOUNT)
        .file("Smoke/b/Z.uasset")
        .file("Smoke/A.uasset")
        .file("Other/Deep/er/X.ubulk")
        .file("Smoke/b/A.uasset")
        .file("Top.uasset");
    assert_eq!(
        list(&builder),
        [
            "Polaris/Content/Other/Deep/er/X.ubulk",
            "Polaris/Content/Smoke/A.uasset",
            "Polaris/Content/Smoke/b/A.uasset",
            "Polaris/Content/Smoke/b/Z.uasset",
            "Polaris/Content/Top.uasset",
        ]
    );
}

#[test]
fn mount_points_normalise() {
    let cases = [
        ("../../../Polaris/Content/", "Polaris/Content/X/a.uasset"),
        (
            "../../../Polaris/Content/Character/Item/",
            "Polaris/Content/Character/Item/X/a.uasset",
        ),
        ("../../../", "X/a.uasset"),
        ("/", "X/a.uasset"),
        ("../../../Polaris/Content", "Polaris/Content/X/a.uasset"),
        ("", "X/a.uasset"),
    ];
    for (mount, expected) in cases {
        let builder = TocBuilder::new(mount).file("X/a.uasset");
        let listing = read_toc_bytes(&builder.build()).unwrap();
        assert_eq!(listing.mount_point, mount);
        assert_eq!(paths(&listing), [expected], "{mount}");
    }
}

#[test]
fn backslash_in_mount_is_malformed() {
    let builder = TocBuilder::new("..\\..\\Polaris\\").file("a.uasset");
    assert!(matches!(
        read_toc_bytes(&builder.build()),
        Err(PakError::Malformed("mount"))
    ));
}

#[test]
fn latin1_names_decode() {
    let builder = TocBuilder::new(MOUNT).file("Café/Grüße_ÿ.uasset");
    let index = builder.build_index();
    assert!(index.contains(&0xE9));
    assert_eq!(list(&builder), ["Polaris/Content/Café/Grüße_ÿ.uasset"]);
}

#[test]
fn utf16_names_decode() {
    let builder = TocBuilder::new(MOUNT)
        .file("日本/Ä.uasset")
        .file("Plain.uasset")
        .utf16_strings(true);
    assert_eq!(
        list(&builder),
        [
            "Polaris/Content/Plain.uasset",
            "Polaris/Content/日本/Ä.uasset"
        ]
    );
    let mixed = TocBuilder::new(MOUNT).file("Ω/a.uasset");
    assert_eq!(list(&mixed), ["Polaris/Content/Ω/a.uasset"]);
}

#[test]
fn unpaired_surrogate_is_malformed() {
    let index = raw_index(
        MOUNT,
        &[[NONE, NONE, NONE, 0]],
        &[[0, NONE, 0]],
        &[utf16_units(&[0x61, 0xD800, 0])],
    );
    assert!(matches!(
        parse_directory_index(&index),
        Err(PakError::Malformed("utf16"))
    ));
    let unterminated = raw_index(MOUNT, &[], &[], &[utf16_units(&[0x61, 0x62])]);
    assert!(matches!(
        parse_directory_index(&unterminated),
        Err(PakError::Malformed("string"))
    ));
}

#[test]
fn signed_container_skips_signatures() {
    for hash_size in [0, 20, 64] {
        let builder = TocBuilder::new(MOUNT)
            .file("A.uasset")
            .file("B/C.uasset")
            .signed(hash_size);
        let listing = read_toc_bytes(&builder.build()).unwrap();
        assert!(
            listing
                .header
                .flags
                .has(remmods_pak::ContainerFlags::SIGNED)
        );
        assert_eq!(listing.files.len(), 2);
    }
    let mut bytes = TocBuilder::new(MOUNT).file("A.uasset").signed(20).build();
    let at =
        usize::try_from(directory_index_offset(&parse_header(&bytes).unwrap()).unwrap()).unwrap();
    for bad in [65u32, u32::MAX] {
        put_u32(&mut bytes, at, bad);
        assert!(matches!(
            read_toc_bytes(&bytes),
            Err(PakError::Malformed("signature"))
        ));
    }
}

#[test]
fn v2_v3_have_no_seed_arrays() {
    for version in [2, 3] {
        let builder = TocBuilder::new(MOUNT)
            .file("A.uasset")
            .file("B.uasset")
            .file("C.uasset")
            .version(version);
        let bytes = builder.build();
        assert_eq!(parse_header(&bytes).unwrap().perfect_hash_seeds_count, 2);
        assert_eq!(read_toc_bytes(&bytes).unwrap().files.len(), 3);
    }
}

#[test]
fn v4_has_seeds_only() {
    let mut bytes = TocBuilder::new(MOUNT)
        .file("A.uasset")
        .file("B.uasset")
        .file("C.uasset")
        .version(4)
        .build();
    put_u32(&mut bytes, 96, 7);
    let header = parse_header(&bytes).unwrap();
    assert_eq!(
        directory_index_offset(&header).unwrap(),
        144 + 4 * 22 + 2 * 4 + 3 * 12 + 32
    );
    assert_eq!(read_toc_bytes(&bytes).unwrap().files.len(), 3);
}

#[test]
fn v5_has_seeds_and_overflow() {
    let builder = TocBuilder::new(MOUNT).file("A.uasset").file("B.uasset");
    let mut bytes = builder.build();
    let overflow_at = 144 + 3 * 22 + 4;
    put_u32(&mut bytes, 96, 2);
    bytes.splice(overflow_at..overflow_at, [0u8; 8]);
    let listing = read_toc_bytes(&bytes).unwrap();
    assert_eq!(listing.header.chunks_without_perfect_hash_count, 2);
    assert_eq!(paths(&listing), list(&builder));
}

#[test]
fn root_only_index_is_empty() {
    let listing = read_toc_bytes(&TocBuilder::new(MOUNT).build()).unwrap();
    assert!(listing.files.is_empty());
    let empty = raw_index(MOUNT, &[], &[], &[]);
    assert_eq!(raw_files(&empty, 1).unwrap(), Vec::<String>::new());
}

#[test]
fn user_data_is_chunk_index() {
    let listing = read_toc_bytes(
        &TocBuilder::new(MOUNT)
            .file("c.uasset")
            .file("a.uasset")
            .file("b/x.uasset")
            .build(),
    )
    .unwrap();
    let chunks: Vec<(String, u32)> = listing
        .files
        .into_iter()
        .map(|f| (f.path, f.chunk_index))
        .collect();
    assert_eq!(
        chunks,
        [
            ("Polaris/Content/a.uasset".to_owned(), 1),
            ("Polaris/Content/b/x.uasset".to_owned(), 2),
            ("Polaris/Content/c.uasset".to_owned(), 0),
        ]
    );
}

#[test]
fn user_data_out_of_range_is_malformed() {
    let index = TocBuilder::new(MOUNT)
        .file("a.uasset")
        .file("b.uasset")
        .build_index();
    assert!(raw_files(&index, 2).is_ok());
    assert!(matches!(
        raw_files(&index, 1),
        Err(PakError::Malformed("user data"))
    ));
}

#[test]
fn trailing_bytes_are_malformed() {
    let mut index = TocBuilder::new(MOUNT).file("a.uasset").build_index();
    index.push(0);
    assert!(matches!(
        parse_directory_index(&index),
        Err(PakError::Malformed("trailing bytes"))
    ));
}

#[test]
fn negative_count_is_malformed() {
    let mut index = latin1(MOUNT);
    index.extend_from_slice(&(-1i32).to_le_bytes());
    assert!(matches!(
        parse_directory_index(&index),
        Err(PakError::Malformed("count"))
    ));
    let string = i32::MIN.to_le_bytes();
    assert!(matches!(
        parse_directory_index(&string),
        Err(PakError::Malformed("string"))
    ));
}

#[test]
fn count_larger_than_rest_is_malformed() {
    let mut index = latin1(MOUNT);
    index.extend_from_slice(&1000i32.to_le_bytes());
    index.extend_from_slice(&[0u8; 64]);
    assert!(matches!(
        parse_directory_index(&index),
        Err(PakError::Malformed("count"))
    ));
    let mut long_string = 1_000_000i32.to_le_bytes().to_vec();
    long_string.extend_from_slice(b"abc");
    assert!(matches!(
        parse_directory_index(&long_string),
        Err(PakError::Malformed("string"))
    ));
}

#[test]
fn sibling_cycle_is_malformed() {
    let index = raw_index(
        MOUNT,
        &[
            [NONE, 1, NONE, NONE],
            [0, NONE, 2, NONE],
            [1, NONE, 1, NONE],
        ],
        &[],
        &[latin1("a"), latin1("b")],
    );
    assert!(matches!(
        raw_files(&index, 1),
        Err(PakError::Malformed("cycle"))
    ));
    let back_to_root = raw_index(
        MOUNT,
        &[[NONE, 1, NONE, NONE], [0, 0, NONE, NONE]],
        &[],
        &[latin1("a")],
    );
    assert!(matches!(
        raw_files(&back_to_root, 1),
        Err(PakError::Malformed("cycle"))
    ));
}

#[test]
fn file_cycle_is_malformed() {
    let index = raw_index(
        MOUNT,
        &[[NONE, NONE, NONE, 0]],
        &[[0, 1, 0], [0, 0, 1]],
        &[latin1("a.uasset")],
    );
    assert!(matches!(
        raw_files(&index, 2),
        Err(PakError::Malformed("cycle"))
    ));
    let out_of_range = raw_index(MOUNT, &[[NONE, NONE, NONE, 5]], &[], &[]);
    assert!(matches!(
        raw_files(&out_of_range, 2),
        Err(PakError::Malformed("link"))
    ));
}

#[test]
fn name_out_of_range_is_malformed() {
    let index = raw_index(
        MOUNT,
        &[[NONE, NONE, NONE, 0]],
        &[[99, NONE, 0]],
        &[latin1("a.uasset")],
    );
    assert!(matches!(
        raw_files(&index, 1),
        Err(PakError::Malformed("name"))
    ));
}

#[test]
fn name_with_slash_or_dotdot_is_malformed() {
    for name in ["a/b", "..", ".", "", "x\\y", "a\0b"] {
        let encoded = if name.is_empty() {
            0i32.to_le_bytes().to_vec()
        } else {
            latin1(name)
        };
        let as_file = raw_index(
            MOUNT,
            &[[NONE, NONE, NONE, 0]],
            &[[0, NONE, 0]],
            std::slice::from_ref(&encoded),
        );
        assert!(
            matches!(raw_files(&as_file, 1), Err(PakError::Malformed("name"))),
            "{name:?}"
        );
        let as_dir = raw_index(
            MOUNT,
            &[[NONE, 1, NONE, NONE], [0, NONE, NONE, NONE]],
            &[],
            &[encoded],
        );
        assert!(
            matches!(raw_files(&as_dir, 1), Err(PakError::Malformed("name"))),
            "{name:?}"
        );
    }
}

#[test]
fn files_without_root_are_malformed() {
    let index = raw_index(MOUNT, &[], &[[0, NONE, 0]], &[latin1("a.uasset")]);
    assert!(matches!(
        raw_files(&index, 1),
        Err(PakError::Malformed("no root"))
    ));
}

#[test]
fn extra_chunks_without_names_are_not_listed() {
    let listing = read_toc_bytes(
        &TocBuilder::new(MOUNT)
            .file("a.uasset")
            .file("b.uasset")
            .extra_chunks(5)
            .build(),
    )
    .unwrap();
    assert_eq!(listing.header.entry_count, 8);
    assert_eq!(listing.files.len(), 2);
}

#[test]
fn read_toc_reads_only_needed_ranges() {
    let builder = TocBuilder::new(MOUNT).file("a.uasset").file("b/c.ubulk");
    let mut bytes = builder.build();
    let tail = 3 * 33;
    let len = bytes.len();
    bytes[len - tail..].fill(0xFF);
    bytes.extend_from_slice(&[0xEE; 4096]);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Garbage_Tail.utoc");
    std::fs::write(&path, &bytes).unwrap();
    assert_eq!(paths(&read_toc(&path).unwrap()), list(&builder));
}

#[test]
fn read_toc_matches_read_toc_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let cases = [
        TocBuilder::new(MOUNT)
            .file("a.uasset")
            .file("x/y/z.uexp")
            .build(),
        TocBuilder::new(MOUNT).file("a.uasset").signed(20).build(),
        TocBuilder::new(MOUNT).file("a.uasset").flags(0x02).build(),
        b"not a toc".to_vec(),
    ];
    for (i, bytes) in cases.iter().enumerate() {
        let path = dir.path().join(format!("{i}.utoc"));
        std::fs::write(&path, bytes).unwrap();
        let from_file = read_toc(&path).map_err(|e| e.to_string());
        let from_bytes = read_toc_bytes(bytes).map_err(|e| e.to_string());
        assert_eq!(from_file, from_bytes);
    }
    assert!(matches!(
        read_toc(&dir.path().join("missing.utoc")),
        Err(PakError::Io(_))
    ));
}
