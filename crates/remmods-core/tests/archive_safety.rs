//! P4 §8.1 A-tests: archive safety. Every archive is crafted in a TempDir.
#![allow(clippy::expect_used)]

mod archive_support;

use std::io::{Cursor, Write};
use std::sync::Arc;

use archive_support::{
    CraftEntry, RecordingTarget, ZipCraft, extract_all, full_plan, sevenz_build, temp_dir,
    write_file,
};
use remmods_core::archive::{
    self, ArchiveError, ArchiveFormat, CancelToken, EntryKind, EntryProblem, ExtractPlan, Limits,
    Listing, NameEncoding, PlannedFile, ProgressCounter, RawEntry, RawEntryName, UnsupportedReason,
    entry_is_unsafe, normalize,
};

const MIB: u64 = 1 << 20;

fn zip_listing(names: &[&[u8]]) -> Listing {
    Listing {
        format: ArchiveFormat::Zip,
        entries: names
            .iter()
            .zip(0u32..)
            .map(|(name, index)| RawEntry {
                index,
                name: RawEntryName::Bytes(name.to_vec()),
                kind: EntryKind::File,
                size: 1,
                crc32: None,
                mtime: None,
                encrypted: false,
                anti: false,
            })
            .collect(),
        duplicate_raw: false,
        solid: false,
    }
}

fn problem(name: &[u8]) -> Option<EntryProblem> {
    let entries = normalize(
        &zip_listing(&[name]),
        NameEncoding::Auto,
        &Limits::default(),
    );
    let entry = &entries[0];
    assert_eq!(entry.problem.is_some(), entry.path.is_empty(), "{entry:?}");
    entry.problem
}

fn assert_unsafe(names: &[&str], expected: EntryProblem) {
    for name in names {
        let entries = normalize(
            &zip_listing(&[name.as_bytes()]),
            NameEncoding::Auto,
            &Limits::default(),
        );
        assert_eq!(entries[0].problem, Some(expected), "{name}");
        assert!(entry_is_unsafe(&entries[0]), "{name}");
    }
}

/// Opens a crafted zip end to end and checks that a refused name never reaches a target.
fn zip_end_to_end_unsafe(name: &str) {
    let (_dir, root) = temp_dir();
    let path = ZipCraft::new()
        .with(CraftEntry::file(name, b"evil"))
        .write(&root, "x.zip");
    let reader = archive::open(&path, &Limits::default()).expect("open");
    let entries = normalize(reader.listing(), NameEncoding::Auto, &Limits::default());
    assert!(entries.iter().any(entry_is_unsafe), "{name}");
}

#[test]
fn a1_traversal() {
    assert_unsafe(
        &[
            "../evil.pak",
            "a/../../evil.pak",
            "..\\evil.pak",
            "a\\..\\..\\evil.pak",
        ],
        EntryProblem::Traversal,
    );
    zip_end_to_end_unsafe("..\\evil.pak");
}

#[test]
fn a2_absolute() {
    assert_unsafe(&["/etc/x.pak", "\\x.pak"], EntryProblem::Absolute);
}

#[test]
fn a3_drive() {
    assert_unsafe(&["C:\\x.pak", "C:x.pak", "c:/x.pak"], EntryProblem::Drive);
    zip_end_to_end_unsafe("C:\\x.pak");
}

#[test]
fn a4_unc() {
    assert_unsafe(
        &["\\\\server\\share\\x.pak", "//server/share/x.pak"],
        EntryProblem::Unc,
    );
}

#[test]
fn a5_dots_only_and_dot_components() {
    assert_unsafe(
        &["a/.../x.pak", "a/. ./x.pak", "a/.. /x.pak"],
        EntryProblem::DotsOnly,
    );
    let entries = normalize(
        &zip_listing(&[b"a/./b//c.pak"]),
        NameEncoding::Auto,
        &Limits::default(),
    );
    assert_eq!(entries[0].path, ["a", "b", "c.pak"]);
    assert!(!entry_is_unsafe(&entries[0]));
}

