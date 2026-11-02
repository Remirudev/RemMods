//! Moving a renamed mod's key in RemMods' own stores (spec §5.10). No other `profiles.json` is ever
//! touched.

use crate::Core;
use crate::mods::RekeyReport;
use crate::safe_fs::WriteGuard;

impl Core {
    /// Under the caller's write guard. Profiles: every profile with `old` moves that entry to
    /// `new` (an existing `new` entry is replaced, logged); `mod-meta.json`: `ModMeta::rekey`.
    /// Each store is updated once; errors are logged and reported in `failed`.
    pub(crate) fn rekey(&self, guard: &WriteGuard<'_>, old: &str, new: &str) -> RekeyReport {
        let mut report = RekeyReport::default();
        let has_old = self
            .profiles
            .load()
            .file
            .profiles
            .iter()
            .any(|profile| profile.mods.contains_key(old));
        if has_old {
            let moved = self.profiles.update(guard, |file| {
                let mut moved = 0_u32;
                for profile in &mut file.profiles {
                    if let Some(entry) = profile.mods.remove(old) {
                        if profile.mods.insert(new.to_owned(), entry).is_some() {
                            log::warn!("profiles.json: a renamed key replaced an existing entry");
                        }
                        moved += 1;
                    }
                }
                Ok(moved)
            });
            match moved {
                Ok(count) => report.profiles = count,
                Err(error) => {
                    log::error!("profiles not rekeyed after a rename: {error}");
                    report.failed = true;
                }
            }
        }
        let meta = self.mods.meta();
        if meta.load().meta.mods.contains_key(old) {
            let mut moved = false;
            match meta.update(guard, |file| moved = file.rekey(old, new)) {
                Ok(()) => report.meta = moved,
                Err(error) => {
                    log::error!("mod-meta.json not rekeyed after a rename: {error}");
                    report.failed = true;
                }
            }
        }
        report
    }
}
