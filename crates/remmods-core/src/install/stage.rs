//! Extraction into the staging folders (spec §5.4 step 5): the plan, the `ExtractTarget` over
//! `StagingDir`s and the extraction thread that forwards progress to the caller's thread.

use std::collections::HashSet;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::SystemTime;

use super::model::{InstallPhase, InstallProgress};
use super::service::StoredCandidate;
use crate::archive::{
    ArchiveError, ArchiveReader, CancelToken, ExtractPlan, ExtractReport, ExtractTarget,
    PlannedFile, ProgressCounter, StagedWrite,
};
use crate::safe_fs::{FsError, StagedFile, StagingDir};

const OFF_SUFFIX: &str = "-x";

/// The files of the picked candidates as one plan (slot = pick position), without the clash
/// members the user did not keep. Live payload files stage with `-x`; returns their indices.
pub(crate) fn build_plan(
    picks: &[&StoredCandidate],
    dropped: &HashSet<u32>,
) -> (ExtractPlan, HashSet<u32>) {
    let mut files = Vec::new();
    let mut inert = HashSet::new();
    for (slot, candidate) in picks.iter().enumerate() {
        for file in candidate
            .files
            .iter()
            .filter(|file| !dropped.contains(&file.index))
        {
            let mut dest = file.dest.clone();
            if file.live_payload {
                if let Some(name) = dest.last_mut() {
                    name.push_str(OFF_SUFFIX);
                }
                inert.insert(file.index);
            }
            files.push(PlannedFile {
                index: file.index,
                slot: u32::try_from(slot).unwrap_or(u32::MAX),
                dest,
                size: file.size,
                mtime: file.mtime,
            });
        }
    }
    files.sort_by_key(|file| file.index);
    (ExtractPlan { files }, inert)
}

/// Writes planned files into the staging folders and records which ones it made inert.
pub(crate) struct InstallTarget<'a> {
    staging: &'a [StagingDir],
    inert_indices: HashSet<u32>,
    /// Per slot: files staged with `-x` (paths inside the staging folder).
    pub(crate) inert: Vec<Vec<PathBuf>>,
    panic_in_create: bool,
}

impl<'a> InstallTarget<'a> {
    pub(crate) fn new(
        staging: &'a [StagingDir],
        inert_indices: HashSet<u32>,
        panic_in_create: bool,
    ) -> Self {
        Self {
            staging,
            inert_indices,
            inert: vec![Vec::new(); staging.len()],
            panic_in_create,
        }
    }
}

impl ExtractTarget for InstallTarget<'_> {
    fn create(&mut self, file: &PlannedFile) -> io::Result<Box<dyn StagedWrite>> {
        if self.panic_in_create {
            self.panic_in_create = false;
            panic!("install hook: panic in the install target");
        }
        let slot = usize::try_from(file.slot).unwrap_or(usize::MAX);
        let staging = self
            .staging
            .get(slot)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "unknown staging slot"))?;
        let staged = staging.create_file(&file.dest).map_err(into_io)?;
        if self.inert_indices.contains(&file.index)
            && let Some(list) = self.inert.get_mut(slot)
        {
            list.push(staged.path().to_path_buf());
        }
        Ok(Box::new(StagedWriter(staged)))
    }
}

struct StagedWriter(StagedFile);

impl Write for StagedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl StagedWrite for StagedWriter {
    fn finish(self: Box<Self>, mtime: Option<SystemTime>) -> io::Result<()> {
        self.0.finish(mtime).map_err(into_io)
    }
}

/// The OS error itself when there is one, so the archive layer classifies it (e.g. NoSpace).
fn into_io(error: FsError) -> io::Error {
    match error {
        FsError::Io { source, .. } => source,
        other => io::Error::other(other),
    }
}

/// Runs `reader.extract` on a scoped thread and forwards its progress ticks to `progress` on the
/// calling thread. A panic that escapes the backend becomes `Corrupt`.
pub(crate) fn run_extract(
    reader: Box<dyn ArchiveReader>,
    plan: &ExtractPlan,
    target: &mut InstallTarget<'_>,
    cancel: &CancelToken,
    progress: &(dyn Fn(&InstallProgress) + Send + Sync),
) -> Result<ExtractReport, ArchiveError> {
    let total_bytes = plan.files.iter().map(|file| file.size).sum();
    let total_files = u32::try_from(plan.files.len()).unwrap_or(u32::MAX);
    let (sender, receiver) = mpsc::channel::<(u64, u32)>();
    let counter = ProgressCounter::new(Arc::new(move |bytes, files| {
        // The receiver lives until the extraction thread ended; a send cannot fail before.
        let _ = sender.send((bytes, files));
    }));
    thread::scope(|scope| {
        let worker = scope.spawn(move || {
            let mut reader = reader;
            let result = reader.extract(plan, target, &counter, cancel);
            counter.flush();
            result
        });
        for (done_bytes, done_files) in receiver {
            progress(&InstallProgress {
                phase: InstallPhase::Extracting,
                done_bytes,
                total_bytes,
                done_files,
                total_files,
            });
        }
        worker.join().unwrap_or_else(|_| {
            Err(ArchiveError::Corrupt {
                detail: "panic".to_owned(),
            })
        })
    })
}
