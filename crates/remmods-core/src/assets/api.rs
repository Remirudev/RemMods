use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use super::game_files::GameFiles;
use super::items::{NameSources, mod_items};
use super::model::{AssetsStatus, ModItems, NAME_LANGUAGES};
use super::names::{build_names, item_list, names_stamp};
use super::own_picture::read_own_picture;
use super::refresh::{OpenContainers, PictureJobs, Wanted, wanted_assets};
use crate::mods::manage_target::{Managed, resolve_managed};
use crate::mods::{ModTarget, ModsError, ScanContent};
use crate::{Core, CoreError};

impl Core {
    /// Brings the assets folder in line with the game and the mods (slow the first time: meant
    /// for a background thread). Reads only; nothing new is read while the game runs. `true` when
    /// pictures or names changed.
    pub fn assets_refresh(&self) -> Result<bool, CoreError> {
        let _turn = self.assets.refresh_turn();
        let snapshot = self.mods_list()?;
        let paks = &snapshot.content.paks;
        let Some(base) = self.mods.stored_base_index(paks) else {
            return Ok(false);
        };
        let game = base.game_files();
        let stamp = names_stamp(base.stamp(), paks);
        let wanted = wanted_assets(&snapshot.content.rows, self.mods.contents(), game);
        let stale = self.stale_languages(&stamp, game);
        let stale_mods = self.stale_mod_languages(&wanted);
        let missing = wanted
            .pictures
            .keys()
            .any(|id| self.assets.needs_picture(id));
        let work = missing || !stale.is_empty() || !stale_mods.is_empty();
        if work && self.game_running_now() {
            return Ok(false);
        }
        let names = self.refresh_names(&stale, stamp, game, paks)?;
        let mod_names = self.refresh_mod_names(&stale_mods, &wanted)?;
        let pictures = self.fill_pictures(&wanted.pictures)?;
        self.remove_unused_own_pictures();
        Ok(names || mod_names || pictures)
    }

    fn remove_unused_own_pictures(&self) {
        let loaded = self.mods.meta().load();
        if loaded.read_only {
            return;
        }
        let entries = loaded.meta.mods.values();
        let used: HashSet<&str> = entries
            .filter_map(|entry| entry.picture.as_deref())
            .collect();
        self.assets.remove_other_own(&used);
    }

    /// Makes the image at `source` the mod's own picture: it stays in RemMods' folder, so moving
    /// the file later changes nothing, and the link follows renames.
    pub fn mods_set_picture(&self, target: &ModTarget, source: &Path) -> Result<(), CoreError> {
        let (format, bytes) =
            read_own_picture(source).map_err(|problem| ModsError::Picture { problem })?;
        let _turn = self.assets.refresh_turn();
        self.run_managed(|guard, _, content| {
            let key = row_key(content, target)?;
            let id = self.assets.add_own_picture(format, &bytes)?;
            self.mods
                .meta()
                .update(guard, |meta| meta.set_picture(&key, Some(id)))?;
            Ok(())
        })?;
        Ok(())
    }

    /// The mod goes back to the pictures of its items.
    pub fn mods_clear_picture(&self, target: &ModTarget) -> Result<(), CoreError> {
        let _turn = self.assets.refresh_turn();
        self.run_managed(|guard, _, content| {
            let key = row_key(content, target)?;
            self.mods
                .meta()
                .update(guard, |meta| meta.set_picture(&key, None))?;
            Ok(())
        })?;
        Ok(())
    }

