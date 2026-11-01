use super::{StateInput, unknown};
use crate::mods::issues::ModIssue;
use crate::mods::model::ModState;

/// LogicMods: the `.disabled_` prefix decides; the paks add a warning when they disagree.
pub(crate) fn logic_state(input: StateInput, prefixed: bool) -> (ModState, Option<ModIssue>) {
    if let Some(state) = unknown(input) {
        return (state, None);
    }
    let live = input.counts.live_pak > 0;
    let off = input.counts.off_pak > 0;
    match (prefixed, live, off) {
        (true, true, _) => (ModState::Disabled, Some(ModIssue::LogicDisabledButLoaded)),
        (true, false, _) => (ModState::Disabled, None),
        (false, true, false) => (ModState::Enabled, None),
        (false, true, true) => (ModState::Partial, None),
        (false, false, true) => (ModState::Disabled, Some(ModIssue::LogicEnabledNotLoaded)),
        (false, false, false) => (ModState::NoPakFiles, None),
    }
}
