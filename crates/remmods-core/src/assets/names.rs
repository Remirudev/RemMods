//! Item names: the game's text tables, joined to items by the item list the TKData mod loader
//! extracts next to the game (when the user has it), else by matching text keys against internal
//! names. Stored once per game state, per language.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use remmods_pak::hints::{ItemRef, prefab_key};
use remmods_unpack::parse_item_list;
use serde::{Deserialize, Serialize};

use super::game_files::GameFiles;
use super::model::ItemNameSource;
use super::refresh::OpenContainers;
use crate::platform;

pub(crate) const NAMES_VERSION: u32 = 1;
const ITEM_LIST_PATH: [&str; 4] = [
    "Binary",
    "Mods",
    "fbsdata_extract",
    "customize_item_common_list.bin",
];
const MAX_ITEM_LIST_BYTES: u64 = 64 * 1024 * 1024;
const TEXT_KEY_PREFIX: &str = "TEXT_UI_";

/// One `names.<language>.json`.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct ItemNames {
    pub version: u32,
    /// `names_stamp` of the game state this was built from.
    pub stamp: String,
    /// `ItemRef::key` → text, from the item list.
    listed: HashMap<String, String>,
    /// `<prefix>_<slot>_<name>` → text, from the text keys.
    matched: HashMap<String, String>,
}

/// `<name>_v<number>` is a colour variant: the game names it like the item itself.
fn base_of_variant(item: &ItemRef) -> Option<ItemRef> {
    let (name, number) = item.name.rsplit_once("_v")?;
    let digits = !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit());
    digits.then(|| ItemRef {
        name: name.to_owned(),
        ..item.clone()
    })
}

impl ItemNames {
    fn exact(&self, item: &ItemRef) -> Option<&String> {
        self.listed.get(&item.key())
    }

    fn matched(&self, item: &ItemRef) -> Option<&String> {
        item.lookup_keys().find_map(|key| self.matched.get(&key))
    }

    pub(crate) fn lookup(&self, item: &ItemRef) -> Option<(String, ItemNameSource)> {
        if let Some(text) = self.exact(item) {
            return Some((text.clone(), ItemNameSource::ItemList));
        }
        let base = base_of_variant(item);
        self.matched(item)
            .or_else(|| base.as_ref().and_then(|base| self.exact(base)))
            .or_else(|| base.as_ref().and_then(|base| self.matched(base)))
            .map(|text| (text.clone(), ItemNameSource::Matched))
    }

    pub(crate) fn len(&self) -> usize {
        self.listed.len() + self.matched.len()
    }
}

/// `TEXT_UI_<number>_<rest>` → `<rest>`, lowercase.
fn matched_key(text_key: &str) -> Option<String> {
    let (number, rest) = text_key.strip_prefix(TEXT_KEY_PREFIX)?.split_once('_')?;
    let digits = !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit());
    digits.then(|| rest.to_ascii_lowercase())
}

/// `Polaris/Content/Binary/Mods/…`, next to Paks; absent without the mod loader.
fn item_list_path(paks: &Path) -> Option<PathBuf> {
    let content = paks.parent()?.to_path_buf();
    Some(
        ITEM_LIST_PATH
            .iter()
            .fold(content, |path, part| path.join(part)),
    )
}

/// `(prefab, text key)` pairs; empty without the file.
pub(crate) fn item_list(paks: &Path) -> Vec<(String, String)> {
    let read = || -> Option<Vec<u8>> {
        let path = platform::fs_path(&item_list_path(paks)?).ok()?;
        let small = fs::metadata(&path).ok()?.len() <= MAX_ITEM_LIST_BYTES;
        small.then(|| fs::read(&path).ok()).flatten()
    };
    let Some(bytes) = read() else {
        return Vec::new();
    };
    parse_item_list(&bytes).unwrap_or_else(|error| {
        log::info!("assets: item list not read: {error}");
        Vec::new()
    })
}

/// Changes with the game's containers and with the item list.
pub(crate) fn names_stamp(base_stamp: u64, paks: &Path) -> String {
    let list = item_list_path(paks)
        .and_then(|path| platform::fs_path(&path).ok())
        .and_then(|path| fs::metadata(path).ok())
        .map(|meta| (meta.len(), meta.modified().ok()));
    let mut hasher = DefaultHasher::new();
    (NAMES_VERSION, base_stamp, list).hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn read_tables(
    language: &str,
    game: &GameFiles,
    containers: &mut OpenContainers,
) -> Vec<(String, String)> {
    let mut records = Vec::new();
    for table in game.tables(language) {
        let Some(utoc) = game.container(table.container) else {
            continue;
        };
        match containers.text_table(utoc, table.chunk) {
            Ok(found) => records.extend(found),
            Err(error) => log::info!("assets: text table not read: {error}"),
        }
    }
    records
}

pub(crate) fn build_names(
    language: &str,
    stamp: String,
    game: &GameFiles,
    item_list: &[(String, String)],
    containers: &mut OpenContainers,
) -> ItemNames {
    let mut texts = HashMap::new();
    let mut matched = HashMap::new();
    for (key, text) in read_tables(language, game, containers) {
        if text.is_empty() {
            continue;
        }
        if let Some(rule) = matched_key(&key) {
            matched.insert(rule, text.clone());
        }
        texts.insert(key, text);
    }
    let listed = item_list
        .iter()
        .filter_map(|(prefab, key)| Some((prefab_key(prefab)?, texts.get(key)?.clone())))
        .collect();
    ItemNames {
        version: NAMES_VERSION,
        stamp,
        listed,
        matched,
    }
}
