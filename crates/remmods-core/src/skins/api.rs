use std::path::{Path, PathBuf};

use super::error::SkinsError;
use super::locate::{SaveFolder, newest_account, read_five, read_game_saves};
use super::model::{
    LinkedSkins, SKIN_SET_NAME_MAX_CHARS, SKIN_SET_NOTES_MAX_CHARS, SkinCopy, SkinSet, SkinSetView,
    SkinSetsFile, SkinsView,
};
use super::store::SkinStore;
use crate::assets::read_own_picture;
use crate::game::{GameLocation, STEAM_APP_ID};
use crate::mods::ModsError;
use crate::platform;
use crate::profiles::now_rfc3339;
use crate::safe_fs::names::fold_key;
use crate::safe_fs::{SkinSaves, WriteGuard};
use crate::{Core, CoreError};

/// SteamID64 of account 0: the folder `userdata/<account>` is named by the difference.
const STEAM_ID64_BASE: u64 = 76_561_197_960_265_728;
const SCREENSHOT_PATH: [&str; 3] = ["760", "remote", "screenshots"];

fn new_set(name: String) -> SkinSet {
    let now = now_rfc3339();
    SkinSet {
        id: uuid::Uuid::new_v4().simple().to_string(),
        name,
        notes: String::new(),
        created: now.clone(),
        updated: now,
        screenshots: Vec::new(),
        extra: serde_json::Map::new(),
    }
}

/// Trimmed, not empty, not too long, and no other set has it in any case.
fn checked_name(file: &SkinSetsFile, name: &str, own: Option<&str>) -> Result<String, SkinsError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > SKIN_SET_NAME_MAX_CHARS {
        return Err(SkinsError::InvalidName);
    }
    let key = fold_key(name);
    let taken = file
        .sets
        .iter()
        .any(|set| Some(set.id.as_str()) != own && fold_key(&set.name) == key);
    if taken {
        return Err(SkinsError::NameTaken);
    }
    Ok(name.to_owned())
}

fn set_mut<'a>(file: &'a mut SkinSetsFile, id: &str) -> Result<&'a mut SkinSet, SkinsError> {
    file.sets
        .iter_mut()
        .find(|set| set.id == id)
        .ok_or(SkinsError::UnknownSet)
}

impl Core {
    /// `--smoke` and tests only ever see the folder they were given, never real saves.
    fn save_games_root(&self) -> Option<PathBuf> {
        if self.save_root.is_some() || self.require_marker.is_some() {
            return self.save_root.clone();
        }
        match self.game_status().location {
            GameLocation::Ready(install) => platform::save_games_root(&install.root, STEAM_APP_ID),
            GameLocation::NotSet | GameLocation::Missing { .. } => None,
        }
    }

    fn save_folder(&self) -> Option<SaveFolder> {
        newest_account(&self.save_games_root()?)
    }

    fn skin_store(&self, folder: &SaveFolder) -> SkinStore {
        SkinStore::new(&self.paths.data_dir, &folder.account)
    }

    fn view_of(&self, folder: &SaveFolder) -> SkinsView {
        let store = self.skin_store(folder);
        let loaded = store.load();
        let sets = loaded.file.sets.into_iter().map(|set| SkinSetView {
            bytes: store.set_bytes(&set.id),
            set,
        });
        SkinsView {
            account: Some(folder.account.clone()),
            available: true,
            sets: sets.collect(),
            active: loaded.file.active,
            interrupted: loaded.file.switching,
            read_only: loaded.read_only,
        }
    }

    /// The account's sets. Empty and not `available` when the game's skin files can't be found.
    pub fn skins_view(&self) -> SkinsView {
        self.save_folder()
            .map_or_else(SkinsView::default, |folder| self.view_of(&folder))
    }

    /// Under the write gate: the list, `change`, the list back.
    fn change_sets(
        &self,
        change: impl FnOnce(
            &WriteGuard<'_>,
            &SaveFolder,
            &SkinStore,
            &mut SkinSetsFile,
        ) -> Result<(), SkinsError>,
    ) -> Result<SkinsView, CoreError> {
        let folder = self.save_folder().ok_or(SkinsError::NoSaves)?;
        {
            let guard = self.ops.write(self.app_scope()?)?;
            let store = self.skin_store(&folder);
            let loaded = store.load();
            if loaded.read_only {
                return Err(SkinsError::ReadOnly.into());
            }
            let mut file = loaded.file;
            change(&guard, &folder, &store, &mut file)?;
            store.save(&guard, &file)?;
        }
        Ok(self.view_of(&folder))
    }

    fn refuse_while_running(&self) -> Result<(), SkinsError> {
        if self.game.maybe_running_now() {
            return Err(SkinsError::GameRunning);
        }
        Ok(())
    }

