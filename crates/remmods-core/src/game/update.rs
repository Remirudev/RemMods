//! Whether TEKKEN 8 changed its build since RemMods last saw it. Pure decision; the service
//! reads the manifest and stores the result.

use crate::settings::LastSeenBuild;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateDecision {
    /// No build id to compare (no manifest or an unreadable one): no notice.
    Unavailable,
    Unchanged,
    /// First run for this root: store the build without a notice.
    StoreSilently,
    Notice {
        from: String,
        to: String,
    },
}

/// `same_root`: the stored `game_root` names the current root (computed by the caller).
pub fn decide_update(
    stored: Option<&LastSeenBuild>,
    same_root: bool,
    current: Option<&str>,
) -> UpdateDecision {
    let Some(current) = current else {
        return UpdateDecision::Unavailable;
    };
    match stored {
        Some(stored) if same_root => {
            if stored.build_id == current {
                UpdateDecision::Unchanged
            } else {
                UpdateDecision::Notice {
                    from: stored.build_id.clone(),
                    to: current.to_owned(),
                }
            }
        }
        _ => UpdateDecision::StoreSilently,
    }
}
