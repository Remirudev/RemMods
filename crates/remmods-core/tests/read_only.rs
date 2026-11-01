//! P1 writes nothing below a game or Steam folder: every read-only entry point leaves the tree
//! byte- and mtime-identical (spec §3, §8.1).

mod mods_support;
mod support;

use mods_support::{exercise_read_only, listing, smoke_tree};

#[test]
fn read_only_entry_points_leave_the_tree_unchanged() {
    let (_home_dir, home) = support::temp_root();
    let (_app_dir, app_root) = support::temp_root();
    let steam = home.join(".local").join("share").join("Steam");
    smoke_tree(&steam);
    let before = listing(&home);
    exercise_read_only(&home, &app_root);
    assert_eq!(listing(&home), before);
}
