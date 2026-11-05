//! Builders for the P4 install tests (spec §8.2): a `Core` on a TempDir game with a TempDir
//! trash, crafted archives in `<root>/src`, starts with the obvious picks, and the "unchanged"
//! state (Paks + Trash, journal entries, app data).
// Fixtures are written into TempDirs, so the std writers are fine here.
#![allow(clippy::disallowed_methods, clippy::expect_used, dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use remmods_core::install::{
    CandidatePick, ClashPick, InspectOptions, Inspection, InstallError, InstallOutcome,
    InstallSource, IssueCode, IssueSeverity, StartRequest,
};
use remmods_core::{CoreError, platform};

use crate::archive_support::{CraftEntry, ZipCraft};
use crate::mods_support::manage::{core_with_trash, core_without_trash, tree};
use crate::mods_support::toggle::ToggleFixture;

pub struct InstallFixture {
    pub fx: ToggleFixture,
    /// Where crafted archives and source folders go (outside the game).
    pub src: PathBuf,
}

/// Paks below `game`, a trash in the TempDir, `entries` below Paks.
pub fn fixture(entries: &[(&str, &[u8])]) -> InstallFixture {
    wrap(core_with_trash(entries))
}

/// The same with `CoreOptions::default()`'s trash env, which refuses every trash.
pub fn fixture_without_trash(entries: &[(&str, &[u8])]) -> InstallFixture {
    wrap(core_without_trash(entries))
}

fn wrap(fx: ToggleFixture) -> InstallFixture {
    let src = fx.root.join("src");
    fs::create_dir_all(&src).expect("src dir");
    InstallFixture { fx, src }
}

/// A stored zip of plain files (`/` in names as given).
pub fn zip_bytes(files: &[(&str, &[u8])]) -> Vec<u8> {
    files
        .iter()
        .fold(ZipCraft::new(), |zip, (name, data)| {
            zip.with(CraftEntry::file(name, data))
        })
        .bytes()
}

impl InstallFixture {
    pub fn zip(&self, name: &str, files: &[(&str, &[u8])]) -> PathBuf {
        let path = self.src.join(name);
        fs::write(&path, zip_bytes(files)).expect("write zip");
        path
    }

    pub fn inspect(&self, path: &Path) -> Inspection {
        self.inspect_with(path, &InspectOptions::default())
    }

    pub fn inspect_with(&self, path: &Path, options: &InspectOptions) -> Inspection {
        self.fx
            .core
            .install_inspect(&InstallSource::Path(path.to_path_buf()), options)
            .expect("install_inspect")
    }

    /// Starts with every selected candidate (replace where a match exists), auto fix on, every
    /// clash keeping its first path.
    pub fn start(&self, inspection: &Inspection) -> Result<InstallOutcome, CoreError> {
        self.fx
            .core
            .install_start(&default_request(inspection), &|_| {})
    }

    pub fn start_with(&self, request: &StartRequest) -> Result<InstallOutcome, CoreError> {
        self.fx.core.install_start(request, &|_| {})
    }

    /// Paks + Trash, journal entries and the app data (the "unchanged" check).
    pub fn state(&self) -> State {
        State {
            tree: tree(&self.fx),
            journal: journal_entries(&self.fx.paths.data_dir),
            app: app_listing(&self.fx.paths.data_dir),
        }
    }

    pub fn exists(&self, rel: &str) -> bool {
        self.fx.exists(rel)
    }

    pub fn read(&self, rel: &str) -> Vec<u8> {
        let path = crate::support::join_rel(&self.fx.paks, rel);
        fs::read(platform::fs_path(&path).expect("fs path")).expect("read")
    }

    pub fn meta(&self) -> serde_json::Value {
        let bytes = fs::read(self.fx.paths.data_dir.join("mod-meta.json")).expect("mod-meta.json");
        serde_json::from_slice(&bytes).expect("mod-meta.json is JSON")
    }

    /// Names below a mod root that look like install temp folders.
    pub fn temp_folders(&self, root: &str) -> Vec<String> {
        let folder = crate::support::join_rel(&self.fx.paks, root);
        if !folder.exists() {
            return Vec::new();
        }
        crate::support::names_in(&folder)
            .into_iter()
            .filter(|name| name.starts_with('_') && name.contains("_tmp_"))
            .collect()
    }
}

pub fn default_request(inspection: &Inspection) -> StartRequest {
    StartRequest {
        inspection: inspection.id.clone(),
        picks: inspection
            .candidates
            .iter()
            .filter(|candidate| candidate.selected)
            .map(|candidate| CandidatePick {
                id: candidate.id,
                replace: candidate.existing.is_some(),
            })
            .collect(),
        clashes: inspection
            .clashes
            .iter()
            .map(|clash| ClashPick {
                id: clash.id,
                keep: 0,
            })
            .collect(),
        auto_fix: true,
        apply_while_running: false,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct State {
    pub tree: Vec<String>,
    pub journal: Vec<serde_json::Value>,
    pub app: Vec<String>,
}

/// Entries of `journal.json` (empty when it does not exist).
pub fn journal_entries(data_dir: &Path) -> Vec<serde_json::Value> {
    crate::support::journal_json(&data_dir.join("journal.json"))
        .and_then(|json| json["entries"].as_array().cloned())
        .unwrap_or_default()
}

/// App data without the journal (compared by its entries), session lock files and logs.
fn app_listing(data_dir: &Path) -> Vec<String> {
    crate::support::tree_listing(data_dir)
        .into_iter()
        .filter(|line| {
            !line.contains("journal.json")
                && !line.contains("sessions")
                && !line.contains("logs")
                && !line.contains("remmods.lock")
        })
        .collect()
}

pub fn install_error<T: std::fmt::Debug>(result: Result<T, CoreError>) -> InstallError {
    match result {
        Err(CoreError::Install(error)) => error,
        other => panic!("expected an InstallError, got {other:?}"),
    }
}

pub fn has_issue(inspection: &Inspection, severity: IssueSeverity, code: IssueCode) -> bool {
    inspection
        .issues
        .iter()
        .any(|issue| issue.severity == severity && issue.code == code)
}

pub fn candidate<'a>(
    inspection: &'a Inspection,
    name: &str,
) -> &'a remmods_core::install::Candidate {
    inspection
        .candidates
        .iter()
        .find(|candidate| candidate.name == name)
        .unwrap_or_else(|| panic!("no candidate {name}: {:?}", inspection.candidates))
}

pub fn row_state(fx: &ToggleFixture, key: &str) -> remmods_core::mods::ModState {
    fx.row(key).state
}
