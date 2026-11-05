//! P4 §8.1 A19: corrupt input never panics and never writes past a declared size.
#![allow(clippy::expect_used)]

mod archive_support;

use archive_support::{
    CraftEntry, RecordingTarget, ZipCraft, extract_all, full_plan, sevenz_fixture, temp_dir,
    write_file,
};
use proptest::prelude::*;
use remmods_core::archive::{self, Limits};

const MAGICS: [&[u8]; 5] = [
    &[0x50, 0x4B, 0x03, 0x04],
    &[0x50, 0x4B, 0x05, 0x06],
    &[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C, 0x00, 0x04],
    &[0x52, 0x61, 0x72, 0x21, 0x1A, 0x07, 0x00],
    &[0x52, 0x61, 0x72, 0x21, 0x1A, 0x07, 0x01, 0x00],
];

/// Opens `bytes` as an archive and, when that works, extracts the full plan.
fn survives(bytes: &[u8], name: &str) -> Result<(), TestCaseError> {
    let (_dir, root) = temp_dir();
    let path = write_file(&root, name, bytes);
    let limits = Limits {
        memory: 64 << 20,
        ..Limits::default()
    };
    if let Ok(mut reader) = archive::open(&path, &limits) {
        let plan = full_plan(reader.as_ref());
        let mut target = RecordingTarget::new();
        let _outcome = extract_all(reader.as_mut(), &plan, &mut target);
        prop_assert!(target.within_declared());
    }
    Ok(())
}

fn small_zip() -> Vec<u8> {
    ZipCraft::new()
        .with(CraftEntry::file("Cool/x.pak", b"pak bytes"))
        .with(CraftEntry::file("Cool/x.utoc", &[7u8; 300]))
        .bytes()
}

fn small_7z() -> Vec<u8> {
    let (_dir, root) = temp_dir();
    let path = sevenz_fixture(
        &root,
        "s.7z",
        &[("Cool/x.pak", b"pak bytes"), ("Cool/x.utoc", &[7u8; 300])],
    );
    std::fs::read(path).expect("read 7z fixture")
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn magic_with_random_tail(
        magic in prop::sample::select(MAGICS.to_vec()),
        tail in prop::collection::vec(any::<u8>(), 0..512),
    ) {
        let mut bytes = magic.to_vec();
        bytes.extend_from_slice(&tail);
        survives(&bytes, "fuzz.bin")?;
    }

    #[test]
    fn truncated_zip(cut in 0usize..400, flip in any::<(u16, u8)>()) {
        let mut bytes = small_zip();
        bytes.truncate(cut.min(bytes.len()));
        if !bytes.is_empty() {
            let at = usize::from(flip.0) % bytes.len();
            bytes[at] ^= flip.1;
        }
        survives(&bytes, "t.zip")?;
    }

    #[test]
    fn truncated_7z(cut in 0usize..400, flip in any::<(u16, u8)>()) {
        let mut bytes = small_7z();
        bytes.truncate(cut.min(bytes.len()));
        if bytes.len() > 8 {
            let at = 8 + usize::from(flip.0) % (bytes.len() - 8);
            bytes[at] ^= flip.1;
        }
        survives(&bytes, "t.7z")?;
    }
}
