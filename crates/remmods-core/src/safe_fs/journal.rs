//! `data_dir/journal.json`: every rename batch is written here before its first rename and
//! removed after its last, so a crash in between is rolled back on the next start. All changes
//! run under the exclusive app lock: re-read, modify, revision + 1, `write_atomic`.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::batch::RenameOp;
use super::names::{self, KeyForm};
use super::{FsError, backup_file, write_atomic};
use crate::platform::{self, RawName};

pub(crate) const JOURNAL_FILE: &str = "journal.json";
const VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JournalProblem {
    /// Written by a newer RemMods: every mutation is refused.
    NewerVersion,
    Unreadable,
    Encode,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecoveryReport {
    /// Entries of dead owners that recovery looked at.
    pub entries: u32,
    /// Rename ops undone (incl. removed link-fallback leftovers).
    pub rolled_back: u32,
    /// Ops that could not be judged safely; their entries stay with `stuck: true`.
    pub stuck: u32,
    /// Entries with a path outside the scope (or not decodable); left as they are.
    pub foreign: u32,
    /// Install staging folders of dead owners that were removed.
    pub discarded: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JournalFile {
    pub(crate) version: u32,
    pub(crate) revision: u64,
    pub(crate) entries: Vec<Entry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Entry {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) owner: Owner,
    pub(crate) created_at: String,
    pub(crate) stuck: bool,
    pub(crate) ops: Vec<OpRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Owner {
    pub(crate) pid: u32,
    pub(crate) session: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum OpRecord {
    #[serde(rename_all = "camelCase")]
    Rename {
        is_dir: bool,
        from: PathRecord,
        to: PathRecord,
    },
    Placeholder {
        path: PathRecord,
    },
    /// An install staging folder, journaled before it is created; `inert` lists the payload
    /// files staging renamed with `-x` (paths inside the folder).
    Staging {
        path: PathRecord,
        #[serde(default)]
        inert: Vec<PathRecord>,
    },
}

impl OpRecord {
    /// Every path the record names.
    pub(crate) fn paths(&self) -> Vec<&PathRecord> {
        match self {
            Self::Rename { from, to, .. } => vec![from, to],
            Self::Placeholder { path } => vec![path],
            Self::Staging { path, inert } => std::iter::once(path).chain(inert).collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PathRecord {
    form: PathForm,
    text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum PathForm {
    Bytes,
    Wide,
}

#[derive(Deserialize)]
struct VersionProbe {
    version: u32,
}

enum Parsed {
    File(JournalFile),
    Newer,
    Corrupt(String),
}

#[derive(Debug)]
pub(crate) struct Journal {
    path: PathBuf,
    sessions: PathBuf,
    session: String,
}

impl Default for JournalFile {
    fn default() -> Self {
        Self {
            version: VERSION,
            revision: 0,
            entries: Vec::new(),
        }
    }
}

impl PathRecord {
    pub(crate) fn encode(path: &Path) -> Self {
        match platform::raw_name(path.as_os_str()) {
            RawName::Bytes(bytes) => Self {
                form: PathForm::Bytes,
                text: names::escape_bytes(&bytes),
            },
            RawName::Wide(units) => Self {
                form: PathForm::Wide,
                text: names::escape_wide(&units),
            },
        }
    }

    pub(crate) fn decode(&self) -> Option<PathBuf> {
        let form = match self.form {
            PathForm::Bytes => KeyForm::Bytes,
            PathForm::Wide => KeyForm::Wide,
        };
        let raw = names::unescape_key(&self.text, form)?;
        platform::os_string(&raw).map(PathBuf::from)
    }
}

impl Journal {
    pub(crate) fn new(data_dir: &Path, sessions: PathBuf, session: String) -> Self {
        Self {
            path: data_dir.join(JOURNAL_FILE),
            sessions,
            session,
        }
    }

    pub(crate) fn session(&self) -> &str {
        &self.session
    }

    pub(crate) fn sessions_dir(&self) -> &Path {
        &self.sessions
    }

    /// Records a batch before its first rename; returns the entry id.
    pub(crate) fn begin(&self, label: &str, ops: &[RenameOp]) -> Result<String, FsError> {
        let id = uuid::Uuid::new_v4().simple().to_string();
        let records = ops
            .iter()
            .map(|op| OpRecord::Rename {
                is_dir: op.is_dir,
                from: PathRecord::encode(&op.from),
                to: PathRecord::encode(&op.to),
            })
            .collect();
        let entry = self.new_entry(&id, label, records);
        self.update(|file| file.entries.push(entry))?;
        Ok(id)
    }

    /// Records an install staging folder before it is created; returns the entry id.
    pub(crate) fn begin_staging(&self, label: &str, path: &Path) -> Result<String, FsError> {
        let id = uuid::Uuid::new_v4().simple().to_string();
        let entry = self.new_entry(
            &id,
            label,
            vec![OpRecord::Staging {
                path: PathRecord::encode(path),
                inert: Vec::new(),
            }],
        );
        self.update(|file| file.entries.push(entry))?;
        Ok(id)
    }

    /// Adds the files staging made inert to the staging record of entry `id`.
    pub(crate) fn add_inert(&self, id: &str, files: &[PathBuf]) -> Result<(), FsError> {
        self.update(|file| {
            let records = file
                .entries
                .iter_mut()
                .filter(|entry| entry.id == id)
                .flat_map(|entry| entry.ops.iter_mut());
            for record in records {
                if let OpRecord::Staging { inert, .. } = record {
                    inert.extend(files.iter().map(|path| PathRecord::encode(path)));
                }
            }
        })
    }

    fn new_entry(&self, id: &str, label: &str, ops: Vec<OpRecord>) -> Entry {
        Entry {
            id: id.to_owned(),
            label: label.to_owned(),
            owner: Owner {
                pid: std::process::id(),
                session: self.session.clone(),
            },
            created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            stuck: false,
            ops,
        }
    }

    /// Records a placeholder folder before the folder fallback creates it.
    pub(crate) fn append_placeholder(&self, id: &str, path: &Path) -> Result<(), FsError> {
        self.update(|file| {
            if let Some(entry) = file.entries.iter_mut().find(|entry| entry.id == id) {
                entry.ops.push(OpRecord::Placeholder {
                    path: PathRecord::encode(path),
                });
            }
        })
    }

    pub(crate) fn remove(&self, id: &str) -> Result<(), FsError> {
        self.update(|file| file.entries.retain(|entry| entry.id != id))
    }

    pub(crate) fn mark_stuck(&self, id: &str) -> Result<(), FsError> {
        self.update(|file| {
            if let Some(entry) = file.entries.iter_mut().find(|entry| entry.id == id) {
                entry.stuck = true;
            }
        })
    }

    /// Entries as they are on disk (under the lock; a corrupt file is backed up and emptied).
    pub(crate) fn entries(&self) -> Result<Vec<Entry>, FsError> {
        self.load().map(|file| file.entries)
    }

    /// Sessions owning an entry, read without the lock and without repairing anything; None when
    /// the file cannot be judged (then nobody's session file may be cleaned).
    pub(crate) fn owners_unlocked(&self) -> Option<HashSet<String>> {
        let bytes = match read_file(&self.path) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return Some(HashSet::new()),
            Err(_) => return None,
        };
        match parse(&bytes) {
            Parsed::File(file) => Some(
                file.entries
                    .into_iter()
                    .map(|entry| entry.owner.session)
                    .collect(),
            ),
            Parsed::Newer | Parsed::Corrupt(_) => None,
        }
    }

    fn update(&self, change: impl FnOnce(&mut JournalFile)) -> Result<(), FsError> {
        let mut file = self.load()?;
        change(&mut file);
        self.save(&mut file)
    }

    fn load(&self) -> Result<JournalFile, FsError> {
        let bytes = match read_file(&self.path) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return Ok(JournalFile::default()),
            Err(error) => {
                log::error!("cannot read {}: {error}", self.path.display());
                return Err(FsError::Journal {
                    reason: JournalProblem::Unreadable,
                });
            }
        };
        match parse(&bytes) {
            Parsed::File(file) => Ok(file),
            Parsed::Newer => Err(FsError::Journal {
                reason: JournalProblem::NewerVersion,
            }),
            Parsed::Corrupt(reason) => {
                let backup = backup_file(&self.path)?;
                log::error!(
                    "{} is corrupt ({reason}); backed up to {} and started empty",
                    self.path.display(),
                    backup.display()
                );
                // Replaced at once, so the next load does not back it up again.
                let mut empty = JournalFile::default();
                self.save(&mut empty)?;
                Ok(empty)
            }
        }
    }

    fn save(&self, file: &mut JournalFile) -> Result<(), FsError> {
        file.revision += 1;
        let bytes = serde_json::to_vec_pretty(file).map_err(|error| {
            log::error!("encoding the journal failed: {error}");
            FsError::Journal {
                reason: JournalProblem::Encode,
            }
        })?;
        write_atomic(&self.path, &bytes)
    }
}

fn read_file(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match platform::fs_path(path).and_then(fs::read) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn parse(bytes: &[u8]) -> Parsed {
    if let Ok(probe) = serde_json::from_slice::<VersionProbe>(bytes)
        && probe.version > VERSION
    {
        return Parsed::Newer;
    }
    match serde_json::from_slice::<JournalFile>(bytes) {
        Ok(file) if file.version == VERSION => Parsed::File(file),
        Ok(file) => Parsed::Corrupt(format!("version {}", file.version)),
        Err(error) => Parsed::Corrupt(error.to_string()),
    }
}
