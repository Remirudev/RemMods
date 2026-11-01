use super::{StateInput, unknown};
use crate::mods::model::ModState;

/// Mods, ~mods and loose rows: live and `-x` paks decide (the original's ACTIVE, split).
pub(crate) fn pak_state(input: StateInput) -> ModState {
    if let Some(state) = unknown(input) {
        return state;
    }
    match (input.counts.live_pak > 0, input.counts.off_pak > 0) {
        (true, false) => ModState::Enabled,
        (true, true) => ModState::Partial,
        (false, true) => ModState::Disabled,
        (false, false) => ModState::NoPakFiles,
    }
}
