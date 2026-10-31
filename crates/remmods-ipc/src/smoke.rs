use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SmokeCheck {
    pub name: String,
    pub ok: bool,
    #[specta(optional)]
    pub detail: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SmokeReport {
    pub ok: bool,
    pub checks: Vec<SmokeCheck>,
}

impl SmokeReport {
    /// Passed only if the frontend said so, every check passed and at least one check ran.
    pub fn passed(&self) -> bool {
        self.ok && !self.checks.is_empty() && self.checks.iter().all(|check| check.ok)
    }
}
