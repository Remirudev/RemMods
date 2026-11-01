use remmods_core::game::{LaunchPlan, STEAM_LAUNCH_URL, SteamKind};

fn argv(steam_kind: Option<SteamKind>) -> Vec<String> {
    LaunchPlan {
        url: STEAM_LAUNCH_URL.to_owned(),
        steam_kind,
    }
    .host_argv()
}

#[test]
fn host_argv_per_steam_kind() {
    let url = "steam://rungameid/1778820";
    assert_eq!(STEAM_LAUNCH_URL, url);
    assert_eq!(argv(Some(SteamKind::Native)), ["xdg-open", url]);
    assert_eq!(
        argv(Some(SteamKind::Flatpak)),
        ["flatpak", "run", "com.valvesoftware.Steam", url]
    );
    assert_eq!(argv(Some(SteamKind::Snap)), ["snap", "run", "steam", url]);
    assert_eq!(argv(Some(SteamKind::Windows)), ["xdg-open", url]);
    assert_eq!(argv(None), ["xdg-open", url]);
}
