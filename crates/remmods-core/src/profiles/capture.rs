//! Row states as profiles see them (spec §5.8): on = the toggle would disable it; off = every
//! other `Disabled` row. Unknown and "no pak files" rows are neither.

use std::collections::BTreeMap;

use super::model::{EntryState, ProfileEntry};
use crate::mods::{ModRow, ModState, ScanContent};

pub(crate) fn is_on(row: &ModRow) -> bool {
    crate::mods::wants_disable(row)
}

pub(crate) fn is_off(row: &ModRow) -> bool {
    !is_on(row) && row.state == ModState::Disabled
}

/// Every non-ambiguous, toggleable row → Enabled if on, Disabled if off. Pure.
pub fn capture_states(content: &ScanContent) -> BTreeMap<String, ProfileEntry> {
    content
        .rows
        .iter()
        .filter(|row| !row.ambiguous && row.toggle_block.is_none())
        .filter_map(|row| {
            let state = if is_on(row) {
                EntryState::Enabled
            } else if is_off(row) {
                EntryState::Disabled
            } else {
                return None;
            };
            Some((row.key.clone(), ProfileEntry::new(state)))
        })
        .collect()
}
