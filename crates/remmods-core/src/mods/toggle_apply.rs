//! Selecting and planning the rows of one toggle (spec §5.5 steps 4–5). Pure on the snapshot,
//! except for listing the rows' files.

use std::collections::BTreeSet;

use super::error::ModsError;
use super::issues::ModIssue;
use super::meta_store::ModMeta;
use super::model::{ModRow, ModState, ScanContent, ScanInput, ToggleBlock};
use super::modref::ModRef;
use super::service::Refreshed;
use super::toggle_files::list_toggle_files;
use super::toggle_plan::{RecordChange, plan_row};
use crate::safe_fs::RenameOp;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToggleTarget {
    pub mod_ref: ModRef,
    pub key: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToggleRequest {
    pub targets: Vec<ToggleTarget>,
    pub enable: bool,
    pub apply_while_running: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkippedRow {
    pub key: String,
    pub name: String,
    pub reason: ToggleBlock,
}

#[derive(Clone, Debug)]
pub struct ToggleOutcome {
    pub refreshed: Refreshed,
    pub skipped: Vec<SkippedRow>,
    /// Unmanaged entries with live payload files (never touched).
    pub unmanaged: u32,
}

/// The rows a toggle acts on, chosen on the fresh scan.
pub(crate) struct Selection<'a> {
    pub rows: Vec<&'a ModRow>,
    pub skipped: Vec<SkippedRow>,
    pub unmanaged: u32,
}

/// The ops of every selected row, in row order, and the records to write after the batch.
pub(crate) struct Plan {
    pub ops: Vec<RenameOp>,
    pub records: Vec<(String, RecordChange)>,
}

/// Each distinct target needs exactly one row with its ref and key; a blocked row stops the
/// whole toggle.
pub(crate) fn select_targets<'a>(
    content: &'a ScanContent,
    targets: &[ToggleTarget],
) -> Result<Selection<'a>, ModsError> {
    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();
    for target in targets {
        if !seen.insert((target.mod_ref.as_str(), target.key.as_str())) {
            continue;
        }
        let mut matching = content
            .rows
            .iter()
            .filter(|row| row.mod_ref == target.mod_ref && row.key == target.key);
        let (Some(row), None) = (matching.next(), matching.next()) else {
            return Err(ModsError::Stale);
        };
        if let Some(reason) = row.toggle_block {
            return Err(ModsError::Blocked {
                key: row.key.clone(),
                reason,
            });
        }
        rows.push(row);
    }
    Ok(Selection {
        rows,
        skipped: Vec::new(),
        unmanaged: 0,
    })
}

/// Enabled and Partial rows plus Logic rows disabled by name but still loaded (A10); blocked rows
/// with live paks are reported, Unmanaged entries only counted.
pub(crate) fn select_disable_all(content: &ScanContent) -> Selection<'_> {
    let mut rows = Vec::new();
    let mut skipped = Vec::new();
    for row in &content.rows {
        match row.toggle_block {
            Some(reason) if row.payload.live_pak > 0 => skipped.push(SkippedRow {
                key: row.key.clone(),
                name: row.name.clone(),
                reason,
            }),
            Some(_) => {}
            None if wants_disable(row) => rows.push(row),
            None => {}
        }
    }
    let unmanaged = content
        .unmanaged
        .iter()
        .filter(|entry| entry.live_files > 0)
        .count();
    Selection {
        rows,
        skipped,
        unmanaged: u32::try_from(unmanaged).unwrap_or(u32::MAX),
    }
}

fn wants_disable(row: &ModRow) -> bool {
    matches!(row.state, ModState::Enabled | ModState::Partial)
        || row.issues.contains(&ModIssue::LogicDisabledButLoaded)
}

pub(crate) fn plan_rows(
    rows: &[&ModRow],
    input: &ScanInput,
    meta: &ModMeta,
    enable: bool,
) -> Result<Plan, ModsError> {
    let mut plan = Plan {
        ops: Vec::new(),
        records: Vec::new(),
    };
    for row in rows {
        let files = list_toggle_files(row, input)?;
        let row_plan = plan_row(row, &files, enable, meta.record(&row.key))?;
        plan.ops.extend(row_plan.ops);
        let write = match &row_plan.record {
            RecordChange::Keep => false,
            RecordChange::Clear => meta.record(&row.key).is_some(),
            RecordChange::Set(_) => true,
        };
        if write {
            plan.records.push((row.key.clone(), row_plan.record));
        }
    }
    Ok(plan)
}
