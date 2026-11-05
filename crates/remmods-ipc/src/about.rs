use serde::{Deserialize, Serialize};

/// The third-party notice texts compiled into the binary (About dialog).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Notices {
    pub rust: String,
    pub npm: String,
}
