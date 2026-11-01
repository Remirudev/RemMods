//! Read-only content view (P2b): `.utoc` file lists, character/slot hints and content conflicts.
//! Reading never fails a scan; every per-container problem becomes a `ContainerStatus`.

mod api;
mod base_index;
mod cache;
mod conflicts;
mod listing;
mod model;
mod summary;

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

pub(crate) use base_index::BaseIndex;
pub use cache::ContentCache;
pub(crate) use model::UtocRef;
pub use model::{
    AssetEntry, AssetOrigin, BaseIndexState, CONFLICT_SAMPLE, CONTENTS_MAX_ENTRIES,
    ContainerStatus, ContainerSummary, ContentConflictLink, ContentSummary, ModContents,
};
pub use remmods_pak::hints::{Slot, character_name, is_common, slot};

use crate::mods::rows::RowDraft;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Attached {
    pub read: u32,
    pub cached: u32,
    pub duration: Duration,
}

/// Summaries and content conflicts for every draft; evicts containers the scan did not see.
pub(crate) fn attach(drafts: &mut [RowDraft], cache: &ContentCache) -> Attached {
    let started = Instant::now();
    let mut attached = Attached::default();
    let mut seen = HashSet::new();
    let mut results = Vec::with_capacity(drafts.len());
    for draft in drafts.iter_mut() {
        draft.utocs.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        let row_results: Vec<_> = draft
            .utocs
            .iter()
            .map(|utoc| {
                seen.insert(utoc.path.clone());
                let (result, read) = cache.get_or_read(utoc);
                let counter = if read {
                    &mut attached.read
                } else {
                    &mut attached.cached
                };
                *counter = counter.saturating_add(1);
                result
            })
            .collect();
        draft.row.content = summary::summarize(&draft.utocs, &row_results);
        results.push(row_results);
    }
    conflicts::link_content_conflicts(drafts, &results);
    cache.retain(&seen);
    cache.set_rows(
        drafts
            .iter()
            .map(|draft| (draft.row.mod_ref.as_str().to_owned(), draft.utocs.clone()))
            .collect::<HashMap<_, _>>(),
    );
    attached.duration = started.elapsed();
    attached
}
