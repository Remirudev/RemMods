//! The read-only half of the install facade (spec §5.3): `install_inspect`, `install_cancel` and
//! the test hooks. `install_start` lives in `api_start.rs`.

use std::path::{Path, PathBuf};

use super::error::InstallError;
use super::inspect::{InspectInput, inspect};
use super::model::{InspectOptions, Inspection, InstallSource, SourceKind};
use super::service::{InstallHooks, StoredInspection};
use super::source::{classify_source, source_names};
use crate::archive::{self, ArchiveError, ArchiveReader, Limits};
use crate::mods::{ModRef, ModsError, ScanContent};
use crate::{Core, CoreError, platform};

impl Core {
    /// Lists the source, plans the candidates and stores the inspection for a start. Takes no
    /// write guard and writes nothing.
    pub fn install_inspect(
        &self,
        source: &InstallSource,
        options: &InspectOptions,
    ) -> Result<Inspection, CoreError> {
        let input = self.mods_scan_input()?;
        let fresh = self.mods_refresh()?;
        let content = &fresh.snapshot.content;
        let (path, folder) = match source {
            InstallSource::Path(path) => {
                let kind = classify_source(path, Some(&input.paks))
                    .map_err(|reason| InstallError::Source { reason })?;
                (path.clone(), kind == SourceKind::Folder)
            }
            InstallSource::ArchiveRow { mod_ref } => (archive_row_path(content, mod_ref)?, false),
        };
        let limits = Limits::default();
        let reader = open_source(&path, folder, &limits)?;
        let fingerprint = if folder {
            None
        } else {
            let fingerprint = archive::fingerprint(&path).map_err(|source| ArchiveError::Io {
                op: "fingerprint",
                class: platform::classify(&source),
                source,
            });
            Some(fingerprint.map_err(InstallError::from)?)
        };
        let listing = reader.listing();
        let entries = archive::normalize(listing, options.encoding, &limits);
        let (source_name, source_stem) = source_names(&path, folder);
        let stored = inspect(&InspectInput {
            source: &path,
            folder,
            fingerprint,
            source_name,
            source_stem,
            options,
            listing,
            entries: &entries,
            content,
            scan_input: &input,
            available_override: self.install.hooks().available_space,
        });
        let inspection = stored.inspection.clone();
        self.install.store(stored);
        Ok(inspection)
    }

    /// Cancels the running install (true) or does nothing (false). Never waits for the OpGate.
    pub fn install_cancel(&self) -> bool {
        self.install.cancel()
    }

    #[doc(hidden)]
    pub fn inject_install_hooks(&self, hooks: InstallHooks) {
        self.install.set_hooks(hooks);
    }
}

/// An archive row inside Paks (read-only source).
fn archive_row_path(content: &ScanContent, token: &str) -> Result<PathBuf, CoreError> {
    let wanted = ModRef::from_token(token);
    let entry = content
        .archives
        .iter()
        .find(|entry| entry.mod_ref == wanted)
        .ok_or(ModsError::Stale)?;
    Ok(entry
        .rel_path
        .split('/')
        .filter(|part| !part.is_empty())
        .fold(content.paks.clone(), |path, part| path.join(part)))
}

pub(crate) fn open_source(
    path: &Path,
    folder: bool,
    limits: &Limits,
) -> Result<Box<dyn ArchiveReader>, InstallError> {
    let reader = if folder {
        archive::open_folder(path, limits)
    } else {
        archive::open(path, limits)
    };
    Ok(reader?)
}

pub(crate) fn fingerprint_of(path: &Path) -> Result<archive::SourceFingerprint, InstallError> {
    archive::fingerprint(path).map_err(|_| InstallError::SourceChanged)
}

/// The source is still the inspected one (folders: checked per file during extraction).
pub(crate) fn check_fingerprint(stored: &StoredInspection) -> Result<(), InstallError> {
    let Some(expected) = stored.fingerprint else {
        return Ok(());
    };
    if fingerprint_of(&stored.source)? == expected {
        Ok(())
    } else {
        Err(InstallError::SourceChanged)
    }
}
