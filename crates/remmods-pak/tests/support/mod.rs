//! Byte helpers shared by the pak tests: header patches and hand-made directory indexes.
#![allow(dead_code, clippy::unwrap_used)]

use remmods_pak::INVALID_INDEX;

pub const MOUNT: &str = "../../../Polaris/Content/";
pub const NONE: u32 = INVALID_INDEX;

pub fn put_u32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

pub fn latin1(text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(i32::try_from(text.len()).unwrap() + 1).to_le_bytes());
    out.extend_from_slice(text.as_bytes());
    out.push(0);
    out
}

pub fn utf16_units(units: &[u16]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(-i32::try_from(units.len()).unwrap()).to_le_bytes());
    for unit in units {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    out
}

/// dirs: [name, first_child, next_sibling, first_file]; files: [name, next_file, user_data].
pub fn raw_index(
    mount: &str,
    dirs: &[[u32; 4]],
    files: &[[u32; 3]],
    strings: &[Vec<u8>],
) -> Vec<u8> {
    let mut out = latin1(mount);
    out.extend_from_slice(&i32::try_from(dirs.len()).unwrap().to_le_bytes());
    for dir in dirs {
        for value in dir {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
    out.extend_from_slice(&i32::try_from(files.len()).unwrap().to_le_bytes());
    for file in files {
        for value in file {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
    out.extend_from_slice(&i32::try_from(strings.len()).unwrap().to_le_bytes());
    for text in strings {
        out.extend_from_slice(text);
    }
    out
}

pub fn paths(listing: &remmods_pak::TocListing) -> Vec<String> {
    listing.files.iter().map(|f| f.path.clone()).collect()
}
