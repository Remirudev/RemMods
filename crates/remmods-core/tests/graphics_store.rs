//! Spec P5 §9.3 G2, G6–G21: `graphics.json`, the crash marker and the app-file helpers.
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use remmods_core::AppPaths;
use remmods_core::graphics::{
    self, GraphicsError, GraphicsFacts, GraphicsMode, GraphicsPaths, GraphicsSession, ModeSource,
    StartInput,
};
use remmods_core::safe_fs::{self, AppLock, FsError, OpGate};
use tempfile::TempDir;

const NOW: &str = "2026-11-02T10:00:00+01:00";
const PID: u32 = 42;
const NV: (&str, &str) = ("__NV_DISABLE_EXPLICIT_SYNC", "1");
const SHM: (&str, &str) = ("WEBKIT_DMABUF_RENDERER_FORCE_SHM", "1");
const SOFTWARE: (&str, &str) = ("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

struct Fixture {
    _dir: TempDir,
    root: PathBuf,
    app: AppPaths,
    paths: GraphicsPaths,
}

fn fixture() -> Fixture {
    let (dir, root) = support::temp_root();
    let app = AppPaths::under(&root);
    fs::create_dir_all(&app.config_dir).expect("config dir");
    fs::create_dir_all(&app.data_dir).expect("data dir");
    let paths = GraphicsPaths::from_app(&app);
    Fixture {
        _dir: dir,
        root,
        app,
        paths,
    }
}

fn input() -> StartInput {
    StartInput {
        safe_flag: false,
        facts: GraphicsFacts::default(),
        present: Vec::new(),
        other_instance: false,
        pid: PID,
        now: NOW.to_owned(),
    }
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|name| (*name).to_owned()).collect()
}

fn marker_json(mode: &str, source: &str, pid: u32) -> String {
    format!("{{\"version\":1,\"mode\":\"{mode}\",\"source\":\"{source}\",\"pid\":{pid}}}\n")
}

fn read_text(path: &Path) -> String {
    fs::read_to_string(path).expect("read file")
}

impl Fixture {
    fn write_file(&self, text: &str) {
        fs::write(&self.paths.file, text).expect("write graphics.json");
    }

    fn write_marker(&self, text: &str) {
        fs::write(&self.paths.marker, text).expect("write marker");
    }

    fn start(&self, input: &StartInput) -> GraphicsSession {
        graphics::start(&self.paths, input, &|_| false)
    }

    fn backups(&self) -> Vec<String> {
        support::names_in(&self.app.config_dir)
            .into_iter()
            .filter(|name| name.starts_with("graphics.json.backup."))
            .collect()
    }
}

#[test]
fn g6_clean_start_writes_only_the_marker() {
    let fx = fixture();
    let session = fx.start(&input());
    assert_eq!(session.effective, GraphicsMode::Auto);
    assert_eq!(session.stored, GraphicsMode::Auto);
    assert_eq!(session.source, ModeSource::Stored);
    assert!(session.marker_written);
    assert!(!session.stepped_now);
    assert!(session.env.is_empty());
    assert_eq!(
        read_text(&fx.paths.marker),
        marker_json("auto", "stored", PID)
    );
    assert!(!fx.paths.file.exists());
}

#[test]
fn g7_g21_surviving_auto_marker_steps_to_force_shm() {
    let fx = fixture();
    fx.write_marker(&marker_json("auto", "stored", 7));
    let session = fx.start(&input());
    assert!(session.stepped_now);
    assert_eq!(session.stored, GraphicsMode::ForceShm);
    assert_eq!(session.effective, GraphicsMode::ForceShm);
    assert_eq!(session.env, vec![SHM]);
    let step = session.stepped.expect("step recorded");
    assert_eq!(
        (step.from, step.to, step.at.as_str()),
        (GraphicsMode::Auto, GraphicsMode::ForceShm, NOW)
    );
    let expected = format!(
        "{{\n  \"version\": 1,\n  \"mode\": \"forceShm\",\n  \"stepped\": {{\n    \"from\": \
         \"auto\",\n    \"to\": \"forceShm\",\n    \"at\": \"{NOW}\"\n  }}\n}}\n"
    );
    assert_eq!(read_text(&fx.paths.file), expected);
    assert_eq!(
        read_text(&fx.paths.marker),
        marker_json("forceShm", "stored", PID)
    );
}

