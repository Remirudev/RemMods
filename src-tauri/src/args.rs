use std::ffi::OsString;
use std::path::PathBuf;

/// Command line: `--smoke`, `--paks <dir>` (or `--paks=<dir>`); anything else is ignored.
#[derive(Debug, Default)]
pub struct LaunchArgs {
    pub smoke: bool,
    pub paks: Option<PathBuf>,
    pub ignored: Vec<String>,
}

impl LaunchArgs {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Self {
        let mut parsed = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.to_str() {
                Some("--smoke") => parsed.smoke = true,
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