    /// Copies the game's five files into a new set, which then counts as the one in the game.
    /// The game has to be closed: it rewrites a whole file when a slot changes.
    pub fn skins_save_new(&self, name: &str) -> Result<SkinsView, CoreError> {
        self.change_sets(|guard, folder, store, file| {
            self.refuse_while_running()?;
            if file.switching.is_some() {
                return Err(SkinsError::Interrupted);
            }
            let set = new_set(checked_name(file, name, None)?);
            store.write_set(guard, &set.id, &read_game_saves(folder)?)?;
            file.active = Some(set.id.clone());
            file.sets.push(set);
            Ok(())
        })
    }

    /// The automatic copies, newest first.
    pub fn skins_copies(&self) -> Vec<SkinCopy> {
        let Some(folder) = self.save_folder() else {
            return Vec::new();
        };
        let store = self.skin_store(&folder);
        let sets = store.load().file.sets;
        let name_of = |id: &str| {
            let set = sets.iter().find(|set| set.id == id)?;
            Some(set.name.clone())
        };
        let copies = store.copies().into_iter().map(|copy| SkinCopy {
            set_name: copy.set.as_deref().and_then(name_of),
            id: copy.id,
            taken: copy.taken,
        });
        copies.collect()
    }

    /// A new set from an automatic copy. The game's files stay as they are until a switch to it,
    /// so this also works during an unfinished switch.
    pub fn skins_keep_copy(&self, copy: &str, name: &str) -> Result<SkinsView, CoreError> {
        self.change_sets(|guard, _, store, file| {
            let saves = store.read_copy(copy)?;
            let set = new_set(checked_name(file, name, None)?);
            store.write_set(guard, &set.id, &saves)?;
            file.sets.push(set);
            Ok(())
        })
    }

    pub fn skins_edit(&self, id: &str, name: &str, notes: &str) -> Result<SkinsView, CoreError> {
        self.change_sets(|_, _, _, file| {
            if notes.chars().count() > SKIN_SET_NOTES_MAX_CHARS {
                return Err(SkinsError::NotesTooLong);
            }
            let name = checked_name(file, name, Some(id))?;
            let set = set_mut(file, id)?;
            set.name = name;
            notes.clone_into(&mut set.notes);
            set.updated = now_rfc3339();
            Ok(())
        })
    }

    /// The set in the game stays: its copy is what a switch away would keep.
    pub fn skins_delete(&self, id: &str) -> Result<SkinsView, CoreError> {
        self.change_sets(|_, _, store, file| {
            set_mut(file, id)?;
            if file.active.as_deref() == Some(id) || file.switching.as_deref() == Some(id) {
                return Err(SkinsError::SetInUse);
            }
            store.delete_set(id)?;
            file.sets.retain(|set| set.id != id);
            Ok(())
        })
    }

    pub fn skins_add_screenshot(&self, id: &str, source: &Path) -> Result<SkinsView, CoreError> {
        let (format, bytes) =
            read_own_picture(source).map_err(|problem| ModsError::Picture { problem })?;
        let _turn = self.assets.refresh_turn();
        self.change_sets(|_, _, _, file| {
            let set = set_mut(file, id)?;
            set.screenshots
                .push(self.assets.add_own_picture(format, &bytes)?);
            set.updated = now_rfc3339();
            Ok(())
        })
    }

    /// The picture file goes with the next refresh of the assets folder.
    pub fn skins_remove_screenshot(&self, id: &str, shot: &str) -> Result<SkinsView, CoreError> {
        self.change_sets(|_, _, _, file| {
            let set = set_mut(file, id)?;
            set.screenshots.retain(|known| known != shot);
            set.updated = now_rfc3339();
            Ok(())
        })
    }

    /// Steam's folder of this game's screenshots, for the file picker; only when it exists.
    pub fn skins_screenshot_folder(&self) -> Option<PathBuf> {
        let folder = self.save_folder()?;
        let account = folder
            .account
            .parse::<u64>()
            .ok()?
            .checked_sub(STEAM_ID64_BASE)?;
        let GameLocation::Ready(install) = self.game_status().location else {
            return None;
        };
        let steam = install.root.parent()?.parent()?.parent()?;
        let [first, second, last] = SCREENSHOT_PATH;
        let path = steam
            .join("userdata")
            .join(account.to_string())
            .join(first)
            .join(second)
            .join(STEAM_APP_ID.to_string())
            .join(last);
        path.is_dir().then_some(path)
    }

