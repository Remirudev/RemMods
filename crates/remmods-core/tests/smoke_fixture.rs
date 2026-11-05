//! The §9.2 smoke tree scanned in Rust, so `scripts/make-fixtures.py`, its marker JSON and the
//! scanner cannot drift apart silently.

mod mods_support;
mod support;

use mods_support::{row, scan, scan_based, smoke_tree};
use remmods_core::content::ContainerStatus;
use remmods_core::mods::{ModState, ToggleBlock, UnmanagedKind};

#[test]
fn smoke_tree_matches_the_marker() {
    let (_dir, root) = support::temp_root();
    let paks = smoke_tree(&root.join("fixture"));
    let content = scan_based(&paks).content;
    let expected = [
        ("Standard:Alpha", ModState::Enabled, 0, 1),
        ("Standard:Beta", ModState::Disabled, 0, 0),
        ("Standard:Gamma", ModState::Partial, 0, 0),
        ("Standard:Delta", ModState::Enabled, 1, 0),
        ("Standard:Epsilon", ModState::Enabled, 1, 0),
        ("Standard:Empty", ModState::NoPakFiles, 0, 0),
        ("Loose:Mods/Loose", ModState::Enabled, 0, 1),
        ("Legacy:Legacy", ModState::Enabled, 0, 0),
        ("Logic:Tool", ModState::Enabled, 0, 0),
        ("Logic:Off", ModState::Disabled, 0, 0),
    ];
    assert_eq!(content.rows.len(), expected.len());
    for (key, state, conflicts, content_conflicts) in expected {
        let found = row(&content, key);
        assert_eq!(found.state, state, "{key}");
        assert_eq!(found.conflicts_with.len(), conflicts, "{key}");
        assert_eq!(found.content_conflicts.len(), content_conflicts, "{key}");
        let block = (state == ModState::NoPakFiles).then_some(ToggleBlock::NoPakFiles);
        assert_eq!(found.toggle_block, block, "{key}");
    }
    let beta = &row(&content, "Standard:Beta").content;
    assert_eq!(beta.assets, 1);
    assert!(!beta.containers[0].live);
    let loose = &row(&content, "Loose:Mods/Loose").content;
    assert_eq!((loose.assets, loose.containers[0].files), (1, 2));
    let alpha_link = &row(&content, "Standard:Alpha").content_conflicts[0];
    assert_eq!(alpha_link.key, "Loose:Mods/Loose");
    assert_eq!(alpha_link.sample, ["Polaris/Content/Smoke/Shared.uasset"]);
    let without_base = scan(&paks).content;
    assert!(
        without_base
            .rows
            .iter()
            .all(|row| row.content_conflicts.is_empty())
    );
    let tool = &row(&content, "Logic:Tool").content;
    assert!(tool.containers.is_empty());
    let alpha = &row(&content, "Standard:Alpha");
    assert_eq!(alpha.content.containers[0].status, ContainerStatus::Read);
    let alpha = &alpha.info;
    assert_eq!(
        (
            alpha.version.as_str(),
            alpha.category.as_str(),
            alpha.author.as_str()
        ),
        ("2.0", "Character", "RemMods")
    );
    let archives: Vec<&str> = content
        .archives
        .iter()
        .map(|a| a.rel_path.as_str())
        .collect();
    assert_eq!(archives, ["install-smoke.zip", "Mods/Pack.zip"]);
    let unmanaged: Vec<(&str, UnmanagedKind)> = content
        .unmanaged
        .iter()
        .map(|e| (e.rel_path.as_str(), e.kind))
        .collect();
    assert_eq!(
        unmanaged,
        [
            (
                "Mods/_Half_tmp_0123456789abcdef0123456789abcdef",
                UnmanagedKind::TempFolder
            ),
            ("Stray.pak", UnmanagedKind::PaksFile),
        ]
    );
    let leftovers: Vec<&str> = content
        .leftovers
        .iter()
        .map(|l| l.rel_path.as_str())
        .collect();
    assert_eq!(
        leftovers,
        ["Mods/_Old_tmp_ABCDEF0123456789ABCDEF0123456789"]
    );
}
