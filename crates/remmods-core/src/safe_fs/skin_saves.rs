//! The game's five skin save files: the only game files RemMods writes (owner's decision, PLAN
//! §15). Exactly these names, in a folder shaped like `Saved/SaveGames/<id>`, whole files only.

use std::fs::{self, File};
use std::io::{self, ErrorKind, Write};
use std::path::{Component, Path, PathBuf};

use super::FsError;
use super::gate::WriteGuard;
use super::retry::with_retry;
use super::scope::ScopeViolation;
use crate::platform;

pub const SKIN_SAVE_FILES: [&str; 5] = [
    "customize_save_data_0_1.sav",
    "customize_save_data_1_1.sav",
    "customize_save_data_2_1.sav",
    "customize_save_data_3_1.sav",
    "customize_save_data_4_1.sav",
];

/// One whole file per entry of `SKIN_SAVE_FILES`, in that order.
pub type SkinSaves = [Vec<u8>; SKIN_SAVE_FILES.len()];

const SAVES_FOLDER: &str = "SaveGames";
const SAVED_FOLDER: &str = "Saved";
/// Steam Cloud syncs `*.sav`, so a file left behind by a crash stays out of it.
const STAGED_SUFFIX: &str = ".remmods-new";

fn is_folder(component: Option<Component<'_>>, name: &str) -> bool {
    matches!(component, Some(Component::Normal(part)) if part.eq_ignore_ascii_case(name))
}

fn is_account(component: Option<Component<'_>>) -> bool {
    matches!(component, Some(Component::Normal(part))
        if part.to_str().is_some_and(|id| !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit())))
}

/// `…/Saved/SaveGames/<digits>`, absolute, with no `.` or `..` in it.
fn is_save_folder(folder: &Path) -> bool {
    let plain = folder
        .components()
        .all(|part| !matches!(part, Component::CurDir | Component::ParentDir));
    let mut parts = folder.components().rev();
    folder.is_absolute()
        && plain
        && is_account(parts.next())
        && is_folder(parts.next(), SAVES_FOLDER)
        && is_folder(parts.next(), SAVED_FOLDER)
}

fn staged_path(folder: &Path, name: &str) -> PathBuf {
    folder.join(format!(".{name}{STAGED_SUFFIX}"))
}

/// A link in place of a save would send the write somewhere else.
fn check_target(target: &Path) -> Result<(), FsError> {
    match fs::symlink_metadata(target) {
        Ok(meta) if meta.file_type().is_file() => Ok(()),
        Ok(_) => Err(FsError::OutOfScope {
            path: target.to_path_buf(),
            reason: ScopeViolation::LinkComponent,
        }),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(FsError::io("skin_saves", target, error)),
    }
}

fn stage(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = File::create_new(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn discard_staged(folder: &Path) {
    for name in SKIN_SAVE_FILES {
        let staged = staged_path(folder, name);
        match fs::remove_file(&staged) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => log::warn!("staged save {} stays: {error}", staged.display()),
        }
    }
}

/// Best effort: Windows cannot open a folder to sync it.
fn sync_folder(folder: &Path) {
    if let Err(error) = File::open(folder).and_then(|file| file.sync_all()) {
        log::debug!("save folder not synced: {error}");
    }
}

impl WriteGuard<'_> {
    /// All five new files are written next to the old ones first, so a full disk changes nothing;
    /// then each takes its place. The files get a fresh time, which is what Steam Cloud goes by.
    /// `fail_after` is a test hook: stop with an error after that many files took their place.
    /// Anything but a real save folder is refused, nothing written.
    pub fn replace_skin_saves(
        &self,
        folder: &Path,
        files: &SkinSaves,
        fail_after: Option<usize>,
    ) -> Result<(), FsError> {
        if !is_save_folder(folder) {
            return Err(FsError::OutOfScope {
                path: folder.to_path_buf(),
                reason: ScopeViolation::OutsideRoots,
            });
        }
        let raw =
            platform::fs_path(folder).map_err(|error| FsError::io("skin_saves", folder, error))?;
        for name in SKIN_SAVE_FILES {
            check_target(&raw.join(name))?;
        }
        discard_staged(&raw);
        let result = stage_all(&raw, files).and_then(|()| swap_all(&raw, fail_after));
        discard_staged(&raw);
        sync_folder(&raw);
        result
    }
}

fn stage_all(folder: &Path, files: &SkinSaves) -> Result<(), FsError> {
    for (name, bytes) in SKIN_SAVE_FILES.iter().zip(files) {
        let staged = staged_path(folder, name);
        stage(&staged, bytes).map_err(|error| FsError::io("skin_saves", &staged, error))?;
    }
    Ok(())
}

fn swap_all(folder: &Path, fail_after: Option<usize>) -> Result<(), FsError> {
    for (done, name) in SKIN_SAVE_FILES.iter().enumerate() {
        if fail_after == Some(done) {
            let error = io::Error::other("injected fault");
            return Err(FsError::io("skin_saves", folder, error));
        }
        let staged = staged_path(folder, name);
        let target = folder.join(name);
        with_retry(
            "skin_saves",
            &target,
            platform::replace_retry_policy,
            || fs::rename(&staged, &target),
        )?;
    }
    Ok(())
}