#[test]
fn g8_force_shm_steps_to_software_and_software_stays() {
    let fx = fixture();
    fx.write_file("{\"version\":1,\"mode\":\"forceShm\"}");
    fx.write_marker(&marker_json("forceShm", "stored", 7));
    let session = fx.start(&input());
    assert!(session.stepped_now);
    assert_eq!(session.effective, GraphicsMode::Software);
    assert_eq!(session.env, vec![SOFTWARE]);

    fx.write_file("{\"version\":1,\"mode\":\"software\"}");
    fx.write_marker(&marker_json("software", "stored", 7));
    let session = fx.start(&input());
    assert!(!session.stepped_now);
    assert_eq!(session.stored, GraphicsMode::Software);
    assert_eq!(
        read_text(&fx.paths.file),
        "{\"version\":1,\"mode\":\"software\"}"
    );
}

#[test]
fn g9_flag_marker_never_steps() {
    let fx = fixture();
    fx.write_marker(&marker_json("software", "flag", 7));
    let session = fx.start(&input());
    assert!(!session.stepped_now);
    assert_eq!(session.effective, GraphicsMode::Auto);
    assert!(!fx.paths.file.exists());
}

#[test]
fn g10_marker_of_another_mode_never_steps() {
    let fx = fixture();
    fx.write_marker(&marker_json("forceShm", "stored", 7));
    let session = fx.start(&input());
    assert!(!session.stepped_now);
    assert_eq!(session.effective, GraphicsMode::Auto);
    assert!(!fx.paths.file.exists());
}

#[test]
fn g11_corrupt_marker_steps() {
    let fx = fixture();
    fx.write_marker("{\"version\":1,\"mo");
    let session = fx.start(&input());
    assert!(session.stepped_now);
    assert_eq!(session.effective, GraphicsMode::ForceShm);
    assert_eq!(
        read_text(&fx.paths.marker),
        marker_json("forceShm", "stored", PID)
    );
}

#[test]
fn g12_safe_flag_is_software_for_this_session_only() {
    let fx = fixture();
    let session = fx.start(&StartInput {
        safe_flag: true,
        ..input()
    });
    assert_eq!(session.effective, GraphicsMode::Software);
    assert_eq!(session.source, ModeSource::Flag);
    assert_eq!(session.stored, GraphicsMode::Auto);
    assert_eq!(session.env, vec![SOFTWARE]);
    assert!(!fx.paths.file.exists());
    assert_eq!(
        read_text(&fx.paths.marker),
        marker_json("software", "flag", PID)
    );
}

#[test]
fn g2_user_set_names_are_never_set() {
    let fx = fixture();
    fx.write_file("{\"version\":1,\"mode\":\"forceShm\"}");
    let all = GraphicsFacts {
        nvidia: true,
        wayland: true,
    };
    for name in graphics::MANAGED_VARS {
        graphics::clear_marker(&fx.paths).expect("no marker from the last start");
        let session = fx.start(&StartInput {
            facts: all,
            present: names(&[name]),
            ..input()
        });
        assert!(
            session.env.iter().all(|(set, _)| *set != name),
            "{name} set: {:?}",
            session.env
        );
    }
    graphics::clear_marker(&fx.paths).expect("no marker from the last start");
    let session = fx.start(&StartInput {
        facts: all,
        present: names(&["__NV_DISABLE_EXPLICIT_SYNC"]),
        ..input()
    });
    assert_eq!(session.env, vec![SHM]);
}

#[test]
fn g2b_user_renderer_vars_suppress_every_webkit_var() {
    let fx = fixture();
    fx.write_file("{\"version\":1,\"mode\":\"software\"}");
    for user_var in [
        "WEBKIT_DMABUF_RENDERER_FORCE_SHM",
        "WEBKIT_DISABLE_COMPOSITING_MODE",
    ] {
        for safe_flag in [false, true] {
            let plain = fx.start(&StartInput {
                safe_flag,
                present: names(&[user_var]),
                ..input()
            });
            assert!(plain.user_controlled);
            assert!(plain.env.is_empty(), "{user_var}: {:?}", plain.env);
            assert_eq!(plain.effective, GraphicsMode::Software);
            let nvidia = fx.start(&StartInput {
                safe_flag,
                facts: GraphicsFacts {
                    nvidia: true,
                    wayland: true,
                },
                present: names(&[user_var]),
                ..input()
            });
            assert_eq!(nvidia.env, vec![NV], "{user_var} safe={safe_flag}");
        }
    }
}

