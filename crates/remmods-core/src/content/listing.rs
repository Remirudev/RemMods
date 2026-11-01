//! The asset list of one row for the details drawer (§5.3 rule 4).

use std::collections::BTreeMap;

use remmods_pak::hints::hint_for;
use remmods_pak::{asset_key, file_key};

use super::base_index::BaseIndex;
use super::cache::{ContainerResult, ContentCache};
use super::conflicts::live_assets;
use super::model::{
    AssetEntry, AssetOrigin, BaseIndexState, CONTENTS_MAX_ENTRIES, ModContents, UtocRef,
};
use super::summary::count;
use crate::mods::{ModRef, ModRow};

fn read_all(cache: &ContentCache, utocs: &[UtocRef]) -> Vec<ContainerResult> {
    utocs.iter().map(|utoc| cache.get_or_read(utoc).0).collect()
}

fn origin(base: &BaseIndex, path: &str) -> AssetOrigin {
    if base.contains(&file_key(path)) {
        AssetOrigin::Replaces
    } else if base.state == BaseIndexState::Complete {
        AssetOrigin::Adds
    } else {
        AssetOrigin::Unknown
    }
}

/// The live asset keys of each content-conflict partner, read through the cache.
fn partner_assets(row: &ModRow, cache: &ContentCache) -> Vec<(ModRef, Vec<String>)> {
    row.content_conflicts
        .iter()
        .filter_map(|link| {
            let utocs = cache.row_containers(link.other.as_str())?;
            let results = read_all(cache, &utocs);
            let keys = live_assets(&utocs, &results)
                .into_keys()
                .map(str::to_owned)
                .collect();
            Some((link.other.clone(), keys))
        })
        .collect()
}

pub(crate) fn mod_contents(
    row: &ModRow,
    utocs: &[UtocRef],
    cache: &ContentCache,
    base: &BaseIndex,
) -> ModContents {
    let partners = partner_assets(row, cache);
    let results = read_all(cache, utocs);
    let mut entries = Vec::new();
    let mut shared_cache: BTreeMap<String, Vec<ModRef>> = BTreeMap::new();
    for (utoc, result) in utocs.iter().zip(&results) {
        let Ok(container) = result.as_ref() else {
            continue;
        };
        for file in &container.listing.files {
            let shared_with = if utoc.live {
                let key = asset_key(&file.path);
                shared_cache
                    .entry(key)
                    .or_insert_with_key(|key| {
                        partners
                            .iter()
                            .filter(|(_, keys)| keys.binary_search(key).is_ok())
                            .map(|(other, _)| other.clone())
                            .collect()
                    })
                    .clone()
            } else {
                Vec::new()
            };
            let hint = hint_for(&file.path);
            entries.push(AssetEntry {
                path: file.path.clone(),
                container: utoc.rel_path.clone(),
                live: utoc.live,
                origin: origin(base, &file.path),
                character: hint.character,
                slot: hint.slot,
                shared_with,
            });
        }
    }
    entries.sort_by(|a, b| (&a.path, &a.container).cmp(&(&b.path, &b.container)));
    let total = count(entries.len());
    let truncated = entries.len() > CONTENTS_MAX_ENTRIES;
    entries.truncate(CONTENTS_MAX_ENTRIES);
    ModContents {
        mod_ref: row.mod_ref.clone(),
        base: base.state,
        entries,
        total,
        truncated,
    }
}
