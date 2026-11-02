use std::ffi::OsStr;
use std::path::Path;

/// What tauri-bundler patched into the binary (`tauri::utils::platform::bundle_type`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BundleKind {
    AppImage,
    Deb,
    Rpm,
    Nsis,
    Msi,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallKind {
    AppImage,
    Nsis,
    Deb,
    Rpm,
    Aur,
    /// A plain release binary that `cachyos-install.sh` put into a user prefix.
    Local,
    Unknown,
}

/// What installed RemMods, as its marker file says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallSource {
    Aur,
    /// `cachyos-install.sh`, which installs an unbundled release binary.
    Local,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallPermission {
    SelfUpdate,
    Notice,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallFacts {
    pub bundle: Option<BundleKind>,
    pub marker: Option<InstallSource>,
    /// `appimage_target_ok` for this process's environment.
    pub appimage_ok: bool,
}

pub fn classify_install(facts: &InstallFacts) -> InstallKind {
    match facts.bundle {
        Some(BundleKind::AppImage) if facts.appimage_ok => InstallKind::AppImage,
        Some(BundleKind::Deb) if facts.marker == Some(InstallSource::Aur) => InstallKind::Aur,
        Some(BundleKind::Deb) => InstallKind::Deb,
        Some(BundleKind::Rpm) => InstallKind::Rpm,
        Some(BundleKind::Nsis) => InstallKind::Nsis,
        Some(BundleKind::Other) | None if facts.marker == Some(InstallSource::Local) => {
            InstallKind::Local
        }
        Some(BundleKind::AppImage | BundleKind::Msi | BundleKind::Other) | None => {
            InstallKind::Unknown
        }
    }
}

/// The hard allowlist: only AppImage and NSIS installs replace themselves. No `_` arm, so a new
/// kind cannot default to self-update.
pub fn install_permission(kind: InstallKind) -> InstallPermission {
    match kind {
        InstallKind::AppImage | InstallKind::Nsis => InstallPermission::SelfUpdate,
        InstallKind::Deb
        | InstallKind::Rpm
        | InstallKind::Aur
        | InstallKind::Local
        | InstallKind::Unknown => InstallPermission::Notice,
    }
}

/// Checked with a fresh `bundle_type()` right before the plugin installs, so its package-manager
/// paths (dpkg, rpm, pkexec) stay unreachable even if the stored kind were wrong.
pub fn may_self_install(kind: InstallKind, bundle: Option<BundleKind>) -> bool {
    matches!(
        (kind, bundle),
        (InstallKind::AppImage, Some(BundleKind::AppImage))
            | (InstallKind::Nsis, Some(BundleKind::Nsis))
    )
}

/// The folder name prefix of a FUSE-mounted AppImage (`$APPDIR`).
pub const APPIMAGE_MOUNT_PREFIX: &str = ".mount_";
/// The folder name prefix the type-2 runtime extracts to for `--appimage-extract-and-run`.
pub const APPIMAGE_EXTRACTED_PREFIX: &str = "appimage_extracted_";

/// This process may let the plugin replace `$APPIMAGE` only when it really runs from that
/// AppImage: `$APPDIR` is an absolute runtime folder (a `.mount_*` FUSE mount or an
/// `appimage_extracted_*` extract-and-run folder), the running executable `exe` lies inside it,
/// and `$APPIMAGE` names an absolute regular file outside it. An inherited `APPIMAGE`/`APPDIR`
/// (an extracted copy started from another AppImage's shell) fails, so the plugin never
/// overwrites another app's file or the current executable.
pub fn appimage_target_ok(
    appimage: Option<&OsStr>,
    appdir: Option<&OsStr>,
    exe: Option<&Path>,
    is_regular_file: impl Fn(&Path) -> bool,
) -> bool {
    let Some(appdir) = runtime_appdir(appdir) else {
        return false;
    };
    if !exe.is_some_and(|exe| exe.starts_with(appdir)) {
        return false;
    }
    let Some(appimage) = appimage.filter(|value| !value.is_empty()) else {
        return false;
    };
    let target = Path::new(appimage);
    target.is_absolute() && is_regular_file(target) && !target.starts_with(appdir)
}

/// True when `$APPDIR` is an `--appimage-extract-and-run` folder: a relaunch must ask the runtime
/// to extract again, since FUSE may be missing.
pub fn appimage_extracted(appdir: Option<&OsStr>) -> bool {
    runtime_appdir(appdir).is_some_and(|appdir| name_starts_with(appdir, APPIMAGE_EXTRACTED_PREFIX))
}

fn runtime_appdir(appdir: Option<&OsStr>) -> Option<&Path> {
    let appdir = Path::new(appdir.filter(|value| !value.is_empty())?);
    let runtime_folder = name_starts_with(appdir, APPIMAGE_MOUNT_PREFIX)
        || name_starts_with(appdir, APPIMAGE_EXTRACTED_PREFIX);
    (appdir.is_absolute() && runtime_folder).then_some(appdir)
}

fn name_starts_with(path: &Path, prefix: &str) -> bool {
    path.file_name()
        .is_some_and(|name| name.as_encoded_bytes().starts_with(prefix.as_bytes()))
}
