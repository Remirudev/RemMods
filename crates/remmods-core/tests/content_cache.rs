//! The per-container cache: unchanged `.utoc` files are read once (SPEC-P2B §5.3 rules 1, 6).
// Fixture edits (rewrite, rename, remove, set_modified) happen in a TempDir.
#![allow(clippy::disallowed_methods, clippy::expect_used)]

mod mods_support;
mod support;

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use mods_support::{DATA, MOUNT, fixture, paks_tree, row, scan_with, toc};
use remmods_core::content::{ContainerStatus, ContentCache};

fn two_mods(paks: &Path) {
    let a = toc(MOUNT, &["Smoke/A.uasset"]);
    let b = toc(MOUNT, &["Smoke/B.uasset"]);
    paks_tree(
        paks,
        &[
            ("Mods/A/A.pak", DATA),
            ("Mods/A/A.utoc", &a),
            ("Mods/B/B.pak", DATA),
            ("Mods/B/B.utoc", &b),
        ],
    );
}

fn counts(paks: &Path, cache: &ContentCache) -> (u32, u32) {
    let stats = scan_with(paks, cache).stats;
    (stats.containers_read, stats.containers_cached)
}

#[test]
fn second_scan_reads_nothing() {
    let (_dir, paks) = fixture();
    two_mods(&paks);
    let cache = ContentCache::new();
    assert_eq!(counts(&paks, &cache), (2, 0));
    assert_eq!(counts(&paks, &cache), (0, 2));
    assert_eq!(cache.len(), 2);
}

#[test]
fn changed_size_rereads() {
    let (_dir, paks) = fixture();
    two_mods(&paks);
    let cache = ContentCache::new();
    counts(&paks, &cache);
    let bigger = toc(MOUNT, &["Smoke/A.uasset", "Smoke/A2.uasset"]);
    fs::write(paks.join("Mods").join("A").join("A.utoc"), bigger).expect("rewrite");
    let output = scan_with(&paks, &cache);
    assert_eq!(
        (output.stats.containers_read, output.stats.containers_cached),
        (1, 1)
    );
    assert_eq!(row(&output.content, "Standard:A").content.assets, 2);
}

#[test]
fn changed_mtime_rereads() {
    let (_dir, paks) = fixture();
    two_mods(&paks);
    let cache = ContentCache::new();
    counts(&paks, &cache);
    let path = paks.join("Mods").join("B").join("B.utoc");
    let modified = fs::metadata(&path)
        .expect("stat")
        .modified()
        .expect("mtime");
    let file = fs::File::options().write(true).open(&path).expect("open");
    file.set_modified(modified + Duration::from_secs(3600))
        .expect("set mtime");
    drop(file);
    assert_eq!(counts(&paks, &cache), (1, 1));
    assert_eq!(counts(&paks, &cache), (0, 2));
}

#[test]
fn toggle_rename_rereads_once() {
    let (_dir, paks) = fixture();
    two_mods(&paks);
    let cache = ContentCache::new();
    counts(&paks, &cache);
    let folder = paks.join("Mods").join("A");
    fs::rename(folder.join("A.pak"), folder.join("A.pak-x")).expect("rename pak");
    fs::rename(folder.join("A.utoc"), folder.join("A.utoc-x")).expect("rename utoc");
    let output = scan_with(&paks, &cache);
    assert_eq!(
        (output.stats.containers_read, output.stats.containers_cached),
        (1, 1)
    );
    assert!(!row(&output.content, "Standard:A").content.containers[0].live);
    assert_eq!(cache.len(), 2);
    assert_eq!(counts(&paks, &cache), (0, 2));
}

#[test]
fn removed_containers_are_evicted() {
    let (_dir, paks) = fixture();
    two_mods(&paks);
    let cache = ContentCache::new();
    counts(&paks, &cache);
    assert_eq!(cache.len(), 2);
    fs::remove_dir_all(paks.join("Mods").join("B")).expect("remove B");
    assert_eq!(counts(&paks, &cache), (0, 1));
    assert_eq!(cache.len(), 1);
}

#[test]
fn unsupported_results_are_cached() {
    let (_dir, paks) = fixture();
    paks_tree(&paks, &[("Mods/G/G.pak", DATA), ("Mods/G/G.utoc", DATA)]);
    let cache = ContentCache::new();
    assert_eq!(counts(&paks, &cache), (1, 0));
    let output = scan_with(&paks, &cache);
    assert_eq!(
        (output.stats.containers_read, output.stats.containers_cached),
        (0, 1)
    );
    let status = row(&output.content, "Standard:G").content.containers[0].status;
    assert_eq!(status, ContainerStatus::NotIoStore);
}

fn three_hundred_mods(paks: &Path) {
    let names: Vec<String> = (0..100).map(|i| format!("File{i:03}.uasset")).collect();
    for m in 0..300 {
        let files: Vec<String> = names.iter().map(|n| format!("Mod{m:03}/{n}")).collect();
        let refs: Vec<&str> = files.iter().map(String::as_str).collect();
        let bytes = toc(MOUNT, &refs);
        let folder = format!("Mods/Mod{m:03}");
        let pak = format!("{folder}/Mod{m:03}.pak");
        let utoc = format!("{folder}/Mod{m:03}.utoc");
        paks_tree(paks, &[(&pak, DATA), (&utoc, &bytes)]);
    }
}

#[test]
fn scan_300_mods_hits_cache() {
    let (_dir, paks) = fixture();
    three_hundred_mods(&paks);
    let cache = ContentCache::new();
    assert_eq!(counts(&paks, &cache), (300, 0));
    assert_eq!(counts(&paks, &cache), (0, 300));
}

#[test]
#[ignore = "timing; run in release"]
fn scan_300_mods_timing() {
    let (_dir, paks) = fixture();
    three_hundred_mods(&paks);
    let cache = ContentCache::new();
    let started = Instant::now();
    let first = scan_with(&paks, &cache).stats;
    let first_total = started.elapsed();
    let second = scan_with(&paks, &cache).stats;
    println!(
        "first: scan {} ms, content {} ms; second: scan {} ms, content {} ms",
        first_total.as_millis(),
        first.content_duration.as_millis(),
        second.duration.as_millis(),
        second.content_duration.as_millis(),
    );
    assert!(first.content_duration <= first.duration / 4 + Duration::from_millis(250));
    assert!(second.content_duration <= Duration::from_millis(20));
    assert_eq!(second.containers_read, 0);
}