    /// Languages whose stored names are missing or from another game state.
    fn stale_languages(&self, stamp: &str, game: &GameFiles) -> Vec<&'static str> {
        NAME_LANGUAGES
            .iter()
            .filter(|language| game.tables(language).next().is_some())
            .filter(|language| {
                self.assets
                    .names(language)
                    .is_none_or(|names| names.stamp != stamp)
            })
            .copied()
            .collect()
    }

    fn stale_mod_languages(&self, wanted: &Wanted) -> Vec<&'static str> {
        NAME_LANGUAGES
            .iter()
            .filter(|language| {
                !self
                    .assets
                    .mod_names(language)
                    .covers(&wanted.tables_of(language))
            })
            .copied()
            .collect()
    }

    fn refresh_mod_names(&self, stale: &[&str], wanted: &Wanted) -> Result<bool, CoreError> {
        let mut containers = OpenContainers::default();
        let none = BTreeMap::new();
        for language in stale {
            let jobs = wanted.tables.get(*language).unwrap_or(&none);
            let stored = self.assets.mod_names(language);
            let table = {
                let _read = self.ops.read()?;
                containers.updated_mod_names(&stored, jobs)
            };
            self.assets.store_mod_names(language, table)?;
        }
        Ok(!stale.is_empty())
    }

    fn refresh_names(
        &self,
        stale: &[&str],
        stamp: String,
        game: &GameFiles,
        paks: &Path,
    ) -> Result<bool, CoreError> {
        if stale.is_empty() {
            return Ok(false);
        }
        let list = item_list(paks);
        let mut containers = OpenContainers::default();
        for language in stale {
            let table = build_names(language, stamp.clone(), game, &list, &mut containers);
            self.assets.store_names(language, table)?;
        }
        Ok(true)
    }

    fn fill_pictures(&self, jobs: &PictureJobs) -> Result<bool, CoreError> {
        let mut containers = OpenContainers::default();
        let mut added = false;
        for (id, job) in jobs {
            if !self.assets.needs_picture(id) {
                continue;
            }
            // Taken per picture: a toggle waits for one picture, not for the whole fill.
            let read = self.ops.read()?;
            let rendered = containers.render(job);
            drop(read);
            match rendered {
                Ok(png) => {
                    self.assets.add_picture(id, &png)?;
                    added = true;
                }
                Err(error) => {
                    log::info!("assets: picture {id} not read: {error}");
                    self.assets.mark_failed(id);
                }
            }
        }
        Ok(self.assets.remove_others(jobs) || added)
    }

    /// The items one row of the current snapshot touches. Pictures and names appear once the
    /// background refresh has stored them; `language` falls back to English.
    pub fn mods_items(&self, mod_ref: &str, language: &str) -> Result<ModItems, CoreError> {
        let snapshot = self.mods_list()?;
        let row = snapshot
            .content
            .rows
            .iter()
            .find(|row| row.mod_ref.as_str() == mod_ref)
            .ok_or(ModsError::Stale)?;
        let cache = self.mods.contents();
        let utocs = cache.row_containers(mod_ref).ok_or(ModsError::Stale)?;
        let base = self.mods.stored_base_index(&snapshot.content.paks);
        let language = name_language(language);
        let game_names = self.assets.names(language);
        let mod_names = self.assets.mod_names(language);
        let english = self.assets.mod_names(NAME_LANGUAGES[0]);
        let names = NameSources {
            game: game_names.as_deref(),
            mods: &[&mod_names, &english],
        };
        let loaded = self.mods.meta().load();
        Ok(mod_items(
            &row.mod_ref,
            &utocs,
            cache,
            base.as_deref(),
            &names,
            &self.assets,
            loaded.meta.picture(&row.key).map(str::to_owned),
        ))
    }

    /// A stored picture as a `data:` URL, the way images reach the UI; `None` when it is gone.
    pub fn assets_picture(&self, id: &str) -> Option<String> {
        let (mime, bytes) = self.assets.read_picture(id)?;
        Some(format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
    }

    pub fn assets_status(&self, language: &str) -> AssetsStatus {
        self.assets.status(name_language(language))
    }
}

fn row_key(content: &ScanContent, target: &ModTarget) -> Result<String, CoreError> {
    match resolve_managed(content, target)? {
        Managed::Row(row) => Ok(row.key.clone()),
        _ => Err(ModsError::Stale.into()),
    }
}

fn name_language(language: &str) -> &str {
    NAME_LANGUAGES
        .iter()
        .find(|known| **known == language)
        .copied()
        .unwrap_or(NAME_LANGUAGES[0])
}
