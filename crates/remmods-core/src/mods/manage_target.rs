//! What rename, trash, delete and metadata may act on (spec §5.1).

use std::path::{Path, PathBuf};

use super::error::ModsError;
use super::issues::ModIssue;
use super::model::{
    ArchiveEntry, LeftoverEntry, ModKind, ModRow, ScanContent, ScanInput, ToggleBlock,
    UnmanagedEntry,
};
use super::toggle_apply::ToggleTarget;
use crate::platform;
use crate::safe_fs::FsError;

pub type ModTarget = ToggleTarget;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManageAction {
    Rename,
    Trash,
    Delete,
    Metadata,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManageBlock {
    Loose,
    Linked,
    Archive,
    Leftover,
    Unmanaged,
    OutsideRoots,
    JournalPending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagedKind {
    Folder,
    Loose,
    ArchiveInRoot,
    ArchiveInPaks,
    Leftover,
    Unmanaged,
}

pub(crate) enum Managed<'a> {
    Row(&'a ModRow),
    Archive(&'a ArchiveEntry),
    Leftover(&'a LeftoverEntry),
    Unmanaged(&'a UnmanagedEntry),
}

impl Managed<'_> {
    pub(crate) fn key(&self) -> &str {
        match self {
            Self::Row(row) => &row.key,
            Self::Archive(entry) => &entry.rel_path,
            Self::Leftover(entry) => &entry.rel_path,
            Self::Unmanaged(entry) => &entry.rel_path,
        }
    }

    pub(crate) fn kind(&self) -> ManagedKind {
        match self {
            Self::Row(row) if row.kind == ModKind::Loose => ManagedKind::Loose,
            Self::Row(_) => ManagedKind::Folder,
            Self::Archive(entry) if entry.rel_path.contains('/') => ManagedKind::ArchiveInRoot,
            Self::Archive(_) => ManagedKind::ArchiveInPaks,
            Self::Leftover(_) => ManagedKind::Leftover,
            Self::Unmanaged(_) => ManagedKind::Unmanaged,
        }
    }

    pub(crate) fn linked(&self) -> bool {
        match self {
            Self::Row(row) => {
                row.toggle_block == Some(ToggleBlock::Linked)
                    || row
                        .issues
                        .iter()
                        .any(|issue| matches!(issue, ModIssue::LinkedMod | ModIssue::LinkedRoot))
            }
            _ => false,
        }
    }

    /// The matrix of §5.1 for this entry.
    pub(crate) fn check(&self, action: ManageAction) -> Result<(), ModsError> {
        match manage_block(self.kind(), self.linked(), action) {
            None => Ok(()),
            Some(reason) => Err(ModsError::ManageBlocked {
                key: self.key().to_owned(),
                action,
                reason,
            }),
        }
    }
}

/// Exactly one entry whose `mod_ref` equals target.mod_ref and whose key (row.key, or rel_path
/// for archives/leftovers/unmanaged) equals target.key; else ModsError::Stale.
pub(crate) fn resolve_managed<'a>(
    content: &'a ScanContent,
    target: &ModTarget,
) -> Result<Managed<'a>, ModsError> {
    let matches =
        |mod_ref: &super::ModRef, key: &str| *mod_ref == target.mod_ref && key == target.key;
    let found: Vec<Managed<'a>> = content
        .rows
        .iter()
        .filter(|row| matches(&row.mod_ref, &row.key))
        .map(Managed::Row)
        .chain(
            content
                .archives
                .iter()
                .filter(|entry| matches(&entry.mod_ref, &entry.rel_path))
                .map(Managed::Archive),
        )
        .chain(
            content
                .leftovers
                .iter()
                .filter(|entry| matches(&entry.mod_ref, &entry.rel_path))
                .map(Managed::Leftover),
        )
        .chain(
            content
                .unmanaged
                .iter()
                .filter(|entry| matches(&entry.mod_ref, &entry.rel_path))
                .map(Managed::Unmanaged),
        )
        .collect();
    let mut found = found.into_iter();
    match (found.next(), found.next()) {
        (Some(managed), None) => Ok(managed),
        _ => Err(ModsError::Stale),
    }
}

pub fn manage_block(
    entry_kind: ManagedKind,
    linked: bool,
    action: ManageAction,
) -> Option<ManageBlock> {
    use ManageAction as A;
    use ManagedKind as K;
    if linked && entry_kind == K::Folder {
        return Some(ManageBlock::Linked);
    }
    match (entry_kind, action) {
        (K::Folder, _) | (K::Loose, A::Trash) | (K::ArchiveInRoot, A::Trash) => None,
        (K::Leftover, A::Trash | A::Delete) => None,
        (K::Loose, _) => Some(ManageBlock::Loose),
        (K::ArchiveInPaks, A::Trash) => Some(ManageBlock::OutsideRoots),
        (K::ArchiveInRoot | K::ArchiveInPaks, _) => Some(ManageBlock::Archive),
        (K::Leftover, _) => Some(ManageBlock::Leftover),
        (K::Unmanaged, _) => Some(ManageBlock::Unmanaged),
    }
}

/// A scan path (mod-side form) in the plain form built from `ScanInput.paks`, like the toggle's
/// file list; rename ops and scope checks use this form.
pub(crate) fn plain_path(path: &Path, input: &ScanInput) -> Result<PathBuf, ModsError> {
    let fs_paks =
        platform::fs_path(&input.paks).map_err(|error| FsError::io("list", &input.paks, error))?;
    let below = path.strip_prefix(&fs_paks).map_err(|_| ModsError::Stale)?;
    Ok(input.paks.join(below))
}
