//! Skin sets: whole copies of the five skin files, switched with the game closed. Every test works
//! in a TempDir shaped like the game's save folder; no test sees real saves.
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod mods_support;
mod profiles_support;
mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use mods_support::DATA;
use mods_support::toggle::{Extras, PlannedScan, ToggleFixture};
use profiles_support::{apply, by_name, snapshots, switch, switch_together};
use remmods_core::CoreError;
use remmods_core::assets::PictureProblem;
use remmods_core::mods::ModsError;
use remmods_core::profiles::{
    ApplyDirection::{Off, On},
    ApplyRequest,
    EntryState::Enabled,
    ProfileMode::Additive,
    ProfileSave,
};
use remmods_core::safe_fs::{FsError, SKIN_SAVE_FILES, Scope};
use remmods_core::skins::{LinkedSkins, SKIN_BACKUPS_KEPT, SkinsError, SkinsView};

const ACCOUNT: &str = "76561198000000001";
const STEAM_BASE: u64 = 76_561_197_960_265_728;
const OTHER_SAVES: [&str; 3] = ["global1.sav", "replay_game_no1.sav", "ghost_0_0.sav"];
const PNG: &[u8] = b"\x89PNG\r\n\x1a\nrest";

struct Skins {
    fixture: ToggleFixture,
    root: PathBuf,
    folder: PathBuf,
}

fn save_root(root: &Path) -> PathBuf {
    root.join("TEKKEN 8").join("Saved").join("SaveGames")
}

fn write_saves(folder: &Path, tag: &str) {
    fs::create_dir_all(folder).expect("folder");
    for (index, name) in SKIN_SAVE_FILES.iter().enumerate() {
        fs::write(folder.join(name), format!("{tag}-{index}")).expect("save");
    }
}

fn saves(folder: &Path) -> Vec<Vec<u8>> {
    SKIN_SAVE_FILES
        .iter()
        .map(|name| fs::read(folder.join(name)).expect("read save"))
        .collect()
}

fn tagged(tag: &str) -> Vec<Vec<u8>> {
    (0..SKIN_SAVE_FILES.len())
        .map(|index| format!("{tag}-{index}").into_bytes())
        .collect()
}

fn skins_with(entries: &[(&str, &[u8])]) -> Skins {
    let fixture = ToggleFixture::build(entries, |root| Extras {
        save_root: Some(save_root(root)),
        ..Extras::default()
    });
    let root = fixture.root.clone();
    let folder = save_root(&root).join(ACCOUNT);
    write_saves(&folder, "first");
    for name in OTHER_SAVES {
        fs::write(folder.join(name), name).expect("other save");
    }
    Skins {
        fixture,
        root,
        folder,
    }
}

fn skins() -> Skins {
    skins_with(&[])
}

impl Skins {
    fn core(&self) -> &remmods_core::Core {
        &self.fixture.core
    }

    fn view(&self) -> SkinsView {
        self.core().skins_view()
    }

    fn save_new(&self, name: &str) -> String {
        let view = self.core().skins_save_new(name).expect("save set");
        view.active.expect("the new set is active")
    }

    /// Two sets: `first` stored, `second` in the game.
    fn two_sets(&self) -> (String, String) {
        let first = self.save_new("First");
        write_saves(&self.folder, "second");
        let second = self.save_new("Second");
        (first, second)
    }

    fn store(&self) -> PathBuf {
        self.fixture.paths.data_dir.join("skin-sets").join(ACCOUNT)
    }

    fn stored(&self, id: &str) -> Vec<Vec<u8>> {
        saves(&self.store().join(id))
    }

    fn copies_in(folder: &Path) -> Vec<Vec<Vec<u8>>> {
        let mut names: Vec<_> = fs::read_dir(folder)
            .map(|entries| entries.flatten().map(|entry| entry.path()).collect())
            .unwrap_or_default();
        names.sort();
        names.iter().map(|path| saves(path)).collect()
    }

    fn backups(&self) -> Vec<Vec<Vec<u8>>> {
        Self::copies_in(&self.store().join("backups"))
    }

    fn replaced(&self, id: &str) -> Vec<Vec<Vec<u8>>> {
        Self::copies_in(&self.store().join("replaced").join(id))
    }

