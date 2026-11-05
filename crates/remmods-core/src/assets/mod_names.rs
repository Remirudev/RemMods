//! Names of items a mod adds: such mods ship small text tables whose keys spell the item, like
//! `TEXT_<tag>_IP_<character>_<slot>_<name>`. Stored per language, table by table, under the
//! table's content hash, so a changed mod replaces only its own table.

use std::collections::{BTreeMap, BTreeSet};

use remmods_pak::hints::ItemRef;
use serde::{Deserialize, Serialize};

pub(crate) const MOD_NAMES_VERSION: u32 = 1;
const ITEM_MARK: &str = "_IP_";
const GENDER_MARKS: [&str; 2] = ["f_", "m_"];

/// Mods spell the same item differently (`f_customislandvacation_04`, `custom_Island_Vacation_04`).
fn squashed(item: &ItemRef) -> ItemRef {
    let name = GENDER_MARKS
        .iter()
        .find_map(|mark| item.name.strip_prefix(mark))
        .unwrap_or(&item.name);
    ItemRef {
        name: name.replace('_', ""),
        ..item.clone()
    }
}

/// The item a text key names, as a key of `ModNames`.
fn item_of_text_key(text_key: &str) -> Option<String> {
    let (_, prefab) = text_key.split_once(ITEM_MARK)?;
    let item = ItemRef::parse(&prefab.to_ascii_lowercase())?;
    Some(squashed(&item).key())
}

/// One table's `item → text`.
pub(crate) fn table_names(records: Vec<(String, String)>) -> BTreeMap<String, String> {
    records
        .into_iter()
        .filter(|(_, text)| !text.is_empty())
        .filter_map(|(key, text)| Some((item_of_text_key(&key)?, text)))
        .collect()
}

/// One `mod-names.<language>.json`.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct ModNames {
    pub version: u32,
    /// Content hash of the table → its names.
    pub tables: BTreeMap<String, BTreeMap<String, String>>,
}

impl ModNames {
    /// The stored tables are exactly the ones the mods carry now.
    pub(crate) fn covers(&self, wanted: &BTreeSet<&String>) -> bool {
        self.tables.len() == wanted.len() && wanted.iter().all(|id| self.tables.contains_key(*id))
    }

    pub(crate) fn lookup(&self, item: &ItemRef) -> Option<String> {
        squashed(item)
            .lookup_keys()
            .find_map(|key| self.tables.values().find_map(|table| table.get(&key)))
            .cloned()
    }
}
