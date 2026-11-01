//! Log lines of a published snapshot; `scripts/scan-expect.py` prints the same debug lines from a
//! Paks listing, and the P1 checklist diffs the two (spec §4.5 step 10).

use std::collections::BTreeMap;

use super::model::{ModRow, ModState, ScanSnapshot, UnmanagedKind};
use crate::safe_fs::names::display_escaped;

pub(super) fn log_snapshot(snapshot: &ScanSnapshot) {
    let content = &snapshot.content;
    let mut per_state: BTreeMap<&str, u32> = BTreeMap::new();
    for row in &content.rows {
        *per_state.entry(state_name(row.state)).or_default() += 1;
    }
    let in_conflict = content
        .rows
        .iter()
        .filter(|row| !row.conflicts_with.is_empty())
        .count();
    log::info!(
        "scan-snapshot\trevision={}\trows={} {per_state:?} archives={} unmanaged={} leftovers={} \
         conflicting={in_conflict} dirs={} ms={}",
        snapshot.revision,
        content.rows.len(),
        content.archives.len(),
        content.unmanaged.len(),
        content.leftovers.len(),
        snapshot.stats.dirs_read,
        snapshot.stats.duration.as_millis(),
    );
    if !log::log_enabled!(log::Level::Debug) {
        return;
    }
    for row in &content.rows {
        log::debug!("{}", row_line(row));
    }
    for archive in &content.archives {
        log::debug!(
            "scan-archive\t{}\t{}",
            display_escaped(&archive.rel_path),
            archive.size
        );
    }
    for entry in &content.unmanaged {
        log::debug!(
            "scan-unmanaged\t{}\t{}\t{}",
            unmanaged_name(entry.kind),
            display_escaped(&entry.rel_path),
            entry.live_files
        );
    }
    for leftover in &content.leftovers {
        log::debug!("scan-leftover\t{}", display_escaped(&leftover.rel_path));
    }
}

/// `scan-row<TAB>{key}<TAB>{state}<TAB>{counts}<TAB>{size or -}<TAB>{conflicts}`.
pub(crate) fn row_line(row: &ModRow) -> String {
    let counts = row.payload;
    let size = row
        .size
        .map_or_else(|| "-".to_owned(), |size| size.to_string());
    let mut conflicts: Vec<&str> = row
        .conflicts_with
        .iter()
        .map(|link| link.key.as_str())
        .collect();
    conflicts.sort_unstable_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    let conflicts = if conflicts.is_empty() {
        "-".to_owned()
    } else {
        conflicts
            .iter()
            .map(|key| display_escaped(key))
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        "scan-row\t{}\t{}\t{}/{}/{}/{}\t{size}\t{conflicts}",
        display_escaped(&row.key),
        state_name(row.state),
        counts.live_pak,
        counts.off_pak,
        counts.live_companion,
        counts.off_companion,
    )
}

fn state_name(state: ModState) -> &'static str {
    match state {
        ModState::Enabled => "enabled",
        ModState::Partial => "partial",
        ModState::Disabled => "disabled",
        ModState::NoPakFiles => "noPakFiles",
        ModState::Unknown => "unknown",
    }
}

fn unmanaged_name(kind: UnmanagedKind) -> &'static str {
    match kind {
        UnmanagedKind::PaksFile => "paksFile",
        UnmanagedKind::Folder => "folder",
        UnmanagedKind::LinkedFolder => "linkedFolder",
        UnmanagedKind::TempFolder => "tempFolder",
    }
}