    fn other_saves_untouched(&self) {
        for name in OTHER_SAVES {
            let bytes = fs::read(self.folder.join(name)).expect("other save");
            assert_eq!(bytes, name.as_bytes(), "{name}");
        }
    }

    fn folder_listing(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(&self.folder)
            .expect("list")
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect();
        names.sort();
        names
    }
}

fn skin_error(result: Result<SkinsView, CoreError>) -> SkinsError {
    match result {
        Err(CoreError::Skins(error)) => error,
        other => panic!("{other:?}"),
    }
}

#[test]
fn without_the_game_files_there_is_nothing_to_offer() {
    let skins = skins();
    fs::remove_file(skins.folder.join(SKIN_SAVE_FILES[2])).expect("remove one");

    let view = skins.view();
    assert!(!view.available);
    assert_eq!(view.account, None);
    assert!(matches!(
        skin_error(skins.core().skins_save_new("Set")),
        SkinsError::NoSaves
    ));
}

#[test]
fn saving_copies_the_five_files_and_marks_the_set() {
    let skins = skins();
    let id = skins.save_new("Fine mods");

    let view = skins.view();
    assert!(view.available);
    assert_eq!(view.account.as_deref(), Some(ACCOUNT));
    assert_eq!(view.active.as_deref(), Some(id.as_str()));
    assert_eq!(view.sets[0].set.name, "Fine mods");
    assert!(view.sets[0].bytes > 0);
    assert_eq!(skins.stored(&id), tagged("first"));
    assert_eq!(
        saves(&skins.folder),
        tagged("first"),
        "the game's files stay"
    );
    skins.other_saves_untouched();
}

#[test]
fn a_switch_swaps_the_five_files_and_nothing_else() {
    let skins = skins();
    let (first, second) = skins.two_sets();

    let view = skins.core().skins_switch(&first).expect("switch");

    assert_eq!(view.active.as_deref(), Some(first.as_str()));
    assert_eq!(saves(&skins.folder), tagged("first"));
    assert_eq!(skins.stored(&second), tagged("second"));
    skins.other_saves_untouched();
    let listed = skins.folder_listing();
    assert_eq!(listed.len(), SKIN_SAVE_FILES.len() + OTHER_SAVES.len());
    assert!(
        !listed.iter().any(|name| name.contains("remmods")),
        "{listed:?}"
    );
}

#[test]
fn what_changed_in_the_game_stays_in_the_set_it_belongs_to() {
    let skins = skins();
    let (first, second) = skins.two_sets();
    write_saves(&skins.folder, "edited");

    skins.core().skins_switch(&first).expect("away");
    assert_eq!(skins.stored(&second), tagged("edited"));

    skins.core().skins_switch(&second).expect("back");
    assert_eq!(saves(&skins.folder), tagged("edited"));
    assert_eq!(skins.stored(&first), tagged("first"));
}

#[test]
fn a_running_game_stops_every_write_to_its_files() {
    let skins = skins();
    let (first, second) = skins.two_sets();
    skins.fixture.running.store(true, Ordering::SeqCst);

    assert!(matches!(
        skin_error(skins.core().skins_switch(&first)),
        SkinsError::GameRunning
    ));
    assert!(matches!(
        skin_error(skins.core().skins_save_new("Third")),
        SkinsError::GameRunning
    ));
    assert!(matches!(
        skin_error(skins.core().skins_put_back(&second)),
        SkinsError::GameRunning
    ));
    assert_eq!(saves(&skins.folder), tagged("second"));
    assert_eq!(skins.view().sets.len(), 2);
}

#[test]
fn a_failed_process_scan_counts_as_a_running_game() {
    let skins = skins();
    let (first, _) = skins.two_sets();
    skins.fixture.plan_scans(&[PlannedScan::Failed]);

    assert!(matches!(
        skin_error(skins.core().skins_switch(&first)),
        SkinsError::GameRunning
    ));
    assert_eq!(saves(&skins.folder), tagged("second"));
}

