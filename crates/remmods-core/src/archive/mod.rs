//! Archive reading for installs: format detection, listing, name decoding and validation, and
//! capped extraction into an `ExtractTarget`. Never writes anything itself (spec P4 §4).

mod cp437;
mod error;
mod folder_backend;
mod memory;
mod model;
mod names;
mod open;
mod progress;
#[cfg(feature = "rar-builtin")]
mod rar_backend;
mod sevenz_backend;
mod sink;
mod validate;
mod zip_backend;

pub use error::{ArchiveError, UnsupportedReason};
pub use memory::{
    memory_limit, ppmd_memory, sevenz_coder_memory, sevenz_header_memory, zip_lzma_memory,
    zip_xz_memory,
};
pub use model::{
    ArchiveFormat, ArchiveReader, DecodedName, Entry, EntryKind, EntryProblem, ExtractPlan,
    ExtractReport, ExtractTarget, Limits, Listing, NameEncoding, PlannedFile, RawEntry,
    RawEntryName, SourceFingerprint, StagedWrite,
};
pub use names::{decode_name, normalize};
pub use open::{fingerprint, open, open_folder, volume_name_kind};
pub use progress::{CancelToken, ProgressCounter};
pub use validate::entry_is_unsafe;