#[test]
fn a6_nul_depth_length() {
    assert_eq!(problem(b"a\0b.pak"), Some(EntryProblem::Nul));
    let deep = vec!["d"; 33].join("/");
    assert_eq!(problem(deep.as_bytes()), Some(EntryProblem::TooDeep));
    let ok_depth = vec!["d"; 32].join("/");
    assert_eq!(problem(ok_depth.as_bytes()), None);
    let long = "x".repeat(4097);
    assert_eq!(problem(long.as_bytes()), Some(EntryProblem::TooLong));
    assert_eq!(problem("x".repeat(4096).as_bytes()), None);
    assert_unsafe(&["a\0b.pak"], EntryProblem::Nul);
}

fn link_plan(index: u32) -> ExtractPlan {
    ExtractPlan {
        files: vec![PlannedFile {
            index,
            slot: 0,
            dest: vec!["link.pak".to_owned()],
            size: 0,
            mtime: None,
        }],
    }
}

fn assert_link_refused(reader: &mut dyn archive::ArchiveReader, index: u32) {
    assert_eq!(
        reader.listing().entries[index as usize].kind,
        EntryKind::Link
    );
    let mut target = RecordingTarget::new();
    let result = extract_all(reader, &link_plan(index), &mut target);
    assert!(
        matches!(result, Err(ArchiveError::Corrupt { .. })),
        "{result:?}"
    );
    assert!(target.created.is_empty());
}

#[test]
fn a7_links_are_listed_and_never_extracted() {
    let (_dir, root) = temp_dir();
    let mut unix_link = CraftEntry::file("link.pak", b"/etc/passwd");
    unix_link.unix_mode = Some(0o120_777);
    let mut reparse = CraftEntry::file("junction", b"");
    reparse.external_attr = Some(0x400);
    let zip = ZipCraft::new()
        .with(unix_link)
        .with(reparse)
        .write(&root, "links.zip");
    let mut reader = archive::open(&zip, &Limits::default()).expect("open zip");
    assert_link_refused(reader.as_mut(), 0);
    assert_link_refused(reader.as_mut(), 1);

    let mut entry = sevenz_rust2::ArchiveEntry::new_file("link.pak");
    entry.has_windows_attributes = true;
    entry.windows_attributes = 0x8000 | (0o120_777 << 16);
    let sevenz = sevenz_build(
        &root,
        "links.7z",
        vec![(entry, Some(b"/etc/passwd".to_vec()))],
        None,
        false,
    );
    let mut reader = archive::open(&sevenz, &Limits::default()).expect("open 7z");
    assert_link_refused(reader.as_mut(), 0);

    #[cfg(feature = "rar-builtin")]
    {
        let mut builder = rars::Builder::new(rars::ArchiveVersion::Rar50);
        builder
            .add_unix_symlink(
                b"link.pak".to_vec(),
                b"/etc/passwd".to_vec(),
                false,
                None,
                None,
            )
            .expect("rar symlink");
        let rar = archive_support::rar_build(&root, "links.rar", &builder);
        let mut reader = archive::open(&rar, &Limits::default()).expect("open rar");
        assert_link_refused(reader.as_mut(), 0);
    }
}

#[test]
fn a8_devices_and_fifos_are_special() {
    let (_dir, root) = temp_dir();
    let mut craft = ZipCraft::new();
    for (name, mode) in [("chr", 0o020_644), ("fifo", 0o010_644), ("sock", 0o140_644)] {
        let mut entry = CraftEntry::file(name, b"");
        entry.unix_mode = Some(mode);
        craft = craft.with(entry);
    }
    let reader = archive::open(&craft.write(&root, "dev.zip"), &Limits::default()).expect("open");
    assert!(
        reader
            .listing()
            .entries
            .iter()
            .all(|entry| entry.kind == EntryKind::Special)
    );
    assert!(full_plan(reader.as_ref()).files.is_empty());
}

fn extract_zip(
    craft: &ZipCraft,
) -> (
    Result<archive::ExtractReport, ArchiveError>,
    RecordingTarget,
) {
    let (_dir, root) = temp_dir();
    let mut reader = archive::open(&craft.write(&root, "x.zip"), &Limits::default()).expect("open");
    let plan = full_plan(reader.as_ref());
    let mut target = RecordingTarget::new();
    let result = extract_all(reader.as_mut(), &plan, &mut target);
    (result, target)
}

