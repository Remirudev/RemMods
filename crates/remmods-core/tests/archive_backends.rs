//! P4 §8.1 B-tests: each backend lists and extracts crafted archives in TempDirs.
// Folder sources and fixtures are written with std into TempDirs.
#![allow(clippy::expect_used, clippy::disallowed_methods)]

mod archive_support;

use std::fs;
use std::io::{Cursor, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use archive_support::{
    RecordingTarget, extract_all, full_plan, plan_of, sevenz_build, sevenz_fixture, temp_dir,
    write_file,
};
use remmods_core::archive::{
    self, ArchiveError, ArchiveFormat, CancelToken, EntryKind, Limits, ProgressCounter,
    RawEntryName,
};
use remmods_core::platform;

fn dest(path: &str) -> Vec<String> {
    path.split('/').map(str::to_owned).collect()
}

fn zip_with(
    files: &[(&str, &[u8], zip::CompressionMethod)],
) -> Result<Vec<u8>, zip::result::ZipError> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        let stamp = zip::DateTime::from_date_and_time(2024, 5, 6, 7, 8, 10).expect("valid date");
        for (name, data, method) in files {
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(*method)
                .last_modified_time(stamp);
            writer.start_file(*name, options)?;
            writer.write_all(data).expect("write entry");
        }
        writer.finish()?;
    }
    Ok(cursor.into_inner())
}

#[test]
fn b1_zip_stored_and_deflate_round_trip() {
    let (_dir, root) = temp_dir();
    let big: Vec<u8> = (0..300_000u32).map(|n| (n % 253) as u8).collect();
    let bytes = zip_with(&[
        (
            "Cool/x.pak",
            b"stored bytes",
            zip::CompressionMethod::Stored,
        ),
        ("Cool/x.utoc", &big, zip::CompressionMethod::Deflated),
    ])
    .expect("zip");
    let mut reader =
        archive::open(&write_file(&root, "b1.zip", &bytes), &Limits::default()).expect("open");
    assert_eq!(reader.listing().format, ArchiveFormat::Zip);
    let plan = full_plan(reader.as_ref());
    assert_eq!(plan.files.len(), 2);
    let ticks = Arc::new(AtomicU32::new(0));
    let counted = Arc::clone(&ticks);
    let progress = ProgressCounter::new(Arc::new(move |_, _| {
        counted.fetch_add(1, Ordering::SeqCst);
    }));
    let mut target = RecordingTarget::new();
    let report = reader
        .extract(&plan, &mut target, &progress, &CancelToken::new())
        .expect("extract");
    target.settle();
    assert_eq!(target.bytes[&dest("Cool/x.pak")], b"stored bytes");
    assert_eq!(target.bytes[&dest("Cool/x.utoc")], big);
    assert_eq!(report.files, 2);
    assert_eq!(report.bytes, 12 + 300_000);
    assert_eq!(progress.bytes(), report.bytes);
    assert_eq!(progress.files(), 2);
    assert!(ticks.load(Ordering::SeqCst) >= 1);
    assert_eq!(target.finished.len(), 2);
    assert!(target.finished.iter().all(|(_, mtime)| mtime.is_some()));
}

#[test]
fn b2_zip_other_methods() {
    let data: Vec<u8> = (0..50_000u32).map(|n| (n % 97) as u8).collect();
    // zip 9.2.0 cannot write LZMA with this feature set (reading only), so LZMA is covered by
    // the property parser tests (A15) instead of a faked archive.
    for method in [
        zip::CompressionMethod::Bzip2,
        zip::CompressionMethod::Xz,
        zip::CompressionMethod::Ppmd,
    ] {
        let bytes = zip_with(&[("a.pak", &data, method)]).expect("ZipWriter writes the method");
        let (_dir, root) = temp_dir();
        let mut reader = archive::open(&write_file(&root, "b2.zip", &bytes), &Limits::default())
            .unwrap_or_else(|error| panic!("{method:?}: {error:?}"));
        let plan = full_plan(reader.as_ref());
        let mut target = RecordingTarget::new();
        extract_all(reader.as_mut(), &plan, &mut target)
            .unwrap_or_else(|error| panic!("{method:?}: {error:?}"));
        assert_eq!(target.bytes[&dest("a.pak")], data, "{method:?}");
    }
}

