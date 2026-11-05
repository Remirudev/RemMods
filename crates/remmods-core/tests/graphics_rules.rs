//! Spec P5 §9.3 G1–G5: the pure graphics decisions.

use std::ffi::OsStr;

use remmods_core::graphics::{
    GraphicsFacts, GraphicsMode, ModeSource, PendingMarker, detect_wayland, env_for,
    is_user_controlled, step_after_crash,
};

const NV: (&str, &str) = ("__NV_DISABLE_EXPLICIT_SYNC", "1");
const SHM: (&str, &str) = ("WEBKIT_DMABUF_RENDERER_FORCE_SHM", "1");
const SOFTWARE: (&str, &str) = ("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

type EnvCase = (
    GraphicsMode,
    GraphicsFacts,
    Vec<(&'static str, &'static str)>,
);
type WaylandCase = (
    Option<&'static OsStr>,
    Option<&'static OsStr>,
    Option<&'static OsStr>,
    bool,
);

fn facts(nvidia: bool, wayland: bool) -> GraphicsFacts {
    GraphicsFacts { nvidia, wayland }
}

fn marker(mode: GraphicsMode, source: ModeSource) -> PendingMarker {
    PendingMarker {
        version: 1,
        mode,
        source,
        pid: 42,
    }
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn g1_env_for_every_mode_and_fact_combination() {
    use GraphicsMode::{Auto, ForceShm, Software};
    let cases: [EnvCase; 12] = [
        (Auto, facts(false, false), vec![]),
        (Auto, facts(true, false), vec![]),
        (Auto, facts(false, true), vec![]),
        (Auto, facts(true, true), vec![NV]),
        (ForceShm, facts(false, false), vec![SHM]),
        (ForceShm, facts(true, false), vec![SHM]),
        (ForceShm, facts(false, true), vec![SHM]),
        (ForceShm, facts(true, true), vec![NV, SHM]),
        (Software, facts(false, false), vec![SOFTWARE]),
        (Software, facts(true, false), vec![SOFTWARE]),
        (Software, facts(false, true), vec![SOFTWARE]),
        (Software, facts(true, true), vec![NV, SOFTWARE]),
    ];
    for (mode, facts, expected) in cases {
        let env = env_for(mode, facts);
        assert_eq!(env, expected, "{mode:?} {facts:?}");
        assert!(
            env.iter()
                .all(|(name, _)| *name != "WEBKIT_DISABLE_COMPOSITING_MODE")
        );
    }
}

#[test]
fn g3_user_control_names() {
    for name in [
        "WEBKIT_DMABUF_RENDERER_FORCE_SHM",
        "WEBKIT_DISABLE_DMABUF_RENDERER",
        "WEBKIT_DISABLE_COMPOSITING_MODE",
    ] {
        assert!(is_user_controlled(&names(&[name])), "{name}");
    }
    assert!(!is_user_controlled(&names(&["__NV_DISABLE_EXPLICIT_SYNC"])));
    assert!(!is_user_controlled(&[]));
}

#[test]
fn g4_next_chain() {
    assert_eq!(GraphicsMode::Auto.next(), Some(GraphicsMode::ForceShm));
    assert_eq!(GraphicsMode::ForceShm.next(), Some(GraphicsMode::Software));
    assert_eq!(GraphicsMode::Software.next(), None);
}

#[test]
fn g5_detect_wayland_table() {
    let os = |text: &'static str| Some(OsStr::new(text));
    let cases: [WaylandCase; 9] = [
        (os("wayland-0"), None, None, true),
        (os(""), None, None, false),
        (None, None, None, false),
        (None, os("wayland"), None, true),
        (None, os("x11"), None, false),
        (os(""), os("wayland"), None, true),
        (os("wayland-0"), os("wayland"), os("x11"), false),
        (os("wayland-0"), None, os("x11,wayland"), false),
        (os("wayland-0"), None, os("wayland,x11"), true),
    ];
    for (display, session, backend, expected) in cases {
        assert_eq!(
            detect_wayland(display, session, backend),
            expected,
            "{display:?} {session:?} {backend:?}"
        );
    }
}

#[test]
fn step_after_crash_rules() {
    use GraphicsMode::{Auto, ForceShm, Software};
    let stored_auto = marker(Auto, ModeSource::Stored);
    assert_eq!(
        step_after_crash(Auto, Some(&stored_auto), false),
        Some(ForceShm)
    );
    let stored_shm = marker(ForceShm, ModeSource::Stored);
    assert_eq!(
        step_after_crash(ForceShm, Some(&stored_shm), false),
        Some(Software)
    );
    let stored_software = marker(Software, ModeSource::Stored);
    assert_eq!(
        step_after_crash(Software, Some(&stored_software), false),
        None
    );
    let flag = marker(Software, ModeSource::Flag);
    assert_eq!(step_after_crash(Auto, Some(&flag), false), None);
    assert_eq!(step_after_crash(ForceShm, Some(&stored_auto), false), None);
    assert_eq!(step_after_crash(Auto, None, true), Some(ForceShm));
    assert_eq!(step_after_crash(Software, None, true), None);
    assert_eq!(step_after_crash(Auto, None, false), None);
}