#[test]
fn a_game_started_during_the_copies_stops_the_switch() {
    let skins = skins();
    let (first, second) = skins.two_sets();
    skins
        .fixture
        .plan_scans(&[PlannedScan::Closed, PlannedScan::Running]);

    assert!(matches!(
        skin_error(skins.core().skins_switch(&first)),
        SkinsError::GameRunning
    ));

    let view = skins.view();
    assert_eq!(view.active.as_deref(), Some(second.as_str()));
    assert_eq!(view.interrupted, None);
    assert_eq!(saves(&skins.folder), tagged("second"));
}

#[test]
fn switching_to_the_set_in_the_game_does_nothing() {
    let skins = skins();
    let id = skins.save_new("Only");
    write_saves(&skins.folder, "edited");

    skins.core().skins_switch(&id).expect("switch");

    assert_eq!(saves(&skins.folder), tagged("edited"));
    assert_eq!(skins.stored(&id), tagged("first"));
    assert!(skins.backups().is_empty());
}

#[test]
fn each_switch_keeps_a_copy_and_only_the_newest_stay() {
    let skins = skins();
    let (first, second) = skins.two_sets();
    for round in 0..=SKIN_BACKUPS_KEPT {
        let target = if round % 2 == 0 { &first } else { &second };
        write_saves(&skins.folder, &format!("round{round}"));
        skins.core().skins_switch(target).expect("switch");
    }

    let backups = skins.backups();
    assert_eq!(backups.len(), SKIN_BACKUPS_KEPT);
    assert_eq!(
        backups.last().expect("newest"),
        &tagged(&format!("round{SKIN_BACKUPS_KEPT}"))
    );
    assert_eq!(backups[0], tagged("round1"), "the oldest went");
}

#[test]
fn the_copy_a_set_loses_to_the_game_is_kept() {
    let skins = skins();
    let (first, second) = skins.two_sets();
    skins.core().skins_switch(&first).expect("to first");
    assert!(skins.replaced(&second).is_empty(), "nothing changed yet");

    write_saves(&skins.folder, "fresh install");
    skins.core().skins_switch(&second).expect("to second");

    assert_eq!(skins.stored(&first), tagged("fresh install"));
    assert_eq!(skins.replaced(&first), vec![tagged("first")]);
}

#[test]
fn a_busy_set_leaves_the_copies_of_another_alone() {
    let skins = skins();
    let (first, second) = skins.two_sets();
    skins.core().skins_switch(&first).expect("to first");
    write_saves(&skins.folder, "fresh install");
    skins.core().skins_switch(&second).expect("to second");
    let third = skins.save_new("Third");

    for round in 0..=2 * SKIN_BACKUPS_KEPT {
        let target = if round % 2 == 0 { &second } else { &third };
        write_saves(&skins.folder, &format!("round{round}"));
        skins.core().skins_switch(target).expect("switch");
    }

    assert_eq!(skins.replaced(&first), vec![tagged("first")]);
    assert_eq!(skins.replaced(&second).len(), SKIN_BACKUPS_KEPT);
}

#[test]
fn deleting_a_set_takes_its_copies_along() {
    let skins = skins();
    let (first, second) = skins.two_sets();
    skins.core().skins_switch(&first).expect("to first");
    write_saves(&skins.folder, "edited");
    skins.core().skins_switch(&second).expect("to second");
    assert_eq!(skins.replaced(&first).len(), 1);

    skins.core().skins_delete(&first).expect("delete");

    assert!(!skins.store().join("replaced").join(&first).exists());
}

#[test]
fn putting_back_brings_the_stored_copy_and_keeps_what_the_game_had() {
    let skins = skins();
    let first = skins.save_new("First");
    write_saves(&skins.folder, "fresh install");

    let view = skins.core().skins_put_back(&first).expect("put back");

    assert_eq!(view.active.as_deref(), Some(first.as_str()));
    assert_eq!(view.interrupted, None);
    assert_eq!(saves(&skins.folder), tagged("first"));
    assert_eq!(skins.stored(&first), tagged("first"));
    assert_eq!(skins.backups(), vec![tagged("fresh install")]);
    skins.other_saves_untouched();
}

#[test]
fn with_nothing_to_put_back_nothing_happens() {
    let skins = skins();
    let first = skins.save_new("First");

    skins.core().skins_put_back(&first).expect("put back");

    assert!(skins.backups().is_empty());
}

