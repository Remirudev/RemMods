use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

use remmods_core::platform;

/// Command line: `--smoke`, `--paks <dir>` (or `--paks=<dir>`), `--safe-graphics`; anything else
/// is ignored.
#[derive(Debug, Default)]
pub struct LaunchArgs {
    pub smoke: bool,
    pub paks: Option<PathBuf>,
    /// Software rendering for this session only (the desktop file's safe-graphics action).
    pub safe_graphics: bool,
    pub ignored: Vec<String>,
}

impl LaunchArgs {
    /// The process's arguments; a relative `--paks` resolves against `$OWD` inside an AppImage.
    pub fn from_env() -> Self {
        let mut parsed = Self::parse(env::args_os().skip(1));
        let owd = env::var_os("OWD");
        let appdir = env::var_os("APPDIR");
        parsed.paks = parsed
            .paks
            .map(|paks| platform::resolve_cli_path(&paks, owd.as_deref(), appdir.as_deref()));
        parsed
    }

    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Self {
        let mut parsed = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.to_str() {
                Some("--smoke") => parsed.smoke = true,
                Some("--safe-graphics") => parsed.safe_graphics = true,
                Some("--paks") => parsed.paks = args.next().map(PathBuf::from),
                Some(flag) if flag.starts_with("--paks=") => {
                    parsed.paks = Some(PathBuf::from(&flag["--paks=".len()..]));
                }
                _ => parsed.ignored.push(arg.to_string_lossy().into_owned()),
            }
        }
        parsed
    }
}