#[test]
fn b3_subset_plans_extract_only_their_entries() {
    let (_dir, root) = temp_dir();
    let files: [(&str, &[u8]); 3] = [("a.pak", b"aaa"), ("b.pak", b"bbbb"), ("c.pak", b"ccccc")];
    let zip = zip_with(&files.map(|(name, data)| (name, data, zip::CompressionMethod::Deflated)))
        .expect("zip");
    let sevenz = sevenz_fixture(&root, "s.7z", &files);
    let mut sources = vec![write_file(&root, "s.zip", &zip), sevenz];
    sources.extend(rar_sources(&root, &files));
    for path in sources {
        let mut reader = archive::open(&path, &Limits::default()).expect("open");
        let plan = plan_of(reader.as_ref(), &[1]);
        let mut target = RecordingTarget::new();
        let report = extract_all(reader.as_mut(), &plan, &mut target)
            .unwrap_or_else(|error| panic!("{path:?}: {error:?}"));
        assert_eq!(report.files, 1, "{path:?}");
        assert_eq!(target.created, vec![dest("b.pak")], "{path:?}");
        assert_eq!(target.bytes[&dest("b.pak")], b"bbbb", "{path:?}");
    }
}

/// A non-solid and a solid RAR5 of `files` (empty without `rar-builtin`).
fn rar_sources(root: &std::path::Path, files: &[(&str, &[u8])]) -> Vec<std::path::PathBuf> {
    #[cfg(feature = "rar-builtin")]
    {
        let plain = archive_support::rar_fixture(root, "plain.rar", files, None);
        let mut builder = rars::Builder::new(rars::ArchiveVersion::Rar50).solid(true);
        for (name, data) in files {
            builder
                .add_bytes(name.as_bytes().to_vec(), data.to_vec(), None, None)
                .expect("add");
        }
        vec![
            plain,
            archive_support::rar_build(root, "solid.rar", &builder),
        ]
    }
    #[cfg(not(feature = "rar-builtin"))]
    {
        let _unused = (root, files);
        Vec::new()
    }
}

#[test]
fn b4_sevenz_solid_lzma2() {
    let (_dir, root) = temp_dir();
    let path = sevenz_fixture(
        &root,
        "b4.7z",
        &[
            ("Cool\\x.pak", b"one"),
            ("Cool/x.utoc", b"two two"),
            ("Cool/x.ucas", b"three!"),
        ],
    );
    let mut reader = archive::open(&path, &Limits::default()).expect("open");
    let listing = reader.listing().clone();
    assert_eq!(listing.format, ArchiveFormat::SevenZip);
    assert!(listing.solid);
    assert_eq!(listing.entries.len(), 3);
    assert!(
        listing
            .entries
            .iter()
            .all(|entry| matches!(entry.name, RawEntryName::Unicode(_)))
    );
    let plan = full_plan(reader.as_ref());
    let mut target = RecordingTarget::new();
    extract_all(reader.as_mut(), &plan, &mut target).expect("extract");
    assert_eq!(target.bytes[&dest("Cool/x.pak")], b"one");
    assert_eq!(target.bytes[&dest("Cool/x.utoc")], b"two two");
    assert_eq!(target.bytes[&dest("Cool/x.ucas")], b"three!");
}

#[test]
fn b5_sevenz_anti_item_and_empty_file() {
    let (_dir, root) = temp_dir();
    let mut anti = sevenz_rust2::ArchiveEntry::new_file("gone.pak");
    anti.is_anti_item = true;
    anti.has_stream = false;
    let path = sevenz_build(
        &root,
        "b5.7z",
        vec![
            (
                sevenz_rust2::ArchiveEntry::new_file("data.pak"),
                Some(b"data".to_vec()),
            ),
            (sevenz_rust2::ArchiveEntry::new_file("empty.pak"), None),
            (anti, None),
        ],
        None,
        true,
    );
    let mut reader = archive::open(&path, &Limits::default()).expect("open");
    let entries = reader.listing().entries.clone();
    let by_name = |name: &str| {
        entries
            .iter()
            .find(|entry| entry.name == RawEntryName::Unicode(name.to_owned()))
            .expect("entry")
            .clone()
    };
    let gone = by_name("gone.pak");
    assert!(gone.anti);
    assert_eq!(gone.kind, EntryKind::Special);
    assert_eq!(by_name("empty.pak").kind, EntryKind::File);
    assert_eq!(by_name("empty.pak").size, 0);
    let plan = full_plan(reader.as_ref());
    assert_eq!(plan.files.len(), 2);
    let mut target = RecordingTarget::new();
    extract_all(reader.as_mut(), &plan, &mut target).expect("extract");
    assert_eq!(target.bytes[&dest("empty.pak")], b"");
    assert_eq!(target.bytes[&dest("data.pak")], b"data");
    assert_eq!(target.finished.len(), 2);
}

