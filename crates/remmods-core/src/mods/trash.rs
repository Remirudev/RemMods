//! Moving a mod to the Trash (spec §5.3); `api_manage.rs` runs it.

use std::path::PathBuf;

use super::details::resolve_ref;
use super::error::ModsError;
use super::manage_target::{Managed, ModTarget, plain_path};
use super::model::{PayloadKind, ScanInput};
use super::payload::classify_payload;
use super::service::Refreshed;
use super::toggle_files::list_toggle_files;
use crate::platform::TrashKind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrashRequest {
    pub target: ModTarget,
    pub apply_while_running: bool,
}

#[derive(Clone, Debug)]
pub struct TrashOutcome {
    pub refreshed: Refreshed,
    pub key: String,
    pub items: u32,
    pub kind: TrashKind,
}

/// Folder row → its folder; leftover → its folder; archive → its file; loose row → its group,
/// `.pak` first (a crash mid-group then leaves only inert companions behind).
pub(crate) fn trash_items(
    managed: &Managed<'_>,
    input: &ScanInput,
) -> Result<Vec<PathBuf>, ModsError> {
    match managed {
        Managed::Row(row) if row.kind == super::model::ModKind::Loose => {
            let mut files = list_toggle_files(row, input)?.files;
            files.sort_by_cached_key(|file| {
                let name = file.path.file_name().unwrap_or_default();
                (group_rank(name), name.as_encoded_bytes().to_vec())
            });
            Ok(files.into_iter().map(|file| file.path).collect())
        }
        Managed::Row(row) => Ok(vec![plain_path(&row.path, input)?]),
        Managed::Archive(entry) => Ok(vec![plain_path(
            &resolve_ref(input, &entry.mod_ref)?,
            input,
        )?]),
        Managed::Leftover(entry) => Ok(vec![plain_path(
            &resolve_ref(input, &entry.mod_ref)?,
            input,
        )?]),
        Managed::Unmanaged(_) => Err(ModsError::Stale),
    }
}

fn group_rank(name: &std::ffi::OsStr) -> u8 {
    match classify_payload(name).map(|payload| payload.kind) {
        Some(PayloadKind::Pak) => 0,
        Some(PayloadKind::Utoc) => 1,
        Some(PayloadKind::Ucas) => 2,
        Some(PayloadKind::Sig) => 3,
        None => 4,
    }
}
