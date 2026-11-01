mod support;

use remmods_pak::fixture::TocBuilder;
use remmods_pak::{
    ContainerFlags, MAX_DIRECTORY_INDEX_BYTES, PakError, TocHeader, directory_index_offset,
    parse_header, read_toc_bytes,
};
use support::{MOUNT, put_u32};

fn sample() -> Vec<u8> {
    TocBuilder::new(MOUNT)
        .file("Smoke/A.uasset")
        .file("Smoke/B.uasset")
        .build()
}

#[test]
fn parses_v5_header_fields() {
    let builder = TocBuilder::new(MOUNT).file("a/b.uasset").file("c.uasset");
    let header = parse_header(&builder.build()).unwrap();
    assert_eq!(header.version, 5);
    assert_eq!(header.header_size, 144);
    assert_eq!(header.entry_count, 3);
    assert_eq!(header.compressed_block_entry_count, 2);
    assert_eq!(header.compressed_block_entry_size, 12);
    assert_eq!(header.compression_method_name_count, 1);
    assert_eq!(header.compression_method_name_length, 32);
    assert_eq!(header.compression_block_size, 0x1_0000);
    assert_eq!(
        header.directory_index_size,
        u32::try_from(builder.build_index().len()).unwrap()
    );
    assert_eq!(header.partition_count, 1);
    assert_eq!(header.container_id, 0x0123_4567_89AB_CDEF);
    assert_eq!(header.encryption_key_guid, [0; 16]);
    assert_eq!(header.flags, ContainerFlags(0x09));
    assert!(header.flags.has(ContainerFlags::INDEXED));
    assert!(header.flags.has(ContainerFlags::COMPRESSED));
    assert!(!header.flags.has(ContainerFlags::SIGNED));
    assert_eq!(header.perfect_hash_seeds_count, 1);
    assert_eq!(header.partition_size, u64::MAX);
    assert_eq!(header.chunks_without_perfect_hash_count, 0);
    assert_eq!(
        directory_index_offset(&header).unwrap(),
        144 + 3 * 22 + 4 + 2 * 12 + 32
    );
}

#[test]
fn ground_truth_offset_anchor() {
    let mut header: TocHeader = parse_header(&sample()).unwrap();
    header.entry_count = 62;
    header.perfect_hash_seeds_count = 31;
    header.chunks_without_perfect_hash_count = 0;
    header.compressed_block_entry_count = 469;
    assert_eq!(directory_index_offset(&header).unwrap(), 7292);
}

#[test]
fn short_file_is_not_iostore() {
    let bytes = sample();
    assert!(matches!(
        parse_header(&bytes[..143]),
        Err(PakError::NotIoStore)
    ));
    assert!(matches!(read_toc_bytes(&[]), Err(PakError::NotIoStore)));
    assert!(matches!(
        read_toc_bytes(&bytes[..100]),
        Err(PakError::NotIoStore)
    ));
}

#[test]
fn wrong_magic_is_not_iostore() {
    let mut bytes = sample();
    bytes[3] = b'x';
    assert!(matches!(read_toc_bytes(&bytes), Err(PakError::NotIoStore)));
}

#[test]
fn version_1_and_9_are_unsupported() {
    for version in [0u8, 1, 9, 255] {
        let mut bytes = sample();
        bytes[16] = version;
        assert!(
            matches!(read_toc_bytes(&bytes), Err(PakError::UnsupportedVersion(v)) if v == version)
        );
    }
}

#[test]
fn versions_2_to_8_are_accepted() {
    for version in 2u8..=8 {
        let bytes = TocBuilder::new(MOUNT)
            .file("Smoke/A.uasset")
            .version(version)
            .build();
        let listing = read_toc_bytes(&bytes).unwrap();
        assert_eq!(listing.header.version, version);
        assert_eq!(listing.files.len(), 1);
    }
}

#[test]
fn header_size_not_144_is_malformed() {
    let mut bytes = sample();
    put_u32(&mut bytes, 20, 145);
    assert!(matches!(
        read_toc_bytes(&bytes),
        Err(PakError::Malformed("header size"))
    ));
}

#[test]
fn block_entry_size_not_12_is_malformed() {
    let mut bytes = sample();
    put_u32(&mut bytes, 32, 16);
    assert!(matches!(
        read_toc_bytes(&bytes),
        Err(PakError::Malformed("block entry size"))
    ));
}

#[test]
fn encrypted_flag_is_unsupported_before_index() {
    let mut bytes = TocBuilder::new(MOUNT)
        .file("Smoke/A.uasset")
        .flags(0x0B)
        .build();
    assert!(matches!(read_toc_bytes(&bytes), Err(PakError::Encrypted)));
    put_u32(&mut bytes, 48, 0);
    bytes[80] = 0x02;
    assert!(matches!(read_toc_bytes(&bytes), Err(PakError::Encrypted)));
}

#[test]
fn not_indexed_is_no_index() {
    for flags in [0x00, 0x01] {
        let bytes = TocBuilder::new(MOUNT)
            .file("Smoke/A.uasset")
            .flags(flags)
            .build();
        assert!(matches!(
            read_toc_bytes(&bytes),
            Err(PakError::NoDirectoryIndex)
        ));
    }
}

#[test]
fn zero_index_size_is_no_index() {
    let mut bytes = sample();
    put_u32(&mut bytes, 48, 0);
    assert!(matches!(
        read_toc_bytes(&bytes),
        Err(PakError::NoDirectoryIndex)
    ));
}

#[test]
fn oversized_index_is_too_large() {
    let mut bytes = sample();
    put_u32(&mut bytes, 48, MAX_DIRECTORY_INDEX_BYTES + 1);
    assert!(matches!(read_toc_bytes(&bytes), Err(PakError::TooLarge)));
    put_u32(&mut bytes, 48, MAX_DIRECTORY_INDEX_BYTES);
    assert!(matches!(
        read_toc_bytes(&bytes),
        Err(PakError::Malformed("bounds"))
    ));
}

#[test]
fn offset_overflow_is_malformed() {
    let mut bytes = sample();
    put_u32(&mut bytes, 28, u32::MAX);
    put_u32(&mut bytes, 36, u32::MAX);
    put_u32(&mut bytes, 40, u32::MAX);
    let header = parse_header(&bytes).unwrap();
    assert!(matches!(
        directory_index_offset(&header),
        Err(PakError::Malformed("offset"))
    ));
    assert!(matches!(
        read_toc_bytes(&bytes),
        Err(PakError::Malformed("offset"))
    ));
}

#[test]
fn index_past_end_is_malformed() {
    let builder = TocBuilder::new(MOUNT).file("Smoke/A.uasset");
    let bytes = builder.build();
    let header = parse_header(&bytes).unwrap();
    let index_end = usize::try_from(directory_index_offset(&header).unwrap()).unwrap()
        + builder.build_index().len();
    assert!(read_toc_bytes(&bytes[..index_end]).is_ok());
    assert!(matches!(
        read_toc_bytes(&bytes[..index_end - 1]),
        Err(PakError::Malformed("bounds"))
    ));
}