#[cfg(feature = "rar-builtin")]
#[test]
fn b6_rar5_listing_skip_and_stop() {
    let (_dir, root) = temp_dir();
    let files: [(&str, &[u8]); 3] = [
        ("Cool/x.pak", b"pak"),
        ("Cool/ü.utoc", b"utoc"),
        ("z.txt", b"z"),
    ];
    let path = archive_support::rar_fixture(&root, "b6.rar", &files, None);
    let mut reader = archive::open(&path, &Limits::default()).expect("open");
    assert_eq!(reader.listing().format, ArchiveFormat::Rar);
    assert_eq!(
        reader.listing().entries[1].name,
        RawEntryName::Unicode("Cool/ü.utoc".to_owned())
    );
    assert!(
        reader
            .listing()
            .entries
            .iter()
            .all(|entry| entry.mtime.is_some())
    );
    let first = plan_of(reader.as_ref(), &[0]);
    let mut target = RecordingTarget::new();
    extract_all(reader.as_mut(), &first, &mut target).expect("first only (Stop)");
    assert_eq!(target.created, vec![dest("Cool/x.pak")]);
    let last = plan_of(reader.as_ref(), &[2]);
    let mut target = RecordingTarget::new();
    extract_all(reader.as_mut(), &last, &mut target).expect("last only (Skip)");
    assert_eq!(target.bytes[&dest("z.txt")], b"z");
    let all = full_plan(reader.as_ref());
    let mut target = RecordingTarget::new();
    let report = extract_all(reader.as_mut(), &all, &mut target).expect("all");
    assert_eq!(report.files, 3);
    assert_eq!(target.bytes[&dest("Cool/ü.utoc")], b"utoc");
}

#[cfg(not(feature = "rar-builtin"))]
#[test]
fn b7_rar_without_the_feature_is_refused() {
    let (_dir, root) = temp_dir();
    let mut bytes = vec![0x52, 0x61, 0x72, 0x21, 0x1A, 0x07, 0x01, 0x00];
    bytes.extend_from_slice(&[0u8; 32]);
    let result = archive::open(&write_file(&root, "x.rar", &bytes), &Limits::default());
    assert!(matches!(
        result,
        Err(ArchiveError::Unsupported {
            reason: archive::UnsupportedReason::RarDisabled
        })
    ));
}

#[test]
fn b8_folder_source() {
    let (_dir, root) = temp_dir();
    let source = root.join("Cool");
    fs::create_dir_all(source.join("sub").join("deeper")).expect("dirs");
    fs::write(source.join("x.pak"), b"pak").expect("pak");
    fs::write(source.join("sub").join("x.utoc"), b"utoc!").expect("utoc");
    fs::write(source.join("sub").join("deeper").join("x.ucas"), b"").expect("ucas");
    let mut reader = archive::open_folder(&source, &Limits::default()).expect("open folder");
    let listing = reader.listing().clone();
    assert_eq!(listing.format, ArchiveFormat::Folder);
    let names: Vec<_> = listing
        .entries
        .iter()
        .map(|entry| (entry.name.clone(), entry.kind, entry.size))
        .collect();
    let unicode = |text: &str| RawEntryName::Unicode(text.to_owned());
    assert!(names.contains(&(unicode("x.pak"), EntryKind::File, 3)));
    assert!(names.contains(&(unicode("sub"), EntryKind::Dir, 0)));
    assert!(names.contains(&(unicode("sub/x.utoc"), EntryKind::File, 5)));
    assert!(names.contains(&(unicode("sub/deeper/x.ucas"), EntryKind::File, 0)));
    assert_eq!(names.len(), 5);
    let plan = full_plan(reader.as_ref());
    let mut target = RecordingTarget::new();
    extract_all(reader.as_mut(), &plan, &mut target).expect("extract");
    assert_eq!(target.bytes[&dest("sub/x.utoc")], b"utoc!");

    fs::write(source.join("sub").join("x.utoc"), b"utoc grew").expect("grow");
    let mut target = RecordingTarget::new();
    let result = extract_all(reader.as_mut(), &plan, &mut target);
    assert!(
        matches!(result, Err(ArchiveError::SizeMismatch { .. })),
        "{result:?}"
    );
    assert!(target.within_declared());
}

#[test]
fn b9_total_memory_and_meminfo() {
    assert!(platform::total_memory().is_some_and(|bytes| bytes > 0));
    assert!(archive::memory_limit() <= 4 << 30);
    assert_eq!(
        platform::parse_meminfo_total("MemTotal:       16318412 kB\nMemFree: 1 kB\n"),
        Some(16_318_412 * 1024)
    );
    assert_eq!(platform::parse_meminfo_total("MemFree: 1 kB\n"), None);
    assert_eq!(platform::parse_meminfo_total("MemTotal: lots kB\n"), None);
    assert_eq!(platform::parse_meminfo_total("MemTotal: 12 MB\n"), None);
    assert_eq!(platform::parse_meminfo_total(""), None);
}
