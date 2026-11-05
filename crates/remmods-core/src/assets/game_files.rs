//! Where the game keeps item pictures and text tables, collected while the base index reads the
//! game's containers. A later container wins, so a patch (`_0_P`) overrides its base.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::{Path, PathBuf};

use remmods_pak::hints::{ItemRef, picture_item};
use remmods_pak::{ContentHash, TocFile, TocListing};

use super::model::{NAME_LANGUAGES, PICTURE_EDGE};

const LOCALIZE: &str = "Polaris/Content/Localize/";
const TEXT_TABLE_STEM: &str = "GTB_";
const ASSET_EXTENSION: &str = ".uasset";
const BULK_EXTENSION: &str = ".ubulk";
/// Half of the stored id comes from each file.
const HASH_PART: usize = 8;

/// A picture's content: the first bytes of its `.uasset` and `.ubulk` hashes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PictureKey([u8; 2 * HASH_PART]);

impl PictureKey {
    fn new(asset: &ContentHash, bulk: Option<&ContentHash>) -> Self {
        let mut key = [0; 2 * HASH_PART];
        key[..HASH_PART].copy_from_slice(&asset[..HASH_PART]);
        if let Some(bulk) = bulk {
            key[HASH_PART..].copy_from_slice(&bulk[..HASH_PART]);
        }
        Self(key)
    }

    /// The stored picture's id; a new edge size asks for new files.
    pub(crate) fn id(self) -> String {
        format!("{PICTURE_EDGE}-{self}")
    }
}

impl fmt::Display for PictureKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|byte| write!(f, "{byte:02x}"))
    }
}

/// A menu picture in one container's listing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ListedPicture {
    pub item: ItemRef,
    pub asset: u32,
    pub bulk: Option<u32>,
    pub key: PictureKey,
}

fn bulk_of<'a>(files: &'a [TocFile], asset_path: &str) -> Option<&'a TocFile> {
    let stem = asset_path.get(..asset_path.len().checked_sub(ASSET_EXTENSION.len())?)?;
    let wanted = format!("{stem}{BULK_EXTENSION}");
    let at = files
        .binary_search_by(|file| file.path.as_str().cmp(&wanted))
        .ok()?;
    files.get(at)
}

/// `files` sorted by display path, as a listing keeps them. Without a content hash a picture has
/// no id, so it is left out.
pub(crate) fn listed_pictures(files: &[TocFile]) -> impl Iterator<Item = ListedPicture> {
    files.iter().filter_map(|file| {
        let item = picture_item(&file.path)?;
        let bulk = bulk_of(files, &file.path);
        let asset_hash = file.content_hash.as_ref()?;
        let bulk_hash = match bulk {
            Some(bulk) => Some(bulk.content_hash.as_ref()?),
            None => None,
        };
        Some(ListedPicture {
            item,
            asset: file.chunk_index,
            bulk: bulk.map(|bulk| bulk.chunk_index),
            key: PictureKey::new(asset_hash, bulk_hash),
        })
    })
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct GamePicture {
    pub container: usize,
    pub asset: u32,
    pub bulk: Option<u32>,
    pub key: PictureKey,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct GameTable {
    pub container: usize,
    pub chunk: u32,
}

/// `(language, table)` of a text table's display path.
pub(crate) fn text_table(path: &str) -> Option<(&str, &str)> {
    let (language, file) = path.strip_prefix(LOCALIZE)?.split_once('/')?;
    let table = file.strip_suffix(ASSET_EXTENSION)?;
    table
        .starts_with(TEXT_TABLE_STEM)
        .then_some((language, table))
}

#[derive(Debug, Default)]
pub(crate) struct GameFiles {
    /// Fs form.
    containers: Vec<PathBuf>,
    /// By `ItemRef::key`.
    pictures: HashMap<String, GamePicture>,
    /// Language, then table name, so tables read in a fixed order and later updates win.
    tables: HashMap<String, BTreeMap<String, GameTable>>,
}

impl GameFiles {
    pub(crate) fn add(&mut self, utoc: &Path, listing: &TocListing) {
        let container = self.containers.len();
        self.containers.push(utoc.to_path_buf());
        for picture in listed_pictures(&listing.files) {
            let entry = GamePicture {
                container,
                asset: picture.asset,
                bulk: picture.bulk,
                key: picture.key,
            };
            self.pictures.insert(picture.item.key(), entry);
        }
        for file in &listing.files {
            if let Some((language, table)) = text_table(&file.path)
                && NAME_LANGUAGES.contains(&language)
            {
                let entry = GameTable {
                    container,
                    chunk: file.chunk_index,
                };
                self.tables
                    .entry(language.to_owned())
                    .or_default()
                    .insert(table.to_owned(), entry);
            }
        }
    }

    pub(crate) fn picture_of(&self, item: &ItemRef) -> Option<&GamePicture> {
        item.lookup_keys().find_map(|key| self.pictures.get(&key))
    }

    pub(crate) fn tables(&self, language: &str) -> impl Iterator<Item = &GameTable> {
        self.tables
            .get(language)
            .into_iter()
            .flat_map(BTreeMap::values)
    }

    pub(crate) fn container(&self, index: usize) -> Option<&Path> {
        self.containers.get(index).map(PathBuf::as_path)
    }
}