    /// Puts `incoming` in place and compares what landed. False leaves the files unknown.
    fn install_saves(
        &self,
        guard: &WriteGuard<'_>,
        folder: &SaveFolder,
        incoming: &SkinSaves,
    ) -> bool {
        let fault = self.take_skin_fault();
        if let Err(error) = guard.replace_skin_saves(&folder.path, incoming, fault) {
            log::error!("skin saves not replaced: {error}");
            return false;
        }
        let same = read_five(&folder.path).as_ref() == Some(incoming);
        if !same {
            log::error!("skin saves differ from the set after the switch");
        }
        same
    }

    /// The game's files go into the set they belong to and into an automatic copy, then the
    /// other set's files take their place and are compared. A failure puts the old files back;
    /// if even that fails the list keeps saying so, and this call finishes the job later. The
    /// game must be closed, also for a group switch: it would write over the files on exit.
    /// That is checked behind the gate, because the wait for it can be long.
    pub fn skins_switch(&self, id: &str) -> Result<SkinsView, CoreError> {
        self.swap_saves(id, false)
    }

    /// The stored copy of the set in the game goes back in, for when the game's files went wrong
    /// (a reinstall, the game's "delete saved data"). They stay as an automatic copy and replace
    /// no stored copy.
    pub fn skins_put_back(&self, id: &str) -> Result<SkinsView, CoreError> {
        self.swap_saves(id, true)
    }

    fn swap_saves(&self, id: &str, put_back: bool) -> Result<SkinsView, CoreError> {
        let folder = self.save_folder().ok_or(SkinsError::NoSaves)?;
        {
            let guard = self.ops.write(self.app_scope()?)?;
            self.swap_under_guard(&guard, &folder, id, put_back)?;
        }
        Ok(self.view_of(&folder))
    }

    fn swap_under_guard(
        &self,
        guard: &WriteGuard<'_>,
        folder: &SaveFolder,
        id: &str,
        put_back: bool,
    ) -> Result<(), SkinsError> {
        self.refuse_while_running()?;
        let store = self.skin_store(folder);
        let loaded = store.load();
        if loaded.read_only {
            return Err(SkinsError::ReadOnly);
        }
        let mut file = loaded.file;
        set_mut(&mut file, id)?;
        let interrupted = file.switching.is_some();
        let in_game = file.active.as_deref() == Some(id);
        if in_game && !interrupted && !put_back {
            return Ok(());
        }
        let incoming = store.read_set(id)?;
        let current = if interrupted {
            // The game's files may be a mix, so they replace no stored copy. They can also hold
            // skins saved since the switch broke, so they are not dropped either.
            if let Some(files) = read_five(&folder.path) {
                store.add_backup(guard, &files)?;
            }
            None
        } else {
            let current = read_game_saves(folder)?;
            if in_game && current == incoming {
                return Ok(());
            }
            store.add_backup(guard, &current)?;
            if !in_game && let Some(active) = file.active.clone() {
                store.refresh_set(guard, &active, &current)?;
                set_mut(&mut file, &active)?.updated = now_rfc3339();
            }
            Some(current)
        };
        // The copies take a moment, and nothing keeps the game from starting meanwhile.
        self.refuse_while_running()?;
        file.switching = Some(id.to_owned());
        store.save(guard, &file)?;
        if self.install_saves(guard, folder, &incoming) {
            file.active = Some(id.to_owned());
            file.switching = None;
            return store.save(guard, &file);
        }
        // A refused replace changed nothing, so the old files can be in place without a write.
        let restored = current.is_some_and(|current| {
            self.install_saves(guard, folder, &current)
                || read_five(&folder.path).as_ref() == Some(&current)
        });
        if restored {
            file.switching = None;
            store.save(guard, &file)?;
        }
        Err(SkinsError::SwitchFailed { restored })
    }

    /// The set in the game, for the automatic snapshot before a group switch.
    pub(crate) fn active_skin_set(&self) -> Option<String> {
        self.skins_view().active
    }

    /// After a group with a linked set was switched on. A link to a set that is gone means
    /// nothing, and the mods stay switched whatever happens here.
    pub(crate) fn switch_linked_skins(&self, id: &str) -> Option<LinkedSkins> {
        let view = self.skins_view();
        let name = view
            .sets
            .iter()
            .find(|set| set.set.id == id)?
            .set
            .name
            .clone();
        if view.active.as_deref() == Some(id) {
            return None;
        }
        Some(match self.skins_switch(id) {
            Ok(_) => LinkedSkins::Switched { name },
            Err(CoreError::Skins(SkinsError::GameRunning)) => LinkedSkins::GameRunning { name },
            Err(error) => {
                log::error!("linked skin set not switched: {error}");
                LinkedSkins::Failed { name }
            }
        })
    }
}
