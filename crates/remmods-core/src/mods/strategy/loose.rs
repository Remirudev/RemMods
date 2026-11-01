//! Loose rows: payload files lying directly in a root, grouped by caseless stem (spec §4.5 step 4).

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

use crate::mods::model::ScanProblem;
use crate::mods::payload::Payload;
use crate::safe_fs::names::{fold_key, order_key};

#[derive(Clone, Debug)]
pub(crate) struct LooseFile {
    pub name: OsString,
    pub key: String,
    pub path: PathBuf,
    pub payload: Payload,
    pub size: Result<u64, ScanProblem>,
}

impl LooseFile {
    /// Key text without `-x` and the payload extension. The suffix is ASCII and key text copies
    /// valid UTF-8 unchanged, so the key text ends with the same bytes as the raw name.
    pub(crate) fn stem_key(&self) -> &str {
        let cut = self.key.len().saturating_sub(self.payload.suffix_len);
        self.key.get(..cut).unwrap_or_default()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LooseGroup {
    /// Sorted by `order_key` of the file key text.
    pub files: Vec<LooseFile>,
}

impl LooseGroup {
    /// The stem of the first file by `order_key`.
    pub(crate) fn stem_key(&self) -> &str {
        self.files.first().map_or("", LooseFile::stem_key)
    }

    /// More than one exact spelling of the stem.
    pub(crate) fn case_variants(&self) -> bool {
        let first = self.stem_key();
        self.files.iter().any(|file| file.stem_key() != first)
    }
}

/// Groups by `fold_key(stem)`; a group without `.pak/.ucas/.utoc` (a lone `.sig`) is dropped.
pub(crate) fn group_loose_files(files: Vec<LooseFile>) -> Vec<LooseGroup> {
    let mut groups: BTreeMap<String, Vec<LooseFile>> = BTreeMap::new();
    for file in files {
        groups
            .entry(fold_key(file.stem_key()))
            .or_default()
            .push(file);
    }
    groups
        .into_values()
        .filter(|files| files.iter().any(|file| file.payload.is_loadable()))
        .map(|mut files| {
            files.sort_by_cached_key(|file| order_key(&file.key));
            LooseGroup { files }
        })
        .collect()
}
