//! `Container`: chunks found through the `.utoc` tables, read from the `.ucas`.
#![allow(clippy::disallowed_methods, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};

use remmods_pak::fixture::TocBuilder;
use remmods_unpack::{Container, UnpackError};
use tempfile::TempDir;

const BLOCK: usize = 0x1_0000;
const HASH: [u8; 20] = [7; 20];
/// An Oodle block that stores its bytes: flag byte 0x4C, then the decoder type (Kraken).
const OODLE_STORED_HEADER: [u8; 2] = [0x4C, 0x06];

fn pattern(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|at| seed.wrapping_add(u8::try_from(at % 251).unwrap_or(0)))
        .collect()
}

fn write(dir: &TempDir, name: &str, toc: &TocBuilder) -> PathBuf {
    let (utoc, ucas) = toc.build_with_cas();
    let path = dir.path().join(format!("{name}.utoc"));
    fs::write(&path, utoc).expect("utoc");
    fs::write(path.with_extension("ucas"), ucas).expect("ucas");
    path
}

/// The `.ucas` is opened per read, so the folder has to outlive the container.
fn container(toc: &TocBuilder) -> (TempDir, Container) {
    let dir = TempDir::new().expect("temp dir");
    let path = write(&dir, "a", toc);
    let opened = Container::open(&path).expect("open");
    (dir, opened)
}

fn builder() -> TocBuilder {
    TocBuilder::new("../../../")
}

#[test]
fn stored_chunks_come_back_whole() {
    let first = pattern(10, 1);
    let second = pattern(BLOCK * 2 + 123, 2);
    let toc = builder()
        .file_with_bytes("a.uasset", HASH, &first)
        .file_with_bytes("b.uasset", HASH, &second);
    let (_dir, opened) = container(&toc);
    assert_eq!(opened.chunk(0).expect("first"), first);
    assert_eq!(opened.chunk(1).expect("second"), second);
}

#[test]
fn an_empty_chunk_is_empty() {
    let toc = builder()
        .file_with_bytes("a.uasset", HASH, b"x")
        .file("empty.uasset");
    let (_dir, opened) = container(&toc);
    assert_eq!(opened.chunk(1).expect("empty"), Vec::<u8>::new());
}

#[test]
fn oodle_blocks_are_decoded() {
    let bytes = pattern(BLOCK + 5000, 3);
    let toc = builder()
        .file_with_bytes("a.uasset", HASH, &bytes)
        .block_wrap(|block| {
            let mut stored = OODLE_STORED_HEADER.to_vec();
            stored.extend_from_slice(block);
            (stored, 1)
        });
    let (_dir, opened) = container(&toc);
    assert_eq!(opened.chunk(0).expect("chunk"), bytes);
}

#[test]
fn a_broken_oodle_block_is_an_error() {
    let toc = builder()
        .file_with_bytes("a.uasset", HASH, b"payload")
        .block_wrap(|_| (vec![0xFF; 16], 1));
    let (_dir, opened) = container(&toc);
    assert!(matches!(
        opened.chunk(0),
        Err(UnpackError::Decompress | UnpackError::Decoder)
    ));
}

#[test]
fn an_unknown_method_is_refused() {
    let toc = builder()
        .file_with_bytes("a.uasset", HASH, b"payload")
        .block_wrap(|block| (block.to_vec(), 9));
    let (_dir, opened) = container(&toc);
    assert!(matches!(opened.chunk(0), Err(UnpackError::UnknownMethod)));
}

#[test]
fn a_chunk_past_the_table_is_refused() {
    let toc = builder().file_with_bytes("a.uasset", HASH, b"x");
    let (_dir, opened) = container(&toc);
    assert!(matches!(opened.chunk(99), Err(UnpackError::Malformed(_))));
}

#[test]
fn a_cut_off_ucas_is_an_error() {
    let dir = TempDir::new().expect("temp dir");
    let toc = builder().file_with_bytes("a.uasset", HASH, &pattern(4096, 4));
    let path = write(&dir, "a", &toc);
    let ucas = path.with_extension("ucas");
    let bytes = fs::read(&ucas).expect("read");
    fs::write(&ucas, &bytes[..100]).expect("cut");
    let opened = Container::open(&path).expect("open");
    assert!(matches!(opened.chunk(0), Err(UnpackError::Io(_))));
}

#[test]
fn a_switched_off_container_reads_its_x_files() {
    let dir = TempDir::new().expect("temp dir");
    let toc = builder().file_with_bytes("a.uasset", HASH, b"payload");
    let path = write(&dir, "a", &toc);
    let off = path.with_extension("utoc-x");
    fs::rename(&path, &off).expect("rename utoc");
    fs::rename(path.with_extension("ucas"), off.with_extension("ucas-x")).expect("rename ucas");
    let opened = Container::open(&off).expect("open");
    assert_eq!(opened.chunk(0).expect("chunk"), b"payload");
}

#[test]
fn only_toc_files_open() {
    let dir = TempDir::new().expect("temp dir");
    let toc = builder().file_with_bytes("a.uasset", HASH, b"payload");
    let path = write(&dir, "a", &toc);
    assert!(matches!(
        Container::open(&path.with_extension("ucas")),
        Err(UnpackError::Malformed(_))
    ));
    assert!(matches!(
        Container::open(Path::new("missing.utoc")),
        Err(UnpackError::Io(_))
    ));
}

#[test]
fn a_garbled_table_never_panics() {
    let toc = builder()
        .file_with_bytes("a.uasset", HASH, &pattern(BLOCK + 10, 5))
        .file_with_bytes("b.uasset", HASH, &pattern(100, 6));
    let (utoc, ucas) = toc.build_with_cas();
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("a.utoc");
    fs::write(path.with_extension("ucas"), ucas).expect("ucas");
    let mut seed = 0x2545_F491_u32;
    for _ in 0..400 {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let mut bytes = utoc.clone();
        let at = usize::try_from(seed).unwrap_or(0) % bytes.len();
        bytes[at] ^= u8::try_from(seed >> 24).unwrap_or(1) | 1;
        fs::write(&path, &bytes).expect("utoc");
        if let Ok(opened) = Container::open(&path) {
            let _ = opened.chunk(0);
            let _ = opened.chunk(1);
        }
    }
}
