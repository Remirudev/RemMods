//! What the assets folder should hold, given the mods of the current snapshot, and filling it.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use remmods_pak::TocFile;
use remmods_pak::hints::{ItemRef, prefab_item};
use remmods_unpack::{Container, UnpackError};

use super::game_files::{GameFiles, listed_pictures, text_table};
use super::mod_names::{MOD_NAMES_VERSION, ModNames, table_names};
use super::model::{NAME_LANGUAGES, PICTURE_EDGE};
use crate::content::ContentCache;
use crate::mods::ModRow;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PictureJob {
    /// Fs form.
    pub utoc: PathBuf,
    pub asset: u32,
    pub bulk: Option<u32>,
}

/// By picture id.
pub(crate) type PictureJobs = BTreeMap<String, PictureJob>;

/// A text table a mod ships.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TableJob {
    /// Fs form.
    pub utoc: PathBuf,
    pub chunk: u32,
}

/// By language, then content hash.
pub(crate) type TableJobs = BTreeMap<String, BTreeMap<String, TableJob>>;

/// What the mods of a snapshot ask of the assets folder.
#[derive(Debug, Default)]
pub(crate) struct Wanted {
    pub pictures: PictureJobs,
    pub tables: TableJobs,
}

impl Wanted {
    pub(crate) fn tables_of(&self, language: &str) -> BTreeSet<&String> {
        self.tables
            .get(language)
            .into_iter()
            .flat_map(BTreeMap::keys)
            .collect()
    }
}

fn want_tables(tables: &mut TableJobs, utoc: &Path, files: &[TocFile]) {
    for file in files {
        let (Some((language, _)), Some(hash)) = (text_table(&file.path), &file.content_hash) else {
            continue;
        };
        if NAME_LANGUAGES.contains(&language) {
            let job = TableJob {
                utoc: utoc.to_path_buf(),
                chunk: file.chunk_index,
            };
            let id = hash.iter().map(|byte| format!("{byte:02x}")).collect();
            tables
                .entry(language.to_owned())
                .or_default()
                .insert(id, job);
        }
    }
}

fn want_game_picture(jobs: &mut PictureJobs, game: &GameFiles, item: &ItemRef) {
    let Some(picture) = game.picture_of(item) else {
        return;
    };
    let Some(utoc) = game.container(picture.container) else {
        return;
    };
    jobs.entry(picture.key.id()).or_insert_with(|| PictureJob {
        utoc: utoc.to_path_buf(),
        asset: picture.asset,
        bulk: picture.bulk,
    });
}

/// Every game picture of an item some mod touches, every picture and every text table a mod ships.
pub(crate) fn wanted_assets(rows: &[ModRow], cache: &ContentCache, game: &GameFiles) -> Wanted {
    let mut wanted = Wanted::default();
    let jobs = &mut wanted.pictures;
    for row in rows {
        let utocs = cache.row_containers(row.mod_ref.as_str());
        for utoc in utocs.iter().flatten() {
            let result = cache.get_or_read(utoc).0;
            let Ok(container) = result.as_ref() else {
                continue;
            };
            let files = &container.listing.files;
            for picture in listed_pictures(files) {
                want_game_picture(jobs, game, &picture.item);
                jobs.entry(picture.key.id()).or_insert_with(|| PictureJob {
                    utoc: utoc.path.clone(),
                    asset: picture.asset,
                    bulk: picture.bulk,
                });
            }
            for item in files.iter().filter_map(|file| prefab_item(&file.path)) {
                want_game_picture(jobs, game, &item);
            }
            want_tables(&mut wanted.tables, &utoc.path, files);
        }
    }
    wanted
}

/// Opens each container once per refresh; one that fails stays failed for the refresh.
#[derive(Default)]
pub(crate) struct OpenContainers {
    open: HashMap<PathBuf, Option<Container>>,
}

impl OpenContainers {
    fn get(&mut self, utoc: &Path) -> Result<&Container, UnpackError> {
        self.open
            .entry(utoc.to_path_buf())
            .or_insert_with(|| {
                Container::open(utoc)
                    .inspect_err(|error| log::info!("assets: container not opened: {error}"))
                    .ok()
            })
            .as_ref()
            .ok_or(UnpackError::Malformed("container"))
    }

    pub(crate) fn text_table(
        &mut self,
        utoc: &Path,
        chunk: u32,
    ) -> Result<Vec<(String, String)>, UnpackError> {
        self.get(utoc)?.text_table(chunk)
    }

    pub(crate) fn render(&mut self, job: &PictureJob) -> Result<Vec<u8>, UnpackError> {
        let picture = self
            .get(&job.utoc)?
            .picture(job.asset, job.bulk, PICTURE_EDGE)?;
        Ok(picture.png)
    }

    /// `stored` with the tables nobody ships dropped and the new ones read. A table that can't be
    /// read stays empty, so it is not tried again until the mod changes.
    pub(crate) fn updated_mod_names(
        &mut self,
        stored: &ModNames,
        wanted: &BTreeMap<String, TableJob>,
    ) -> ModNames {
        let tables = wanted.iter().map(|(id, job)| {
            let names = stored.tables.get(id).cloned().unwrap_or_else(|| {
                let records = self
                    .text_table(&job.utoc, job.chunk)
                    .unwrap_or_else(|error| {
                        log::info!("assets: mod text table not read: {error}");
                        Vec::new()
                    });
                table_names(records)
            });
            (id.clone(), names)
        });
        ModNames {
            tables: tables.collect(),
            version: MOD_NAMES_VERSION,
        }
    }
}
