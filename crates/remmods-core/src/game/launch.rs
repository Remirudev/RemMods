//! The game is always started through Steam; core never starts the exe itself.

use super::{LaunchPlan, SteamKind};

const FLATPAK_STEAM_APP: &str = "com.valvesoftware.Steam";

impl LaunchPlan {
    /// The host command line on Linux. A Flatpak or Snap Steam gets its own launcher, because
    /// xdg-open would usually start the native Steam (another install, or none). Native, Windows
    /// or no install: the desktop's `x-scheme-handler/steam`.
    pub fn host_argv(&self) -> Vec<String> {
        let parts: Vec<&str> = match self.steam_kind {
            Some(SteamKind::Flatpak) => vec!["flatpak", "run", FLATPAK_STEAM_APP, &self.url],
            Some(SteamKind::Snap) => vec!["snap", "run", "steam", &self.url],
            Some(SteamKind::Native | SteamKind::Windows) | None => vec!["xdg-open", &self.url],
        };
        parts.into_iter().map(str::to_owned).collect()
    }
}
