//! Per-row content summary (SPEC-P2B §5.3 rule 2).

use std::collections::{BTreeSet, HashMap, HashSet};

use super::cache::ContainerResult;
use super::model::{ContainerStatus, ContainerSummary, ContentSummary, UtocRef};

/// `utocs` and `results` are parallel and sorted by `rel_path`.
pub(crate) fn summarize(utocs: &[UtocRef], results: &[ContainerResult]) -> ContentSummary {
    let mut containers = Vec::with_capacity(utocs.len());
    let mut assets: HashSet<&str> = HashSet::new();
    let mut live_owners: HashMap<&str, u32> = HashMap::new();
    let mut characters = BTreeSet::new();
    let mut slots = BTreeSet::new();
    for (utoc, result) in utocs.iter().zip(results) {
        let summary = match result.as_ref() {
            Ok(container) => {
                for (key, _) in &container.assets {
                    assets.insert(key);
                    if utoc.live {
                        *live_owners.entry(key).or_default() += 1;
                    }
                }
                characters.extend(container.characters.iter().cloned());
                slots.extend(container.slots.iter().cloned());
                ContainerSummary {
                    rel_path: utoc.rel_path.clone(),
                    live: utoc.live,
                    status: ContainerStatus::Read,
                    version: Some(container.listing.header.version),
                    files: count(container.listing.files.len()),
                }
            }
            Err(failure) => ContainerSummary {
                rel_path: utoc.rel_path.clone(),
                live: utoc.live,
                status: failure.status,
                version: failure.version,
                files: 0,
            },
        };
        containers.push(summary);
    }
    ContentSummary {
        containers,
        assets: count(assets.len()),
        characters: characters.into_iter().collect(),
        slots: slots.into_iter().collect(),
        internal_overlaps: count(live_owners.values().filter(|n| **n > 1).count()),
        shared_identical: 0,
        shared_other_versions: 0,
    }
}

pub(crate) fn count(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}