#[test]
fn a9_declared_size_caps_every_write() {
    let data = b"0123456789abcdef";
    let (exact, target) = extract_zip(&ZipCraft::new().with(CraftEntry::file("a.pak", data)));
    assert_eq!(exact.expect("exact size").bytes, 16);
    assert_eq!(target.bytes[&vec!["a.pak".to_owned()]], data);

    let mut larger = CraftEntry::file("a.pak", data);
    larger.central_size = Some(15);
    let (result, target) = extract_zip(&ZipCraft::new().with(larger));
    assert!(
        matches!(result, Err(ArchiveError::SizeMismatch { index: 0 })),
        "{result:?}"
    );
    assert!(target.bytes[&vec!["a.pak".to_owned()]].len() <= 15);
    assert!(target.within_declared());
    assert!(target.finished.is_empty());

    let mut smaller = CraftEntry::file("a.pak", data);
    smaller.central_size = Some(17);
    let (result, target) = extract_zip(&ZipCraft::new().with(smaller));
    assert!(
        matches!(result, Err(ArchiveError::SizeMismatch { index: 0 })),
        "{result:?}"
    );
    assert!(target.finished.is_empty());
}

#[test]
fn a9b_deflate_bomb_stops_at_the_declared_size() {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        writer.start_file("bomb.pak", options).expect("start");
        writer.write_all(&vec![0u8; 8 << 20]).expect("write zeros");
        writer.finish().expect("finish");
    }
    let mut bytes = cursor.into_inner();
    let central = find(&bytes, &[0x50, 0x4B, 0x01, 0x02]).expect("central header");
    bytes[22..26].copy_from_slice(&10u32.to_le_bytes());
    bytes[central + 24..central + 28].copy_from_slice(&10u32.to_le_bytes());
    let (_dir, root) = temp_dir();
    let mut reader =
        archive::open(&write_file(&root, "bomb.zip", &bytes), &Limits::default()).expect("open");
    let plan = full_plan(reader.as_ref());
    assert_eq!(plan.files[0].size, 10);
    let mut target = RecordingTarget::new();
    let result = extract_all(reader.as_mut(), &plan, &mut target);
    assert!(
        matches!(result, Err(ArchiveError::SizeMismatch { index: 0 })),
        "{result:?}"
    );
    assert!(target.bytes.values().all(|data| data.len() <= 10));
    assert!(target.within_declared());
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[test]
fn a10_damaged_data_is_corrupt() {
    let data = b"payload bytes of a mod";
    let mut flipped = CraftEntry::file("a.pak", data);
    flipped.crc = Some(crc32fast::hash(data) ^ 1);
    let (result, _) = extract_zip(&ZipCraft::new().with(flipped));
    assert!(
        matches!(result, Err(ArchiveError::Corrupt { .. })),
        "{result:?}"
    );

    let (_dir, root) = temp_dir();
    let payload: Vec<u8> = (0..4096u32).map(|n| (n * 7 % 251) as u8).collect();
    let good = sevenz_build(
        &root,
        "good.7z",
        vec![(
            sevenz_rust2::ArchiveEntry::new_file("a.pak"),
            Some(payload.clone()),
        )],
        None,
        false,
    );
    let mut bytes = std::fs::read(&good).expect("read 7z");
    bytes[32 + 40] ^= 0x55;
    let damaged = write_file(&root, "bad.7z", &bytes);
    assert_corrupt_on_open_or_extract(&damaged);

    let mut builder = rars::Builder::new(rars::ArchiveVersion::Rar50).store(true);
    builder
        .add_bytes(b"a.pak".to_vec(), payload.clone(), None, None)
        .expect("add");
    let mut bytes = builder.to_bytes().expect("rar bytes");
    let at = find(&bytes, &payload[..64]).expect("stored payload") + 100;
    bytes[at] ^= 0x55;
    assert_corrupt_on_open_or_extract(&write_file(&root, "bad.rar", &bytes));
}

