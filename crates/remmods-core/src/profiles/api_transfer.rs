//! Import from the original app, export and import of RemMods files (spec §5.9). Paths come
//! from the shell's native dialogs, never from IPC.

use std::path::Path;

use super::api::build_view;
use super::import_tmm::{MergeCounts, merge_tmm, parse_tmm_profiles, read_import_file};
use super::model::now_rfc3339;
use super::transfer::{ExportReport, ImportReport, export_bytes, merge_import, parse_export};
use crate::{Core, CoreError};

impl Core {
    pub fn profiles_import_tmm(&self, file: &Path) -> Result<ImportReport, CoreError> {
        let bytes = read_import_file(file)?;
        let (imported, skipped) = parse_tmm_profiles(&bytes)?;
        let counts = {
            let guard = self.ops.write(self.app_scope()?)?;
            self.profiles
                .update(&guard, |profiles| Ok(merge_tmm(profiles, imported)))?
        };
        self.import_report(counts, skipped)
    }

    pub fn profiles_import(&self, file: &Path) -> Result<ImportReport, CoreError> {
        let bytes = read_import_file(file)?;
        let (imported, skipped) = parse_export(&bytes)?;
        let counts = {
            let guard = self.ops.write(self.app_scope()?)?;
            self.profiles
                .update(&guard, |profiles| Ok(merge_import(profiles, imported)))?
        };
        self.import_report(counts, skipped)
    }

    /// Writes the non-snapshot profiles to the file the user picked (never into Paks, the mod
    /// roots, the app folders or the original app's folder).
    pub fn profiles_export(&self, file: &Path) -> Result<ExportReport, CoreError> {
        let loaded = self.profiles.load();
        let bytes = export_bytes(&loaded.file.profiles, &now_rfc3339());
        let count = loaded
            .file
            .profiles
            .iter()
            .filter(|profile| !profile.is_snapshot())
            .count();
        let input = self.mods_scan_input().ok();
        let scope = match &input {
            Some(input) => self.manage_scope(input)?,
            None => self.app_scope()?,
        };
        let mut forbidden: Vec<&Path> = Vec::new();
        if let Some(input) = &input {
            forbidden.push(&input.paks);
        }
        if let Some(tmm_dir) = self.tmm_dir() {
            forbidden.push(tmm_dir);
        }
        {
            let guard = self.ops.write(scope)?;
            guard.write_export(file, &bytes, &forbidden)?;
        }
        Ok(ExportReport {
            count: u32::try_from(count).unwrap_or(u32::MAX),
            file_name: file
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
        })
    }

    fn import_report(&self, counts: MergeCounts, skipped: u32) -> Result<ImportReport, CoreError> {
        let snapshot = self.mods_list().ok();
        let view = build_view(
            self.profiles.load(),
            snapshot.as_ref().map(|snapshot| &snapshot.content),
        );
        Ok(ImportReport {
            added: counts.added,
            updated: counts.updated,
            unchanged: counts.unchanged,
            skipped,
            view,
        })
    }
}
