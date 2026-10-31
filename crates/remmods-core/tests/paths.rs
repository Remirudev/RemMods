mod support;

use std::path::Path;

use remmods_core::paths::{APP_IDENTIFIER, display_path};
use remmods_core::{AppPaths, Core};

#[test]
fn under_puts_everything_below_one_root() {
    let root = Path::new("smoke-root");

    let paths = AppPaths::under(root);

    assert_eq!(paths.config_dir, root.join("config"));
    assert_eq!(paths.data_dir, root.join("data"));
    assert_eq!(paths.log_dir, root.join("data").join("logs"));
    assert_eq!(
        paths.settings_file,
        root.join("config").join("settings.json")
    );
    assert_eq!(paths.lock_file, root.join("data").join("remmods.lock"));
}

#[test]
fn for_user_folders_end_with_the_app_identifier() {
    let Ok(paths) = AppPaths::for_user() else {
        return;
    };

    assert!(paths.config_dir.ends_with(APP_IDENTIFIER));
    assert!(paths.data_dir.ends_with(APP_IDENTIFIER));
    assert_eq!(paths.log_dir, paths.data_dir.join("logs"));
    assert_eq!(paths.settings_file, paths.config_dir.join("settings.json"));
    assert_eq!(paths.lock_file, paths.data_dir.join("remmods.lock"));
}

#[test]
fn core_open_creates_data_and_log_folders() {
    let (_dir, root) = support::temp_root();
    let paths = AppPaths::under(&root);

    let core = Core::open(paths.clone()).expect("open core");

    assert!(paths.data_dir.is_dir());
    assert!(paths.log_dir.is_dir());
    assert_eq!(core.settings().settings.version, 1);
}

#[test]
fn display_path_keeps_ordinary_paths() {
    let path = Path::new("some").join("folder");

    assert_eq!(display_path(&path), path.display().to_string());
}