fn assert_corrupt_on_open_or_extract(path: &std::path::Path) {
    let result = archive::open(path, &Limits::default()).and_then(|mut reader| {
        let plan = full_plan(reader.as_ref());
        extract_all(reader.as_mut(), &plan, &mut RecordingTarget::new())
    });
    assert!(
        matches!(result, Err(ArchiveError::Corrupt { .. })),
        "{path:?}: {result:?}"
    );
}

#[test]
fn a11_encrypted_archives_are_refused() {
    let (_dir, root) = temp_dir();
    let mut entry = CraftEntry::file("a.pak", b"secret");
    entry.encrypted_flag = true;
    let zip = ZipCraft::new().with(entry).write(&root, "enc.zip");
    assert!(matches!(
        archive::open(&zip, &Limits::default()),
        Err(ArchiveError::Encrypted)
    ));

    let aes = sevenz_raw(&[0u8; 16], &aes_encoded_header());
    let result = archive::open(&write_file(&root, "enc.7z", &aes), &Limits::default());
    assert!(
        matches!(result, Err(ArchiveError::Encrypted)),
        "{:?}",
        result.err()
    );

    #[cfg(feature = "rar-builtin")]
    {
        let rar =
            archive_support::rar_fixture(&root, "enc.rar", &[("a.pak", b"secret")], Some(b"pw"));
        let result = archive::open(&rar, &Limits::default());
        assert!(
            matches!(result, Err(ArchiveError::Encrypted)),
            "{:?}",
            result.err()
        );

        let mut builder = rars::Builder::new(rars::ArchiveVersion::Rar50)
            .password(Some(b"pw".to_vec()))
            .header_encryption(true);
        builder
            .add_bytes(b"a.pak".to_vec(), b"secret".to_vec(), None, None)
            .expect("add");
        let headers = archive_support::rar_build(&root, "headers.rar", &builder);
        let result = archive::open(&headers, &Limits::default());
        assert!(
            matches!(result, Err(ArchiveError::Encrypted)),
            "{:?}",
            result.err()
        );
    }
}

/// 7z bytes: start header (with both CRCs), `packed`, then `next_header`.
fn sevenz_raw(packed: &[u8], next_header: &[u8]) -> Vec<u8> {
    let mut start = Vec::new();
    start.extend_from_slice(&(packed.len() as u64).to_le_bytes());
    start.extend_from_slice(&(next_header.len() as u64).to_le_bytes());
    start.extend_from_slice(&crc32fast::hash(next_header).to_le_bytes());
    let mut out = vec![0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C, 0, 4];
    out.extend_from_slice(&crc32fast::hash(&start).to_le_bytes());
    out.extend_from_slice(&start);
    out.extend_from_slice(packed);
    out.extend_from_slice(next_header);
    out
}

/// An encoded header: one 16-byte pack stream through one coder (`id`, `props`).
fn encoded_header(id: &[u8], props: &[u8]) -> Vec<u8> {
    let mut header = vec![
        0x17, 0x06, 0x00, 0x01, 0x09, 16, 0x00, 0x07, 0x0B, 0x01, 0x00, 0x01,
    ];
    header.push(0x20 | u8::try_from(id.len()).expect("short id"));
    header.extend_from_slice(id);
    header.push(u8::try_from(props.len()).expect("short props"));
    header.extend_from_slice(props);
    header.extend_from_slice(&[0x0C, 16, 0x00, 0x00]);
    header
}

fn aes_encoded_header() -> Vec<u8> {
    encoded_header(&[0x06, 0xF1, 0x07, 0x01], &[0x13])
}

