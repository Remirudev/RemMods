use super::error::ArchiveError;
use super::model::{Entry, EntryKind, EntryProblem, ExtractPlan, Listing};

/// Install refuses the whole archive when any entry is unsafe (no skipping, Appendix A12).
/// Link, Special and anti entries are never unsafe by kind: they are skipped and listed.
pub fn entry_is_unsafe(entry: &Entry) -> bool {
    match entry.problem {
        Some(
            EntryProblem::Traversal
            | EntryProblem::Absolute
            | EntryProblem::Drive
            | EntryProblem::Unc
            | EntryProblem::DotsOnly
            | EntryProblem::Nul
            | EntryProblem::TooDeep
            | EntryProblem::TooLong,
        ) => true,
        Some(EntryProblem::Empty) => entry.kind == EntryKind::File,
        None => false,
    }
}

/// Every backend checks the whole plan before it creates the first file: strictly ascending
/// indices of plain, unencrypted file entries with their listed size.
pub(crate) fn check_plan(listing: &Listing, plan: &ExtractPlan) -> Result<(), ArchiveError> {
    let mut previous: Option<u32> = None;
    for file in &plan.files {
        if previous.is_some_and(|last| file.index <= last) {
            return Err(ArchiveError::corrupt("plan is not sorted by index"));
        }
        previous = Some(file.index);
        let entry = usize::try_from(file.index)
            .ok()
            .and_then(|index| listing.entries.get(index))
            .filter(|entry| entry.index == file.index)
            .ok_or_else(|| ArchiveError::corrupt("plan names a missing entry"))?;
        if entry.kind != EntryKind::File || entry.anti {
            return Err(ArchiveError::corrupt("plan names a non-file entry"));
        }
        if entry.encrypted {
            return Err(ArchiveError::Encrypted);
        }
        if file.size != entry.size || file.dest.is_empty() {
            return Err(ArchiveError::corrupt("plan does not match the listing"));
        }
    }
    Ok(())
}
