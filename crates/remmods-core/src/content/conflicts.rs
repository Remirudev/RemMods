//! Content conflicts: rows replacing the same base-game asset in live Read containers with
//! different versions (§5.3 rule 3). Identical copies (same content hashes) are not conflicts, and
//! mod files absent from the base game are only counted (`shared_other_versions`).
//!
//! Every link is kept for the details, but only upper-body and full-body clothing makes a link
//! count toward the row's badge (`ContentConflictLink::outfit`): many outfit mods ship the same
//! hidden shoes or lower-body pieces, which would otherwise flag them all against each other.

use std::collections::{BTreeMap, HashMap};

use remmods_pak::ContentHash;
use remmods_pak::hints::{Slot, slot};

use super::base_index::BaseIndex;
use super::cache::ContainerResult;
use super::model::BaseIndexState;
use super::model::{CONFLICT_SAMPLE, ContentConflictLink, UtocRef};
use super::summary::count;
use crate::mods::ModState;
use crate::mods::rows::RowDraft;
use crate::safe_fs::names::order_key;

/// One asset of a row: its smallest display path, its fingerprint (`None` = unknown) and whether
/// any of the row's containers names it upper-body or full-body clothing.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LiveAsset<'a> {
    pub path: &'a str,
    pub print: Option<&'a [(String, ContentHash)]>,
    pub outfit: bool,
}

/// `true` for the raw slot codes of upper-body (`bdu`) and full-body (`bdf`) clothing.
pub(crate) fn is_outfit_slot(code: &str) -> bool {
    matches!(slot(code), Some(Slot::Upper | Slot::FullBody))
}

/// `true` only when both fingerprints are known and equal; unknown counts as different.
pub(crate) fn identical(
    a: Option<&[(String, ContentHash)]>,
    b: Option<&[(String, ContentHash)]>,
) -> bool {
    matches!((a, b), (Some(a), Some(b)) if a == b)
}

/// Asset key → this row's smallest display path and fingerprint, over live Read containers.
/// Two containers of the row with different versions of an asset make its fingerprint unknown.
pub(crate) fn live_assets<'a>(
    utocs: &[UtocRef],
    results: &'a [ContainerResult],
) -> BTreeMap<&'a str, LiveAsset<'a>> {
    let mut assets: BTreeMap<&str, LiveAsset<'a>> = BTreeMap::new();
    for (utoc, result) in utocs.iter().zip(results) {
        let Ok(container) = result.as_ref() else {
            continue;
        };
        if !utoc.live {
            continue;
        }
        let per_asset = container.prints.iter().zip(&container.outfits);
        for ((key, index), (print, &outfit)) in container.assets.iter().zip(per_asset) {
            let asset = LiveAsset {
                path: container.path_of(*index),
                print: print.as_deref(),
                outfit,
            };
            assets
                .entry(key.as_str())
                .and_modify(|existing| {
                    if !identical(existing.print, asset.print) {
                        existing.print = None;
                    }
                    if asset.path < existing.path {
                        existing.path = asset.path;
                    }
                    existing.outfit |= asset.outfit;
                })
                .or_insert(asset);
        }
    }
    assets
}

type SortKey = ((String, Vec<u8>), Vec<u8>, String);

/// Without a usable base index (still loading, or unavailable) nothing differing is classified:
/// no links and no `shared_other_versions`.
pub(crate) fn link_content_conflicts(
    drafts: &mut [RowDraft],
    results: &[Vec<ContainerResult>],
    base: Option<&BaseIndex>,
) {
    let base = base.filter(|base| base.state() != BaseIndexState::Unavailable);
    let live: Vec<BTreeMap<&str, LiveAsset<'_>>> = drafts
        .iter()
        .zip(results)
        .map(|(draft, results)| {
            if draft.row.state == ModState::Unknown {
                BTreeMap::new()
            } else {
                live_assets(&draft.utocs, results)
            }
        })
        .collect();
    let mut owners: HashMap<&str, Vec<usize>> = HashMap::new();
    for (index, assets) in live.iter().enumerate() {
        for key in assets.keys() {
            owners.entry(key).or_default().push(index);
        }
    }
    // Per row pair: (asset key, upper-body or full-body clothing).
    let mut differing: BTreeMap<(usize, usize), Vec<(&str, bool)>> = BTreeMap::new();
    let mut identical_only = vec![0usize; drafts.len()];
    let mut other_versions = vec![0usize; drafts.len()];
    for (key, rows) in owners.iter().filter(|(_, rows)| rows.len() > 1) {
        let replaces_game_file = base.is_some_and(|base| base.contains_asset(key));
        // One answer per key, so a pair counts it the same from both sides.
        let outfit = rows
            .iter()
            .any(|&row| live[row].get(key).is_some_and(|asset| asset.outfit));
        for &this in rows {
            let print = live[this].get(key).and_then(|asset| asset.print);
            let mut conflicting = false;
            for &other in rows.iter().filter(|&&other| other != this) {
                let other_print = live[other].get(key).and_then(|asset| asset.print);
                if !identical(print, other_print) {
                    conflicting = true;
                    if replaces_game_file {
                        differing
                            .entry((this, other))
                            .or_default()
                            .push((key, outfit));
                    }
                }
            }
            if !conflicting {
                identical_only[this] += 1;
            } else if base.is_some() && !replaces_game_file {
                other_versions[this] += 1;
            }
        }
    }
    let mut links: Vec<Vec<(SortKey, ContentConflictLink)>> = vec![Vec::new(); drafts.len()];
    for ((this, other), mut keys) in differing {
        // Clothing first, so the sample shows what makes the link count toward the badge.
        keys.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
        let outfit = keys.iter().filter(|(_, outfit)| *outfit).count();
        let other_draft = &drafts[other];
        let sample = keys
            .iter()
            .take(CONFLICT_SAMPLE)
            .filter_map(|(key, _)| live[this].get(key).map(|asset| asset.path.to_owned()))
            .collect();
        let link = ContentConflictLink {
            other: other_draft.row.mod_ref.clone(),
            key: other_draft.row.key.clone(),
            name: other_draft.row.name.clone(),
            shared: count(keys.len()),
            outfit: count(outfit),
            sample,
        };
        let sort_key = (
            order_key(&other_draft.logical_key),
            other_draft.row.key.clone().into_bytes(),
            other_draft.row.mod_ref.as_str().to_owned(),
        );
        links[this].push((sort_key, link));
    }
    let counts = identical_only.into_iter().zip(other_versions);
    for ((draft, mut found), (identical, other)) in drafts.iter_mut().zip(links).zip(counts) {
        found.sort_by(|a, b| a.0.cmp(&b.0));
        draft.row.content_conflicts = found.into_iter().map(|(_, link)| link).collect();
        draft.row.content.shared_identical = count(identical);
        draft.row.content.shared_other_versions = count(other);
    }
}
