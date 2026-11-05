//! The items of one mod row, with their stored pictures and names.

use std::collections::{BTreeMap, HashMap, HashSet};

use remmods_pak::hints::{ItemRef, prefab_item};

use super::game_files::{GameFiles, PictureKey, listed_pictures};
use super::mod_names::ModNames;
use super::model::{ItemNameSource, ModItem, ModItems};
use super::names::ItemNames;
use super::store::AssetStore;
use crate::content::{ContentCache, UtocRef};
use crate::mods::ModRef;

struct RowPicture {
    item: ItemRef,
    key: PictureKey,
    live: bool,
}

/// What the row's containers carry.
#[derive(Default)]
struct RowItems {
    /// Item → live.
    items: BTreeMap<ItemRef, bool>,
    /// By `ItemRef::key`.
    pictures: HashMap<String, RowPicture>,
}

fn collect(utocs: &[UtocRef], cache: &ContentCache) -> RowItems {
    let mut found = RowItems::default();
    for utoc in utocs {
        let result = cache.get_or_read(utoc).0;
        let Ok(container) = result.as_ref() else {
            continue;
        };
        let files = &container.listing.files;
        for item in files.iter().filter_map(|file| prefab_item(&file.path)) {
            *found.items.entry(item).or_default() |= utoc.live;
        }
        for picture in listed_pictures(files) {
            let stored = found
                .pictures
                .entry(picture.item.key())
                .or_insert(RowPicture {
                    item: picture.item,
                    key: picture.key,
                    live: utoc.live,
                });
            if utoc.live && !stored.live {
                stored.key = picture.key;
                stored.live = true;
            }
        }
    }
    found
}

/// A picture no prefab of the row claims is an item of its own: hidden pieces ship only pictures.
fn add_picture_only_items(found: &mut RowItems) {
    let claimed: HashSet<String> = found
        .items
        .keys()
        .flat_map(ItemRef::lookup_keys)
        .filter(|key| found.pictures.contains_key(key))
        .collect();
    for (key, picture) in &found.pictures {
        if !claimed.contains(key) {
            *found.items.entry(picture.item.clone()).or_default() |= picture.live;
        }
    }
}

/// The game's own name first: a row says what it replaces. Names a mod ships fall back to its
/// English table.
pub(crate) struct NameSources<'a> {
    pub game: Option<&'a ItemNames>,
    pub mods: &'a [&'a ModNames],
}

impl NameSources<'_> {
    fn name_of(&self, item: &ItemRef) -> (Option<String>, ItemNameSource) {
        if let Some((text, source)) = self.game.and_then(|game| game.lookup(item)) {
            return (Some(text), source);
        }
        match self.mods.iter().find_map(|mods| mods.lookup(item)) {
            Some(text) => (Some(text), ItemNameSource::ModText),
            None => (None, ItemNameSource::Internal),
        }
    }
}

pub(crate) fn mod_items(
    mod_ref: &ModRef,
    utocs: &[UtocRef],
    cache: &ContentCache,
    game: Option<&GameFiles>,
    names: &NameSources<'_>,
    store: &AssetStore,
) -> ModItems {
    let mut found = collect(utocs, cache);
    add_picture_only_items(&mut found);
    let stored = |key: PictureKey| Some(key.id()).filter(|id| store.has_picture(id));
    let items = found
        .items
        .iter()
        .map(|(item, &live)| {
            let own = item
                .lookup_keys()
                .find_map(|key| found.pictures.get(&key))
                .map(|picture| picture.key);
            let (name, name_source) = names.name_of(item);
            ModItem {
                key: item.key(),
                character: item.character.clone(),
                slot: item.slot.clone(),
                name,
                name_source,
                game_picture: game
                    .and_then(|game| game.picture_of(item))
                    .and_then(|picture| stored(picture.key)),
                mod_picture: own.and_then(stored),
                live,
            }
        })
        .collect();
    ModItems {
        mod_ref: mod_ref.clone(),
        items,
    }
}