#[test]
fn g13_user_control_removes_a_surviving_marker() {
    let fx = fixture();
    fx.write_marker(&marker_json("auto", "stored", 7));
    let session = fx.start(&StartInput {
        present: names(&["WEBKIT_DISABLE_DMABUF_RENDERER"]),
        ..input()
    });
    assert!(!session.stepped_now);
    assert!(!session.marker_written);
    assert!(!fx.paths.marker.exists());
    assert!(!fx.paths.file.exists());
}

#[test]
fn g14_other_instance_changes_nothing() {
    let fx = fixture();
    fx.write_file("{ not json");
    fx.write_marker(&marker_json("auto", "stored", 7));
    let before = support::tree_listing(&fx.root);
    let session = fx.start(&StartInput {
        other_instance: true,
        ..input()
    });
    assert!(!session.marker_written);
    assert!(!session.stepped_now);
    assert_eq!(support::tree_listing(&fx.root), before);
}

#[test]
fn g14b_marker_of_a_live_other_start_changes_nothing() {
    let fx = fixture();
    let marker = marker_json("auto", "stored", 4242);
    fx.write_marker(&marker);
    let session = graphics::start(&fx.paths, &input(), &|pid| pid == 4242);
    assert!(!session.marker_written);
    assert!(!session.stepped_now);
    assert_eq!(session.stored, GraphicsMode::Auto);
    assert_eq!(session.effective, GraphicsMode::Auto);
    assert!(!fx.paths.file.exists());
    assert_eq!(read_text(&fx.paths.marker), marker);
}

#[test]
fn g14c_marker_of_a_dead_or_own_pid_still_steps() {
    let fx = fixture();
    fx.write_marker(&marker_json("auto", "stored", 4242));
    let session = graphics::start(&fx.paths, &input(), &|pid| pid != 4242);
    assert!(session.stepped_now);
    assert_eq!(session.stored, GraphicsMode::ForceShm);

    let fx = fixture();
    fx.write_marker(&marker_json("auto", "stored", PID));
    let session = graphics::start(&fx.paths, &input(), &|_| true);
    assert!(session.stepped_now);
    assert_eq!(session.stored, GraphicsMode::ForceShm);
}

#[test]
fn g15_corrupt_file_is_backed_up_once_and_replaced_by_defaults() {
    let fx = fixture();
    fx.write_file("{ not json");
    let loaded = graphics::load(&fx.paths);
    assert!(loaded.recovered);
    assert!(!loaded.read_only);
    assert_eq!(loaded.file.mode, GraphicsMode::Auto);
    assert_eq!(fx.backups().len(), 1);
    let again = graphics::load(&fx.paths);
    assert!(!again.recovered);
    assert_eq!(fx.backups().len(), 1);

    let fx = fixture();
    fx.write_file("[]");
    let session = fx.start(&input());
    assert!(!session.read_only);
    assert!(session.marker_written);
    assert_eq!(fx.backups().len(), 1);
}

#[test]
fn g16_newer_version_is_read_only() {
    let fx = fixture();
    let newer = "{\"version\":2,\"mode\":\"software\"}";
    fx.write_file(newer);
    let session = fx.start(&input());
    assert!(session.read_only);
    assert!(!session.marker_written);
    assert_eq!(session.effective, GraphicsMode::Auto);
    assert!(!fx.paths.marker.exists());
    assert!(matches!(
        graphics::set_mode(&fx.paths, GraphicsMode::ForceShm),
        Err(GraphicsError::ReadOnly)
    ));
    assert!(matches!(
        graphics::ack_step(&fx.paths, true),
        Err(GraphicsError::ReadOnly)
    ));
    assert!(graphics::load(&fx.paths).read_only);
    assert_eq!(read_text(&fx.paths.file), newer);
}

#[test]
fn g17_clear_marker() {
    let fx = fixture();
    fx.write_marker(&marker_json("auto", "stored", 7));
    graphics::clear_marker(&fx.paths).expect("clear");
    assert!(!fx.paths.marker.exists());
    graphics::clear_marker(&fx.paths).expect("missing is ok");

    fs::create_dir(&fx.paths.marker).expect("marker folder");
    fs::write(fx.paths.marker.join("keep"), b"x").expect("file in folder");
    assert!(graphics::clear_marker(&fx.paths).is_err());
    assert!(fx.paths.marker.join("keep").is_file());
}