#[test]
fn an_automatic_copy_becomes_a_set_and_the_game_stays_as_it_is() {
    let skins = skins();
    let (first, second) = skins.two_sets();
    skins.core().skins_switch(&first).expect("to first");
    write_saves(&skins.folder, "fresh install");
    skins.core().skins_switch(&second).expect("to second");

    let copies = skins.core().skins_copies();
    let origins: Vec<_> = copies.iter().map(|copy| copy.set_name.as_deref()).collect();
    assert_eq!(origins, [Some("First"), None, None], "newest first");
    assert!(copies[0].taken.ends_with('Z'), "{}", copies[0].taken);

    let view = skins
        .core()
        .skins_keep_copy(&copies[0].id, "First again")
        .expect("keep");

    let kept = view.sets.iter().find(|set| set.set.name == "First again");
    let kept = kept.expect("the new set");
    assert_eq!(skins.stored(&kept.set.id), tagged("first"));
    assert_eq!(view.active.as_deref(), Some(second.as_str()));
    assert_eq!(saves(&skins.folder), tagged("second"));
}

#[test]
fn only_a_listed_copy_can_be_kept() {
    let skins = skins();
    skins.save_new("First");

    assert!(matches!(
        skin_error(skins.core().skins_keep_copy("game-../..", "Name")),
        SkinsError::UnknownSet
    ));
}

#[test]
fn a_copy_cut_short_is_neither_offered_nor_counted() {
    let skins = skins();
    let (first, second) = skins.two_sets();
    for round in 0..SKIN_BACKUPS_KEPT {
        let target = if round % 2 == 0 { &first } else { &second };
        skins.core().skins_switch(target).expect("switch");
    }
    let backups = skins.store().join("backups");
    let partial = backups.join("99991231-235959-000000000");
    fs::create_dir_all(&partial).expect("folder");
    fs::write(partial.join(SKIN_SAVE_FILES[0]), b"cut short").expect("file");

    assert_eq!(skins.core().skins_copies().len(), SKIN_BACKUPS_KEPT);

    let target = if SKIN_BACKUPS_KEPT.is_multiple_of(2) {
        &first
    } else {
        &second
    };
    skins.core().skins_switch(target).expect("switch");

    assert!(!partial.exists());
    assert_eq!(skins.backups().len(), SKIN_BACKUPS_KEPT);
}

#[test]
fn names_have_rules() {
    let skins = skins();
    let id = skins.save_new("Taken");
    skins.save_new("Other");

    for (name, wanted) in [("", "invalid"), ("   ", "invalid"), ("tAKEN", "taken")] {
        let error = skin_error(skins.core().skins_save_new(name));
        match (wanted, error) {
            ("invalid", SkinsError::InvalidName) | ("taken", SkinsError::NameTaken) => {}
            (_, other) => panic!("{name}: {other:?}"),
        }
    }
    assert!(matches!(
        skin_error(skins.core().skins_save_new(&"x".repeat(101))),
        SkinsError::InvalidName
    ));
    assert!(matches!(
        skin_error(skins.core().skins_edit(&id, "other", "")),
        SkinsError::NameTaken
    ));

    let view = skins
        .core()
        .skins_edit(&id, "  taken  ", "Reina 3")
        .expect("own name");
    let set = view.sets.iter().find(|set| set.set.id == id).expect("set");
    assert_eq!(set.set.name, "taken");
    assert_eq!(set.set.notes, "Reina 3");
    assert!(matches!(
        skin_error(skins.core().skins_edit(&id, "taken", &"n".repeat(4001))),
        SkinsError::NotesTooLong
    ));
}

#[test]
fn the_set_in_the_game_cannot_be_deleted_and_another_goes_whole() {
    let skins = skins();
    let (first, second) = skins.two_sets();

    assert!(matches!(
        skin_error(skins.core().skins_delete(&second)),
        SkinsError::SetInUse
    ));

    let view = skins.core().skins_delete(&first).expect("delete");
    assert_eq!(view.sets.len(), 1);
    assert!(!skins.store().join(&first).exists());
    assert!(matches!(
        skin_error(skins.core().skins_switch(&first)),
        SkinsError::UnknownSet
    ));
}

