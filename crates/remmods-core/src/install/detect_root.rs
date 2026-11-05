//! Which mod root an archive's files belong to (spec §5.3 step 6). Pure and independent of the
//! order of its input.

use super::model::{IssueCode, RootReason};
use crate::mods::RootKind;

const LIVE_PAYLOAD: [&str; 4] = ["pak", "ucas", "utoc", "sig"];
const OFF_PAYLOAD: [&str; 4] = ["pak-x", "ucas-x", "utoc-x", "sig-x"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootDetection {
    pub kind: RootKind,
    pub reason: RootReason,
    /// Per input file: its path below the root segment, or `None` when it lies outside the root
    /// (`OutsideRoot`). Without segments every path is kept as it is.
    pub stripped: Vec<Option<Vec<String>>>,
    /// Files dropped as outside the root.
    pub outside_root: u32,
    /// `MixedRoots`, `ScriptMod`, `NoPayload` (in that order).
    pub blocks: Vec<IssueCode>,
}

/// `.pak/.ucas/.utoc/.sig` and their `-x` forms, ASCII-case-insensitive.
pub(crate) fn is_payload_name(name: &str) -> bool {
    is_live_payload_name(name) || is_off_payload_name(name)
}

pub(crate) fn is_live_payload_name(name: &str) -> bool {
    has_extension(name, &LIVE_PAYLOAD)
}

pub(crate) fn is_off_payload_name(name: &str) -> bool {
    has_extension(name, &OFF_PAYLOAD)
}

fn has_extension(name: &str, extensions: &[&str]) -> bool {
    name.rsplit_once('.').is_some_and(|(stem, extension)| {
        !stem.is_empty()
            && extensions
                .iter()
                .any(|wanted| extension.eq_ignore_ascii_case(wanted))
    })
}

fn root_segment(name: &str) -> Option<RootKind> {
    RootKind::ALL
        .into_iter()
        .find(|kind| kind.folder().eq_ignore_ascii_case(name))
}

/// The first folder component naming a root, with its position.
fn first_segment(path: &[String]) -> Option<(usize, RootKind)> {
    let folders = path.split_last().map_or(&[][..], |(_, folders)| folders);
    folders
        .iter()
        .enumerate()
        .find_map(|(index, part)| root_segment(part).map(|kind| (index, kind)))
}

fn file_name(path: &[String]) -> &str {
    path.last().map_or("", String::as_str)
}

pub fn detect_root(files: &[Vec<String>]) -> RootDetection {
    let payload: Vec<&Vec<String>> = files
        .iter()
        .filter(|path| is_payload_name(file_name(path)))
        .collect();
    let segments: Vec<Option<RootKind>> = payload
        .iter()
        .map(|path| first_segment(path).map(|(_, kind)| kind))
        .collect();
    let mut blocks = Vec::new();
    let with_segment = segments.iter().filter(|kind| kind.is_some()).count();
    let detection = if with_segment > 0 {
        by_segment(files, &segments, with_segment == payload.len(), &mut blocks)
    } else {
        without_segment(files, &mut blocks)
    };
    if payload.is_empty() {
        blocks.push(IssueCode::NoPayload);
    }
    RootDetection {
        blocks,
        ..detection
    }
}

fn by_segment(
    files: &[Vec<String>],
    segments: &[Option<RootKind>],
    all_have_one: bool,
    blocks: &mut Vec<IssueCode>,
) -> RootDetection {
    let mut kinds: Vec<RootKind> = segments.iter().flatten().copied().collect();
    kinds.sort_by_key(|kind| kind_rank(*kind));
    kinds.dedup();
    if !all_have_one || kinds.len() > 1 {
        blocks.push(IssueCode::MixedRoots);
    }
    let kind = kinds.first().copied().unwrap_or(RootKind::Standard);
    let mut outside_root = 0;
    let stripped = files
        .iter()
        .map(|path| match first_segment(path) {
            Some((index, found)) if found == kind => Some(path[index + 1..].to_vec()),
            _ => {
                outside_root += 1;
                None
            }
        })
        .collect();
    RootDetection {
        kind,
        reason: RootReason::PathSegment,
        stripped,
        outside_root,
        blocks: Vec::new(),
    }
}

fn without_segment(files: &[Vec<String>], blocks: &mut Vec<IssueCode>) -> RootDetection {
    let payload_folders: Vec<&[String]> = files
        .iter()
        .filter(|path| is_payload_name(file_name(path)))
        .map(|path| folder_of(path))
        .collect();
    let config_lua = files.iter().any(|path| {
        file_name(path).eq_ignore_ascii_case("config.lua")
            && payload_folders.contains(&folder_of(path))
    });
    let script = files.iter().any(|path| {
        let name = file_name(path);
        has_extension(name, &["lua"]) || has_extension(name, &["dll"])
    });
    let (kind, reason) = if config_lua {
        (RootKind::Logic, RootReason::ConfigLua)
    } else {
        if script {
            blocks.push(IssueCode::ScriptMod);
        }
        (RootKind::Standard, RootReason::Default)
    };
    RootDetection {
        kind,
        reason,
        stripped: files.iter().cloned().map(Some).collect(),
        outside_root: 0,
        blocks: Vec::new(),
    }
}

fn folder_of(path: &[String]) -> &[String] {
    path.split_last().map_or(&[][..], |(_, folders)| folders)
}

fn kind_rank(kind: RootKind) -> u8 {
    match kind {
        RootKind::Standard => 0,
        RootKind::Legacy => 1,
        RootKind::Logic => 2,
    }
}
