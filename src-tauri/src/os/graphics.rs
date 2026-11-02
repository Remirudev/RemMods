//! Linux graphics fallback (PLAN §5): picks the WebKitGTK renderer variables before GTK starts
//! and clears the crash marker once the frontend ran. Windows: every function is a no-op.

#[cfg(target_os = "linux")]
pub use linux::{log_startup, on_exit, on_frontend_ready, prepare, session};
#[cfg(not(target_os = "linux"))]
pub use other::{log_startup, on_exit, on_frontend_ready, prepare, session};

pub fn supported() -> bool {
    cfg!(target_os = "linux")
}

#[cfg(not(target_os = "linux"))]
mod other {
    use remmods_core::AppPaths;
    use remmods_core::graphics::GraphicsSession;
    use tauri::AppHandle;

    use crate::args::LaunchArgs;

    pub fn prepare(_paths: &AppPaths, _args: &LaunchArgs) {}

    pub fn log_startup() {}

    pub fn session() -> Option<&'static GraphicsSession> {
        None
    }

    pub fn on_frontend_ready(_app: &AppHandle) {}

    pub fn on_exit() {}
}

#[cfg(target_os = "linux")]
mod linux {
    use std::env;
    use std::path::Path;
    use std::process;
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::Duration;

    use remmods_core::graphics::{
        self, GraphicsFacts, GraphicsPaths, GraphicsSession, MANAGED_VARS, StartInput,
        USER_CONTROL_VARS,
    };
    use remmods_core::{AppPaths, platform, safe_fs};
    use tauri::AppHandle;

    use crate::args::LaunchArgs;

    /// The window may show through the 3 s fallback while the page is still blank; only a
    /// frontend that called `app_ready` and kept running this long counts as a good start.
    const MARKER_CLEAR_DELAY: Duration = Duration::from_secs(10);
    const NVIDIA_PROBES: [&str; 2] = ["/sys/module/nvidia_drm", "/proc/driver/nvidia/version"];

    static SESSION: OnceLock<GraphicsSession> = OnceLock::new();
    static PATHS: OnceLock<GraphicsPaths> = OnceLock::new();
    static FRONTEND_READY: AtomicBool = AtomicBool::new(false);

    /// Runs in `run()` before the log plugin, the Tauri builder, GTK or any other thread.
    pub fn prepare(paths: &AppPaths, args: &LaunchArgs) {
        let graphics_paths = GraphicsPaths::from_app(paths);
        let input = StartInput {
            safe_flag: args.safe_graphics,
            facts: facts(),
            present: present_vars(),
            other_instance: safe_fs::live_session_exists(&paths.data_dir),
            pid: process::id(),
            now: graphics::now_rfc3339(),
        };
        let session = graphics::start(&graphics_paths, &input, &other_remmods_alive);
        let mut own = Vec::with_capacity(session.env.len());
        for (name, value) in &session.env {
            // SAFETY: runs in run() before the log plugin, the Tauri builder, GTK or any other
            // thread exists, so nothing reads the environment concurrently.
            unsafe { env::set_var(name, value) };
            own.push(*name);
        }
        platform::set_own_env_vars(own);
        let _ = PATHS.set(graphics_paths);
        let _ = SESSION.set(session);
    }

    pub fn log_startup() {
        let Some(session) = SESSION.get() else {
            return;
        };
        log::info!(
            "graphics: stored {:?}, effective {:?} from {:?}, env {:?}, stepped now {}, user env \
             {}, marker {}, read-only {}",
            session.stored,
            session.effective,
            session.source,
            session.env,
            session.stepped_now,
            session.user_controlled,
            session.marker_written,
            session.read_only
        );
        for note in &session.notes {
            log::warn!("graphics: {note}");
        }
    }

    pub fn session() -> Option<&'static GraphicsSession> {
        SESSION.get()
    }

    /// Once: if this session wrote a marker, a thread clears it after [`MARKER_CLEAR_DELAY`].
    pub fn on_frontend_ready(_app: &AppHandle) {
        if FRONTEND_READY.swap(true, Ordering::SeqCst) || !marker_written() {
            return;
        }
        let started = thread::Builder::new()
            .name("graphics-marker".to_owned())
            .spawn(|| {
                thread::sleep(MARKER_CLEAR_DELAY);
                clear_marker("after a good start");
            });
        if let Err(error) = started {
            log::warn!("cannot start the graphics marker thread: {error}");
        }
    }

    /// An exit before `app_ready` keeps the marker: it counts as a failed start.
    pub fn on_exit() {
        if FRONTEND_READY.load(Ordering::SeqCst) && marker_written() {
            clear_marker("on exit");
        }
    }

    fn marker_written() -> bool {
        SESSION.get().is_some_and(|session| session.marker_written)
    }

    fn clear_marker(when: &str) {
        let Some(paths) = PATHS.get() else {
            return;
        };
        match graphics::clear_marker(paths) {
            Ok(()) => log::debug!("graphics marker cleared {when}"),
            Err(error) => log::warn!("clearing the graphics marker {when} failed: {error}"),
        }
    }

    /// Whether `pid` is another running process with this process's `comm`. Compared with our own
    /// name, not a literal, so AppImage starts (through AppRun) match too. A reused pid of an
    /// unrelated program does not match, so a real crash still steps.
    fn other_remmods_alive(pid: u32) -> bool {
        let own = process::id();
        pid != own && comm_of(pid).is_some_and(|comm| Some(comm) == comm_of(own))
    }

    fn comm_of(pid: u32) -> Option<String> {
        let path = Path::new("/proc").join(pid.to_string()).join("comm");
        std::fs::read(path)
            .ok()
            .map(|raw| platform::parse_comm(&raw))
    }

    fn facts() -> GraphicsFacts {
        GraphicsFacts {
            nvidia: NVIDIA_PROBES.iter().any(|probe| Path::new(probe).exists()),
            wayland: graphics::detect_wayland(
                env::var_os("WAYLAND_DISPLAY").as_deref(),
                env::var_os("XDG_SESSION_TYPE").as_deref(),
                env::var_os("GDK_BACKEND").as_deref(),
            ),
        }
    }

    fn present_vars() -> Vec<String> {
        let mut present: Vec<String> = Vec::new();
        for name in MANAGED_VARS.iter().chain(USER_CONTROL_VARS.iter()) {
            if env::var_os(name).is_some() && !present.iter().any(|known| known == name) {
                present.push((*name).to_owned());
            }
        }
        present
    }
}
