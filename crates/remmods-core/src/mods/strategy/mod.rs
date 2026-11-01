//! State rules per root kind (spec §4.6 tables).

mod logic;
mod loose;
mod pak;

pub(crate) use logic::logic_state;
pub(crate) use loose::{LooseFile, LooseGroup, group_loose_files};
pub(crate) use pak::pak_state;

use super::model::{ModState, PayloadCounts};

/// What decides a row's state besides its payload counts.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct StateInput {
    pub counts: PayloadCounts,
    /// The walk root could not be listed.
    pub unreadable: bool,
    /// A linked subfolder was not followed, so its paks are not counted.
    pub contains_link: bool,
}

/// `Unknown` when the counts cannot be trusted: unreadable root, or no live pak next to a link.
fn unknown(input: StateInput) -> Option<ModState> {
    (input.unreadable || (input.contains_link && input.counts.live_pak == 0))
        .then_some(ModState::Unknown)
}