#[test]
fn a_stored_set_with_a_lost_file_leaves_the_game_alone() {
    let skins = skins();
    let (first, _) = skins.two_sets();
    fs::remove_file(skins.store().join(&first).join(SKIN_SAVE_FILES[0])).expect("lose a file");

    assert!(matches!(
        skin_error(skins.core().skins_switch(&first)),
        SkinsError::BrokenSet
    ));
    assert_eq!(saves(&skins.folder), tagged("second"));
    assert!(skins.backups().is_empty());
}

#[test]
fn a_failed_switch_puts_the_old_files_back() {
    let skins = skins();
    let (first, second) = skins.two_sets();
    skins.core().inject_skin_faults(vec![Some(2)]);

    assert!(matches!(
        skin_error(skins.core().skins_switch(&first)),
        SkinsError::SwitchFailed { restored: true }
    ));

    let view = skins.view();
    assert_eq!(view.active.as_deref(), Some(second.as_str()));
    assert_eq!(view.interrupted, None);
    assert_eq!(saves(&skins.folder), tagged("second"));
    assert!(
        !skins
            .folder_listing()
            .iter()
            .any(|name| name.contains("remmods"))
    );
}

#[test]
fn a_switch_that_could_not_be_undone_is_finished_later() {
    let skins = skins();
    let (first, second) = skins.two_sets();
    skins.core().inject_skin_faults(vec![Some(2), Some(1)]);

    assert!(matches!(
        skin_error(skins.core().skins_switch(&first)),
        SkinsError::SwitchFailed { restored: false }
    ));

    let view = skins.view();
    assert_eq!(view.interrupted.as_deref(), Some(first.as_str()));
    assert_eq!(view.active.as_deref(), Some(second.as_str()));
    assert!(matches!(
        skin_error(skins.core().skins_save_new("Mixed")),
        SkinsError::Interrupted
    ));
    assert!(matches!(
        skin_error(skins.core().skins_delete(&first)),
        SkinsError::SetInUse
    ));

    let view = skins.core().skins_switch(&first).expect("finish");

    assert_eq!(view.interrupted, None);
    assert_eq!(view.active.as_deref(), Some(first.as_str()));
    assert_eq!(saves(&skins.folder), tagged("first"));
    assert_eq!(
        skins.stored(&second),
        tagged("second"),
        "the mix replaced no copy"
    );
}

#[test]
fn a_switch_that_changed_nothing_leaves_no_mark() {
    let skins = skins();
    let (first, second) = skins.two_sets();
    skins.core().inject_skin_faults(vec![Some(0), Some(0)]);

    assert!(matches!(
        skin_error(skins.core().skins_switch(&first)),
        SkinsError::SwitchFailed { restored: true }
    ));

    let view = skins.view();
    assert_eq!(view.active.as_deref(), Some(second.as_str()));
    assert_eq!(view.interrupted, None);
    assert_eq!(saves(&skins.folder), tagged("second"));
}

#[test]
fn finishing_a_broken_switch_keeps_what_the_game_has_now() {
    let skins = skins();
    let (first, _) = skins.two_sets();
    skins.core().inject_skin_faults(vec![Some(2), Some(1)]);
    assert!(skins.core().skins_switch(&first).is_err());
    write_saves(&skins.folder, "saved since");

    skins.core().skins_switch(&first).expect("finish");

    assert_eq!(saves(&skins.folder), tagged("first"));
    assert_eq!(
        skins.backups().last().expect("newest"),
        &tagged("saved since")
    );
}

#[test]
fn a_restart_still_knows_about_an_unfinished_switch() {
    let skins = skins();
    let (first, _) = skins.two_sets();
    skins.core().inject_skin_faults(vec![Some(1), Some(0)]);
    assert!(skins.core().skins_switch(&first).is_err());

    assert_eq!(
        skins
            .fixture
            .second_core()
            .skins_view()
            .interrupted
            .as_deref(),
        Some(first.as_str())
    );
}

