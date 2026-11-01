//! Content conflicts: rows shipping the same asset key in live Read containers (§5.3 rule 3).

use std::collections::{BTreeMap, HashMap};

use super::cache::ContainerResult;
use super::model::{CONFLICT_SAMPLE, ContentConflictLink, UtocRef};
use super::summary::count;
use crate::mods::ModState;
use crate::mods::rows::RowDraft;
use crate::safe_fs::names::order_key;

/// Asset key → this row's smallest display path, over live Read containers.
pub(crate) fn live_assets<'a>(
    utocs: &[UtocRef],
    results: &'a [ContainerResult],
) -> BTreeMap<&'a str, &'a str> {
    let mut assets: BTreeMap<&str, &str> = BTreeMap::new();
    for (utoc, result) in utocs.iter().zip(results) {
        let Ok(container) = result.as_ref() else {
            continue;
        };
        if !utoc.live {
            continue;
        }
        for (key, index) in &container.assets {
            let path = container.path_of(*index);
            assets
                .entry(key.as_str())
                .and_modify(|existing| {
                    if path < *existing {
                        *existing = path;
                    }
                })
                .or_insert(path);
        }
    }
    assets
}

type SortKey = ((String, Vec<u8>), Vec<u8>, String);

pub(crate) fn link_content_conflicts(drafts: &mut [RowDraft], results: &[Vec<ContainerResult>]) {
    let live: Vec<BTreeMap<&str, &str>> = drafts
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
    let mut shared: BTreeMap<(usize, usize), Vec<&str>> = BTreeMap::new();
    for (key, rows) in owners.iter().filter(|(_, rows)| rows.len() > 1) {
        for &this in rows {
            for &other in rows.iter().filter(|&&other| other != this) {
                shared.entry((this, other)).or_default().push(key);
            }
        }
    }
    let mut links: Vec<Vec<(SortKey, ContentConflictLink)>> = vec![Vec::new(); drafts.len()];
    for ((this, other), mut keys) in shared {
        keys.sort_unstable();
        let other_draft = &drafts[other];
        let sample = keys
            .iter()
            .take(CONFLICT_SAMPLE)
            .filter_map(|key| live[this].get(key).map(|path| (*path).to_owned()))
            .collect();
        let link = ContentConflictLink {
            other: other_draft.row.mod_ref.clone(),
            key: other_draft.row.key.clone(),
            name: other_draft.row.name.clone(),
            shared: count(keys.len()),
            sample,
        };
        let sort_key = (
            order_key(&other_draft.logical_key),
            other_draft.row.key.clone().into_bytes(),
            other_draft.row.mod_ref.as_str().to_owned(),
        );
        links[this].push((sort_key, link));
    }
    for (draft, mut found) in drafts.iter_mut().zip(links) {
        found.sort_by(|a, b| a.0.cmp(&b.0));
        draft.row.content_conflicts = found.into_iter().map(|(_, link)| link).collect();
    }
}