#[test]
fn a12_multi_volume_is_refused() {
    let (_dir, root) = temp_dir();
    let zip = ZipCraft::new()
        .with(CraftEntry::file("a.pak", b"a"))
        .bytes();
    for name in [
        "x.z01",
        "x.zip.001",
        "x.7z.001",
        "x.part2.rar",
        "x.part1.rar",
        "x.r00",
        "X.PART01.RAR",
    ] {
        let result = archive::open(&write_file(&root, name, &zip), &Limits::default());
        assert!(
            matches!(
                result,
                Err(ArchiveError::Unsupported {
                    reason: UnsupportedReason::MultiVolume
                })
            ),
            "{name}: {:?}",
            result.err()
        );
    }
    let mut on_disk = CraftEntry::file("a.pak", b"a");
    on_disk.disk = 1;
    let split = ZipCraft::new().with(on_disk).write(&root, "split.zip");
    let result = archive::open(&split, &Limits::default());
    assert!(
        matches!(
            result,
            Err(ArchiveError::Unsupported {
                reason: UnsupportedReason::MultiVolume
            })
        ),
        "{:?}",
        result.err()
    );
    let mut spanned = vec![0x50, 0x4B, 0x07, 0x08];
    spanned.extend_from_slice(&zip);
    let result = archive::open(
        &write_file(&root, "spanned.zip", &spanned),
        &Limits::default(),
    );
    assert!(matches!(
        result,
        Err(ArchiveError::Unsupported {
            reason: UnsupportedReason::MultiVolume
        })
    ));
    for name in ["x.zip", "x.part0.rar", "x.rar", "x.7z", "x.001"] {
        assert_eq!(
            archive::volume_name_kind(std::ffi::OsStr::new(name)),
            None,
            "{name}"
        );
    }
}

#[test]
fn a13_magic_decides_the_format() {
    let (_dir, root) = temp_dir();
    let zip = ZipCraft::new()
        .with(CraftEntry::file("a.pak", b"a"))
        .bytes();
    let mut sfx = vec![0x4D; 64];
    sfx.extend_from_slice(&zip);
    let result = archive::open(&write_file(&root, "sfx.exe", &sfx), &Limits::default());
    assert!(matches!(result, Err(ArchiveError::NotAnArchive)));
    let renamed = archive::open(&write_file(&root, "x.rar", &zip), &Limits::default())
        .expect("zip named rar");
    assert_eq!(renamed.listing().format, ArchiveFormat::Zip);
    let empty = archive::open(&write_file(&root, "empty.zip", b""), &Limits::default());
    assert!(matches!(empty, Err(ArchiveError::NotAnArchive)));
    let folder = archive::open(&root, &Limits::default());
    assert!(matches!(folder, Err(ArchiveError::NotAnArchive)));
    let not_folder = archive::open_folder(&root.join("x.rar"), &Limits::default());
    assert!(matches!(not_folder, Err(ArchiveError::NotAnArchive)));
}

#[test]
fn a14_entry_count_and_total_limits() {
    let (_dir, root) = temp_dir();
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for index in 0..65_537u32 {
            writer
                .start_file(format!("{index}"), options)
                .expect("start");
            writer.write_all(b"x").expect("write");
        }
        writer.finish().expect("finish");
    }
    let many = write_file(&root, "many.zip", &cursor.into_inner());
    let result = archive::open(&many, &Limits::default());
    assert!(
        matches!(result, Err(ArchiveError::TooManyEntries { count: 65_537 })),
        "{:?}",
        result.err()
    );

    let big = ZipCraft::new()
        .with(CraftEntry::file("a.pak", &[1; 8]))
        .with(CraftEntry::file("b.pak", &[2; 8]))
        .write(&root, "big.zip");
    let limits = Limits {
        max_total: 15,
        ..Limits::default()
    };
    let result = archive::open(&big, &limits);
    assert!(
        matches!(result, Err(ArchiveError::TooLarge { bytes: 16 })),
        "{:?}",
        result.err()
    );
    assert!(
        archive::open(
            &big,
            &Limits {
                max_total: 16,
                ..Limits::default()
            }
        )
        .is_ok()
    );
}