#[test]
fn only_a_real_save_folder_is_written() {
    let skins = skins();
    let paths = &skins.fixture.paths;
    let scope = Scope::from_roots(&[], std::slice::from_ref(&paths.data_dir)).expect("scope");
    let guard = skins.core().ops.write(scope).expect("guard");
    let files: [Vec<u8>; 5] = Default::default();
    let elsewhere = skins.root.join("Saved").join("NotSaves").join(ACCOUNT);
    fs::create_dir_all(&elsewhere).expect("folder");
    let not_account = save_root(&skins.root).join("notes");
    fs::create_dir_all(&not_account).expect("folder");
    let relative = Path::new("Saved").join("SaveGames").join(ACCOUNT);

    for folder in [
        elsewhere.as_path(),
        not_account.as_path(),
        relative.as_path(),
    ] {
        let refused = guard.replace_skin_saves(folder, &files, None);
        assert!(
            matches!(refused, Err(FsError::OutOfScope { .. })),
            "{folder:?}"
        );
    }
    assert_eq!(fs::read_dir(&elsewhere).expect("list").count(), 0);
    assert_eq!(fs::read_dir(&not_account).expect("list").count(), 0);
}

#[test]
fn the_account_that_played_last_is_the_one_in_use() {
    let skins = skins();
    let first_set = skins.save_new("Mine");
    let other = save_root(&skins.root).join("76561198000000002");
    write_saves(&other, "other");
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(60);
    for name in SKIN_SAVE_FILES {
        let file = fs::File::options()
            .write(true)
            .open(other.join(name))
            .expect("open");
        file.set_modified(later).expect("touch");
    }

    let view = skins.view();
    assert_eq!(view.account.as_deref(), Some("76561198000000002"));
    assert!(view.sets.is_empty(), "each account has its own sets");

    fs::remove_dir_all(&other).expect("remove");
    assert_eq!(skins.view().active.as_deref(), Some(first_set.as_str()));
}

#[test]
fn screenshots_belong_to_their_set() {
    let skins = skins();
    let id = skins.save_new("Fine");
    let source = skins.root.join("shot.png");
    fs::write(&source, PNG).expect("source");

    let view = skins
        .core()
        .skins_add_screenshot(&id, &source)
        .expect("add");
    let shot = view.sets[0].set.screenshots[0].clone();
    fs::remove_file(&source).expect("the source may go");

    let url = skins.core().assets_picture(&shot).expect("stored");
    assert!(url.starts_with("data:image/png;base64,"));

    let fake = skins.root.join("fake.png");
    fs::write(&fake, b"no picture").expect("fake");
    let refused = skins.core().skins_add_screenshot(&id, &fake);
    assert!(matches!(
        refused,
        Err(CoreError::Mods(ModsError::Picture {
            problem: PictureProblem::NotAnImage
        }))
    ));
}

fn refresh_assets(skins: &Skins) {
    skins.core().mods_refresh().expect("scan");
    skins.core().mods_base_refresh().expect("base");
    skins.core().assets_refresh().expect("assets");
}

#[test]
fn a_screenshot_lives_as_long_as_a_set_names_it() {
    let skins = skins_with(&[("pakchunk0-Windows.utoc", DATA)]);
    let id = skins.save_new("Fine");
    let source = skins.root.join("shot.png");
    fs::write(&source, PNG).expect("source");
    let view = skins
        .core()
        .skins_add_screenshot(&id, &source)
        .expect("add");
    let shot = view.sets[0].set.screenshots[0].clone();

    refresh_assets(&skins);
    assert!(skins.core().assets_picture(&shot).is_some(), "kept");

    let view = skins
        .core()
        .skins_remove_screenshot(&id, &shot)
        .expect("remove");
    assert!(view.sets[0].set.screenshots.is_empty());
    refresh_assets(&skins);
    assert_eq!(skins.core().assets_picture(&shot), None, "gone");
}

#[test]
fn steam_s_screenshot_folder_is_offered_when_it_exists() {
    let skins = skins();
    assert_eq!(skins.core().skins_screenshot_folder(), None);

    let account: u64 = ACCOUNT.parse().expect("id");
    let steam = skins
        .fixture
        .game_root
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent);
    let folder = steam
        .expect("library")
        .join("userdata")
        .join((account - STEAM_BASE).to_string())
        .join("760")
        .join("remote")
        .join("1778820")
        .join("screenshots");
    fs::create_dir_all(&folder).expect("folder");

    assert_eq!(skins.core().skins_screenshot_folder(), Some(folder));
}

