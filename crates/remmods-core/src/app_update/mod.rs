//! Self-update rules (PLAN §10, SPEC-P5 §5.1): which installs may replace themselves, the AUR
//! install-source marker, version comparison, and the seal that keeps mod writes out while an
//! update installs. Pure and read-only: no network, no process, no write.

mod api;
mod error;
mod install_kind;
mod marker;
mod version;

pub use error::{AppUpdateError, UpdateBlock};
pub use install_kind::{
    APPIMAGE_EXTRACTED_PREFIX, APPIMAGE_MOUNT_PREFIX, BundleKind, InstallFacts, InstallKind,
    InstallPermission, InstallSource, appimage_extracted, appimage_target_ok, classify_install,
    install_permission, may_self_install,
};
pub use marker::{INSTALL_SOURCE_FILE, MARKER_MAX_BYTES, marker_path, parse_marker, read_marker};
pub use version::{AppVersion, is_offered};