#[test]
fn a15_decoder_memory_from_properties() {
    assert_eq!(
        archive::sevenz_coder_memory(&[0x21], &[40]),
        Some((4 << 30) - 1)
    );
    assert_eq!(archive::sevenz_coder_memory(&[0x21], &[28]), Some(64 * MIB));
    assert_eq!(archive::sevenz_coder_memory(&[0x21], &[41]), None);
    assert_eq!(
        archive::sevenz_coder_memory(&[0x03, 0x01, 0x01], &[0x5D, 0, 0, 0, 4]),
        Some(64 * MIB)
    );
    assert_eq!(
        archive::sevenz_coder_memory(&[0x03, 0x04, 0x01], &[6, 0, 0, 0, 1]),
        Some(16 * MIB)
    );
    assert_eq!(
        archive::sevenz_coder_memory(&[0x04, 0xF7, 0x11, 0x01], &[]),
        None
    );
    assert_eq!(archive::ppmd_memory(16 << 20), 16 * MIB);
    assert_eq!(
        archive::zip_lzma_memory(&[9, 20, 5, 0, 0x5D, 0, 0, 0, 4]),
        Some(64 * MIB)
    );
    assert_eq!(
        archive::zip_lzma_memory(&[9, 20, 4, 0, 0x5D, 0, 0, 0, 4]),
        None
    );
    assert_eq!(archive::zip_xz_memory(&xz_head(28)), Some(64 * MIB));
    assert_eq!(archive::zip_xz_memory(&xz_head(40)), Some((4 << 30) - 1));
    assert_eq!(archive::zip_xz_memory(&xz_head(41)), None);
    assert_eq!(archive::zip_xz_memory(&[0u8; 16]), None);

    let (_dir, root) = temp_dir();
    let path = sevenz_build(
        &root,
        "small.7z",
        vec![(
            sevenz_rust2::ArchiveEntry::new_file("a.pak"),
            Some(b"abc".to_vec()),
        )],
        None,
        true,
    );
    let mut bytes = std::fs::read(&path).expect("read 7z");
    patch_lzma2_dict(&mut bytes, 28);
    let patched = write_file(&root, "dict64.7z", &bytes);
    let limits = Limits {
        memory: 16 * MIB,
        ..Limits::default()
    };
    let result = archive::open(&patched, &limits);
    assert!(
        matches!(result, Err(ArchiveError::MemoryLimit { needed, limit }) if needed == 64 * MIB && limit == 16 * MIB),
        "{:?}",
        result.err()
    );
    assert!(
        archive::open(
            &patched,
            &Limits {
                memory: 64 * MIB,
                ..Limits::default()
            }
        )
        .is_ok()
    );
}

/// xz stream header + a block header with one LZMA2 filter of props byte `prop`.
fn xz_head(prop: u8) -> Vec<u8> {
    let mut head = vec![0xFD, b'7', b'z', b'X', b'Z', 0x00, 0x00, 0x04, 0, 0, 0, 0];
    head.extend_from_slice(&[0x02, 0x00, 0x21, 0x01, prop, 0, 0, 0, 0, 0, 0, 0]);
    head
}

/// Rewrites the LZMA2 property byte in a plain (not encoded) 7z header and fixes its CRC.
fn patch_lzma2_dict(bytes: &mut [u8], prop: u8) {
    let next_offset = u64::from_le_bytes(bytes[12..20].try_into().expect("8 bytes")) as usize;
    let next_size = u64::from_le_bytes(bytes[20..28].try_into().expect("8 bytes")) as usize;
    let start = 32 + next_offset;
    let header = &mut bytes[start..start + next_size];
    assert_eq!(header[0], 0x01, "the writer stores a plain header");
    let coder = find(header, &[0x21, 0x21, 0x01]).expect("LZMA2 coder in the header");
    header[coder + 3] = prop;
    let crc = crc32fast::hash(header);
    bytes[28..32].copy_from_slice(&crc.to_le_bytes());
    let start_crc = crc32fast::hash(&bytes[12..32]);
    bytes[8..12].copy_from_slice(&start_crc.to_le_bytes());
}

