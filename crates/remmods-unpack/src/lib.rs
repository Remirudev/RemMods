//! Reads single assets out of IoStore containers: chunks (`.ucas` blocks, Oodle included), item
//! pictures and text tables. Everything comes from files the user already has; nothing is shipped.
//!
//! `remmods-pak` lists a container without decompressing anything; this crate is the part that
//! decompresses. Every length, count and offset is checked before use, and a mod's file is
//! untrusted input: bad data is an error, never a large allocation.
#![forbid(unsafe_code)]

#[doc(hidden)]
pub mod fixture;

mod chunk;
mod error;
mod item_list;
mod package;
mod picture;
mod text_table;
mod texture;
mod toc;

use std::fs::File;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

pub use error::UnpackError;
pub use item_list::parse_item_list;
pub use picture::{Picture, render_picture};
pub use text_table::parse_text_table;
use toc::ChunkTable;

const UTOC: &str = "utoc";
const UCAS: &str = "ucas";

/// `X.utoc` → `X.ucas`, `X.utoc-x` → `X.ucas-x`, keeping the case of the extension.
fn ucas_of(utoc: &Path) -> Option<PathBuf> {
    let extension = utoc.extension()?.to_str()?;
    let (head, tail) = extension.split_at_checked(UTOC.len())?;
    if !head.eq_ignore_ascii_case(UTOC) {
        return None;
    }
    let name = if head.chars().all(char::is_uppercase) {
        UCAS.to_ascii_uppercase()
    } else {
        UCAS.to_owned()
    };
    Some(utoc.with_extension(format!("{name}{tail}")))
}

/// The decoders are other people's code reading a mod's bytes: a panic costs one asset, not the app.
fn guarded<T>(read: impl FnOnce() -> Result<T, UnpackError>) -> Result<T, UnpackError> {
    catch_unwind(AssertUnwindSafe(read)).unwrap_or(Err(UnpackError::Decoder))
}

/// One container, opened for reading single chunks.
#[derive(Clone, Debug)]
pub struct Container {
    table: ChunkTable,
    ucas: PathBuf,
}

impl Container {
    /// `utoc` is a `.utoc` or `.utoc-x` in file-system form; its `.ucas` sits beside it.
    pub fn open(utoc: &Path) -> Result<Self, UnpackError> {
        let ucas = ucas_of(utoc).ok_or(UnpackError::Malformed("container name"))?;
        Ok(Self {
            table: ChunkTable::read(utoc)?,
            ucas,
        })
    }

    pub fn chunk(&self, index: u32) -> Result<Vec<u8>, UnpackError> {
        guarded(|| {
            let mut file = File::open(&self.ucas)?;
            self.table.read_chunk(&mut file, index)
        })
    }

    /// The texture asset at chunk `asset`, with its `.ubulk` chunk when it has one, as a PNG whose
    /// longest edge is at most `max_edge`.
    pub fn picture(
        &self,
        asset: u32,
        bulk: Option<u32>,
        max_edge: u32,
    ) -> Result<Picture, UnpackError> {
        let package = self.chunk(asset)?;
        let bulk = bulk.map(|index| self.chunk(index)).transpose()?;
        guarded(|| render_picture(&package, bulk.as_deref(), max_edge))
    }

    /// The `key → text` records of a text table asset.
    pub fn text_table(&self, asset: u32) -> Result<Vec<(String, String)>, UnpackError> {
        Ok(parse_text_table(&self.chunk(asset)?))
    }
}