#[test]
fn a_list_from_a_newer_remmods_is_shown_and_left_alone() {
    let skins = skins();
    let (first, _) = skins.two_sets();
    let list = skins.store().join("sets.json");
    let text = fs::read_to_string(&list).expect("read");
    let newer = text.replace("\"version\": 1", "\"version\": 99");
    assert_ne!(newer, text);
    fs::write(&list, &newer).expect("write");

    assert!(skins.view().read_only);
    assert!(matches!(
        skin_error(skins.core().skins_switch(&first)),
        SkinsError::ReadOnly
    ));
    assert!(matches!(
        skin_error(skins.core().skins_edit(&first, "New", "")),
        SkinsError::ReadOnly
    ));
    assert_eq!(fs::read_to_string(&list).expect("read"), newer);
    assert_eq!(saves(&skins.folder), tagged("second"));
}

#[test]
fn a_damaged_list_is_kept_beside_a_fresh_one() {
    let skins = skins();
    skins.save_new("Fine");
    fs::write(skins.store().join("sets.json"), b"{broken").expect("damage");
    let kept = || {
        fs::read_dir(skins.store())
            .expect("list")
            .flatten()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("sets.json.")
            })
            .count()
    };

    assert!(skins.view().sets.is_empty());
    assert_eq!(kept(), 0, "only looking writes nothing");

    skins.save_new("Fresh");

    assert_eq!(kept(), 1, "the damaged file is backed up");
    assert_eq!(skins.view().sets.len(), 1);
}

#[test]
fn a_set_in_the_game_has_to_be_on_the_list() {
    let skins = skins();
    let (first, second) = skins.two_sets();
    let list = skins.store().join("sets.json");
    let text = fs::read_to_string(&list).expect("list");
    let edited = text.replace(
        &format!("\"active\": \"{second}\""),
        "\"active\": \"../..\"",
    );
    assert_ne!(text, edited);
    fs::write(&list, edited).expect("edit");

    assert_eq!(skins.view().active, None);
    skins.core().skins_switch(&first).expect("switch");

    assert_eq!(skins.view().active.as_deref(), Some(first.as_str()));
    assert_eq!(saves(&skins.folder), tagged("first"));
}

fn linked_group(skins: &Skins, set: Option<&str>) -> String {
    named_linked_group(skins, "Group", set)
}

fn named_linked_group(skins: &Skins, name: &str, set: Option<&str>) -> String {
    let view = skins
        .core()
        .profiles_save(&ProfileSave::Picked {
            id: None,
            name: name.to_owned(),
            description: String::new(),
            mode: Additive,
            mods: vec![("Standard:A".to_owned(), Enabled)],
            skins: set.map(str::to_owned),
        })
        .expect("save group");
    let group = &by_name(&view, name);
    assert_eq!(group.skin_set(), set);
    group.id.clone()
}

const WITH_MOD: &[(&str, &[u8])] = &[("Mods/A/A.pak-x", DATA)];

#[test]
fn switching_a_group_on_brings_its_skins_along() {
    let skins = skins_with(WITH_MOD);
    let (first, second) = skins.two_sets();
    let group = linked_group(&skins, Some(&first));

    let outcome = apply(&skins.fixture, &group).expect("apply");

    assert_eq!(
        outcome.skins,
        Some(LinkedSkins::Switched {
            name: "First".to_owned()
        })
    );
    assert_eq!(skins.view().active.as_deref(), Some(first.as_str()));
    assert_eq!(saves(&skins.folder), tagged("first"));
    assert_eq!(
        snapshots(&skins.fixture)[0].skin_set(),
        Some(second.as_str())
    );
}

#[test]
fn undo_takes_the_skins_back_too() {
    let skins = skins_with(WITH_MOD);
    let (first, _) = skins.two_sets();
    let group = linked_group(&skins, Some(&first));
    let outcome = apply(&skins.fixture, &group).expect("apply");

    let undone = apply(&skins.fixture, &outcome.snapshot_id.expect("snapshot")).expect("undo");

    assert_eq!(
        undone.skins,
        Some(LinkedSkins::Switched {
            name: "Second".to_owned()
        })
    );
    assert_eq!(saves(&skins.folder), tagged("second"));
}