#[test]
fn a16_overlapping_entries_are_corrupt() {
    let craft = ZipCraft::new()
        .with(CraftEntry::file("a.pak", b"same"))
        .with(CraftEntry::file("b.pak", b"same"));
    let mut bytes = craft.bytes();
    let first = find(&bytes, &[0x50, 0x4B, 0x01, 0x02]).expect("first central header");
    let second = first + 4 + find(&bytes[first + 4..], &[0x50, 0x4B, 0x01, 0x02]).expect("second");
    bytes[second + 42..second + 46].copy_from_slice(&0u32.to_le_bytes());
    let (_dir, root) = temp_dir();
    let result = archive::open(
        &write_file(&root, "overlap.zip", &bytes),
        &Limits::default(),
    );
    assert!(
        matches!(result, Err(ArchiveError::Corrupt { .. })),
        "{:?}",
        result.err()
    );
}

#[test]
fn a17_cancel_inside_the_first_create_stops_before_the_second() {
    let (_dir, root) = temp_dir();
    let path = ZipCraft::new()
        .with(CraftEntry::file("a.pak", b"a"))
        .with(CraftEntry::file("b.pak", b"b"))
        .write(&root, "two.zip");
    let mut reader = archive::open(&path, &Limits::default()).expect("open");
    let plan = full_plan(reader.as_ref());
    let cancel = CancelToken::new();
    let hook_token = cancel.clone();
    let mut target = RecordingTarget::new();
    target.on_create = Some(Box::new(move |_| hook_token.cancel()));
    let ticks = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let counted = Arc::clone(&ticks);
    let progress = ProgressCounter::new(Arc::new(move |_, _| {
        counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }));
    let result = reader.extract(&plan, &mut target, &progress, &cancel);
    assert!(matches!(result, Err(ArchiveError::Cancelled)), "{result:?}");
    assert_eq!(target.created.len(), 1);
}

#[test]
fn a18_duplicate_raw_names_are_flagged() {
    let (_dir, root) = temp_dir();
    let path = ZipCraft::new()
        .with(CraftEntry::file("a.pak", b"one"))
        .with(CraftEntry::file("a.pak", b"two"))
        .write(&root, "dup.zip");
    let reader = archive::open(&path, &Limits::default()).expect("open");
    assert!(reader.listing().duplicate_raw);
    let single = ZipCraft::new()
        .with(CraftEntry::file("a.pak", b"one"))
        .write(&root, "one.zip");
    assert!(
        !archive::open(&single, &Limits::default())
            .expect("open")
            .listing()
            .duplicate_raw
    );
}

#[test]
fn a20_encoded_header_memory_is_checked_before_the_crate() {
    let lzma2 = encoded_header(&[0x21], &[40]);
    assert_eq!(archive::sevenz_header_memory(&lzma2), Some((4 << 30) - 1));
    assert_eq!(archive::sevenz_header_memory(&lzma2[..15]), None);
    assert_eq!(archive::sevenz_header_memory(&[0x17, 0xFF, 0x00]), None);
    assert_eq!(archive::sevenz_header_memory(&[]), None);
    assert_eq!(
        archive::sevenz_header_memory(&encoded_header(&[0x04, 0xF7, 0x11, 0x01], &[])),
        None
    );

    let (_dir, root) = temp_dir();
    let claims = sevenz_raw(&[0u8; 16], &encoded_header(&[0x21], &[28]));
    let path = write_file(&root, "claims.7z", &claims);
    let limits = Limits {
        memory: 16 * MIB,
        ..Limits::default()
    };
    let result = archive::open(&path, &limits);
    assert!(
        matches!(result, Err(ArchiveError::MemoryLimit { needed, .. }) if needed == 64 * MIB),
        "{:?}",
        result.err()
    );

    let mut past_end = claims.clone();
    past_end[20..28].copy_from_slice(&(1u64 << 20).to_le_bytes());
    let result = archive::open(&write_file(&root, "past.7z", &past_end), &Limits::default());
    assert!(
        matches!(result, Err(ArchiveError::Corrupt { .. })),
        "{:?}",
        result.err()
    );
    let mut huge = claims;
    huge[20..28].copy_from_slice(&(65u64 << 20).to_le_bytes());
    let result = archive::open(&write_file(&root, "huge.7z", &huge), &Limits::default());
    assert!(
        matches!(result, Err(ArchiveError::Corrupt { .. })),
        "{:?}",
        result.err()
    );
}