#[test]
fn g18_set_mode_and_ack_step() {
    let fx = fixture();
    let file = graphics::set_mode(&fx.paths, GraphicsMode::Software).expect("set");
    assert_eq!(file.mode, GraphicsMode::Software);
    assert_eq!(graphics::load(&fx.paths).file, file);
    assert!(matches!(
        graphics::ack_step(&fx.paths, false),
        Err(GraphicsError::NoStep)
    ));

    let stepped = "{\"version\":1,\"mode\":\"forceShm\",\"stepped\":{\"from\":\"auto\",\
                   \"to\":\"forceShm\",\"at\":\"2026-11-02T10:00:00+01:00\"}}";
    fx.write_file(stepped);
    let kept = graphics::ack_step(&fx.paths, false).expect("keep");
    assert_eq!((kept.mode, kept.stepped), (GraphicsMode::ForceShm, None));
    assert_eq!(
        read_text(&fx.paths.file),
        "{\n  \"version\": 1,\n  \"mode\": \"forceShm\"\n}\n"
    );

    fx.write_file(stepped);
    let reverted = graphics::ack_step(&fx.paths, true).expect("revert");
    assert_eq!(
        (reverted.mode, reverted.stepped),
        (GraphicsMode::Auto, None)
    );
    assert!(matches!(
        graphics::ack_step(&fx.paths, true),
        Err(GraphicsError::NoStep)
    ));

    fx.write_file(stepped);
    let set = graphics::set_mode(&fx.paths, GraphicsMode::Auto).expect("set clears the step");
    assert_eq!(set.stepped, None);
}

#[test]
fn start_without_the_app_lock_writes_nothing() {
    let fx = fixture();
    fx.write_marker(&marker_json("auto", "stored", 7));
    let _held = AppLock::try_exclusive(&fx.app.lock_file).expect("hold the app lock");
    let before = support::tree_listing(&fx.root);
    let session = fx.start(&input());
    assert!(!session.marker_written);
    assert!(!session.stepped_now);
    assert!(!session.notes.is_empty());
    assert_eq!(support::tree_listing(&fx.root), before);
    assert!(matches!(
        graphics::set_mode(&fx.paths, GraphicsMode::Software),
        Err(GraphicsError::Fs(FsError::Busy))
    ));
}

#[test]
fn g19_remove_app_file_refuses_everything_but_a_direct_child_file() {
    let fx = fixture();
    let config = fx.app.config_dir.as_path();
    let outside = fx.root.join("outside.txt");
    fs::write(&outside, b"x").expect("outside file");
    fs::create_dir(config.join("sub")).expect("sub");
    fs::write(config.join("sub").join("nested"), b"x").expect("nested file");
    fs::create_dir(config.join("folder")).expect("folder");
    let before = support::tree_listing(&fx.root);

    assert!(matches!(
        safe_fs::remove_app_file(&outside, &[config]),
        Err(FsError::OutOfScope { .. })
    ));
    assert!(matches!(
        safe_fs::remove_app_file(&config.join("sub").join("nested"), &[config]),
        Err(FsError::OutOfScope { .. })
    ));
    assert!(safe_fs::remove_app_file(&config.join("folder"), &[config]).is_err());
    assert!(safe_fs::remove_app_file(config, &[config]).is_err());
    assert_eq!(support::tree_listing(&fx.root), before);

    assert!(!safe_fs::remove_app_file(&config.join("missing"), &[config]).expect("missing"));
    fs::write(config.join("file"), b"x").expect("file");
    assert!(safe_fs::remove_app_file(&config.join("file"), &[config]).expect("removed"));
    assert!(!config.join("file").exists());
}

#[test]
fn g20_live_session_exists() {
    let fx = fixture();
    assert!(!safe_fs::live_session_exists(&fx.app.data_dir));

    let sessions = fx.app.data_dir.join("sessions");
    fs::create_dir_all(&sessions).expect("sessions dir");
    let stale = sessions.join(format!("{}.lock", "ab".repeat(16)));
    fs::write(&stale, b"").expect("stale session file");
    assert!(!safe_fs::live_session_exists(&fx.app.data_dir));
    assert!(stale.exists(), "the probe never removes a file");

    let gate = OpGate::open(&fx.app).expect("open gate");
    assert!(safe_fs::live_session_exists(&fx.app.data_dir));
    drop(gate);
    assert!(!safe_fs::live_session_exists(&fx.app.data_dir));
}