#[test]
fn switching_a_group_off_leaves_the_skins() {
    let skins = skins_with(WITH_MOD);
    let (first, _) = skins.two_sets();
    let group = linked_group(&skins, Some(&first));
    apply(&skins.fixture, &group).expect("on");
    skins
        .core()
        .skins_switch(&skins.view().sets[1].set.id)
        .expect("other set");

    let outcome = switch(&skins.fixture, &group, Off).expect("off");

    assert_eq!(outcome.skins, None);
    assert_eq!(saves(&skins.folder), tagged("second"));
}

#[test]
fn a_running_game_keeps_the_skins_and_still_switches_the_mods() {
    let skins = skins_with(WITH_MOD);
    let (first, _) = skins.two_sets();
    let group = linked_group(&skins, Some(&first));
    let diff = skins
        .core()
        .profiles_diff(std::slice::from_ref(&group), On)
        .expect("diff");
    skins.fixture.running.store(true, Ordering::SeqCst);

    let outcome = skins
        .core()
        .profiles_apply(&ApplyRequest {
            profile_ids: vec![group.clone()],
            fingerprint: diff.fingerprint,
            apply_while_running: true,
            direction: On,
        })
        .expect("apply");

    assert_eq!(outcome.changed, 1);
    assert_eq!(
        outcome.skins,
        Some(LinkedSkins::GameRunning {
            name: "First".to_owned()
        })
    );
    assert_eq!(saves(&skins.folder), tagged("second"));
}

#[test]
fn groups_with_different_sets_leave_the_skins_alone_and_still_switch_the_mods() {
    let skins = skins_with(WITH_MOD);
    let (first, second) = skins.two_sets();
    let one = named_linked_group(&skins, "One", Some(&first));
    let other = named_linked_group(&skins, "Other", Some(&second));

    let outcome = switch_together(&skins.fixture, &[&one, &other], On).expect("switch");

    assert_eq!(outcome.changed, 1);
    assert_eq!(outcome.skins, Some(LinkedSkins::Several));
    assert_eq!(saves(&skins.folder), tagged("second"));
}

#[test]
fn groups_that_agree_on_a_set_bring_it_along() {
    let skins = skins_with(WITH_MOD);
    let (first, _) = skins.two_sets();
    let one = named_linked_group(&skins, "One", Some(&first));
    let other = named_linked_group(&skins, "Other", Some(&first));
    let plain = named_linked_group(&skins, "Plain", None);

    let outcome = switch_together(&skins.fixture, &[&one, &other, &plain], On).expect("switch");

    assert_eq!(
        outcome.skins,
        Some(LinkedSkins::Switched {
            name: "First".to_owned()
        })
    );
    assert_eq!(saves(&skins.folder), tagged("first"));
}

#[test]
fn a_deleted_set_among_several_links_does_not_count() {
    let skins = skins_with(WITH_MOD);
    let (first, second) = skins.two_sets();
    let one = named_linked_group(&skins, "One", Some(&first));
    let other = named_linked_group(&skins, "Other", Some(&second));
    skins.core().skins_delete(&first).expect("delete");

    let outcome = switch_together(&skins.fixture, &[&one, &other], On).expect("switch");

    assert_eq!(outcome.skins, None, "the set still linked is in the game");
    assert_eq!(saves(&skins.folder), tagged("second"));
}

#[test]
fn a_link_to_a_deleted_set_means_nothing() {
    let skins = skins_with(WITH_MOD);
    let (first, _) = skins.two_sets();
    let group = linked_group(&skins, Some(&first));
    skins.core().skins_delete(&first).expect("delete");

    let outcome = apply(&skins.fixture, &group).expect("apply");

    assert_eq!(outcome.skins, None);
    assert_eq!(outcome.changed, 1);
}

#[test]
fn a_group_already_on_still_brings_its_skins() {
    let skins = skins_with(&[("Mods/A/A.pak", DATA)]);
    let (first, _) = skins.two_sets();
    let group = linked_group(&skins, Some(&first));

    let outcome = apply(&skins.fixture, &group).expect("apply");

    assert_eq!(outcome.changed, 0);
    assert!(matches!(outcome.skins, Some(LinkedSkins::Switched { .. })));
    assert_eq!(saves(&skins.folder), tagged("first"));
}
