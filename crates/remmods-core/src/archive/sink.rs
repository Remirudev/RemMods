use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::SystemTime;

use super::error::ArchiveError;
use super::model::{PlannedFile, StagedWrite};
use super::progress::{CancelToken, ProgressCounter};

/// Bytes between two cancel checks and progress updates inside one entry.
const CHECK_EVERY: u64 = 1 << 20;
/// Read buffer of the zip, 7z and folder copy loops (spec: at most 8 MiB per read).
pub(crate) const READ_CHUNK: usize = 1 << 20;

/// The first failure raised inside a sink. A crate that receives the sink's `io::Error` wraps it
/// in its own error type, so backends report this slot first.
#[derive(Clone, Default)]
pub(crate) struct FailureSlot(Arc<Mutex<Option<ArchiveError>>>);

impl FailureSlot {
    pub(crate) fn set(&self, error: ArchiveError) {
        let mut slot = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if slot.is_none() {
            *slot = Some(error);
        }
    }

    pub(crate) fn take(&self) -> Option<ArchiveError> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).take()
    }
}

/// One staged file with its declared size: nothing beyond that size ever reaches the target.
pub(crate) struct CappedSink {
    inner: Box<dyn StagedWrite>,
    index: u32,
    declared: u64,
    written: u64,
    unreported: u64,
    cancel: CancelToken,
    progress: ProgressCounter,
    failure: FailureSlot,
}

impl CappedSink {
    pub(crate) fn new(
        inner: Box<dyn StagedWrite>,
        file: &PlannedFile,
        cancel: &CancelToken,
        progress: &ProgressCounter,
        failure: &FailureSlot,
    ) -> Self {
        Self {
            inner,
            index: file.index,
            declared: file.size,
            written: 0,
            unreported: 0,
            cancel: cancel.clone(),
            progress: progress.clone(),
            failure: failure.clone(),
        }
    }

    /// `written != declared` is a `SizeMismatch`; otherwise the target sets the mtime and syncs.
    pub(crate) fn finish(mut self, mtime: Option<SystemTime>) -> Result<u64, ArchiveError> {
        if self.written != self.declared {
            return Err(ArchiveError::SizeMismatch { index: self.index });
        }
        self.report();
        let written = self.written;
        self.inner.finish(mtime).map_err(ArchiveError::target)?;
        self.progress.add_file();
        Ok(written)
    }

    fn report(&mut self) {
        self.progress.add_bytes(self.unreported);
        self.unreported = 0;
    }

    /// Never `ErrorKind::Interrupted`: `write_all` and `io::copy` would retry it forever.
    fn fail(&self, error: ArchiveError, kind: io::ErrorKind) -> io::Error {
        self.failure.set(error);
        io::Error::new(kind, "extraction sink refused the write")
    }
}

impl Write for CappedSink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let remaining = self.declared - self.written;
        if remaining == 0 {
            let error = ArchiveError::SizeMismatch { index: self.index };
            return Err(self.fail(error, io::ErrorKind::InvalidData));
        }
        let allowed = usize::try_from(remaining).map_or(buf.len(), |left| left.min(buf.len()));
        let accepted = match self.inner.write(&buf[..allowed]) {
            Ok(accepted) => accepted,
            Err(error) => {
                let kind = error.kind();
                return Err(self.fail(ArchiveError::target(error), kind));
            }
        };
        self.written += accepted as u64;
        self.unreported += accepted as u64;
        if self.unreported >= CHECK_EVERY {
            self.report();
            if self.cancel.is_cancelled() {
                return Err(self.fail(ArchiveError::Cancelled, io::ErrorKind::Other));
            }
        }
        Ok(accepted)
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.inner.flush() {
            Ok(()) => Ok(()),
            Err(error) => {
                let kind = error.kind();
                Err(self.fail(ArchiveError::target(error), kind))
            }
        }
    }
}

/// Reads `reader` to EOF into `sink` (so the crate's CRC check runs at EOF).
pub(crate) fn copy_entry(
    reader: &mut dyn Read,
    sink: &mut CappedSink,
    buf: &mut [u8],
    failure: &FailureSlot,
) -> Result<(), ArchiveError> {
    loop {
        let read = match reader.read(buf) {
            Ok(0) => return Ok(()),
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(failure.take().unwrap_or_else(|| ArchiveError::read(error))),
        };
        if let Err(error) = sink.write_all(&buf[..read]) {
            return Err(failure
                .take()
                .unwrap_or_else(|| ArchiveError::target(error)));
        }
    }
}
