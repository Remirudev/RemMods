//! Writing `modinfo.json` in a fixed, stable byte layout (spec §5.5, PLAN §7).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use super::error::ModsError;
use super::listing::Lister;
use super::manage_target::{ManageAction, Managed, ModTarget, plain_path, resolve_managed};
use super::metadata::{MODINFO_MAX_BYTES, decode_modinfo_text};
use super::service::Refreshed;
use crate::safe_fs::FsError;
use crate::{Core, CoreError};

pub const MODINFO_FIELD_MAX_CHARS: usize = 10_000;
const MODINFO_FILE: &str = "modinfo.json";
const FIELDS: [&str; 4] = ["Version", "Category", "Description", "Author"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModInfoEdit {
    pub version: String,
    pub category: String,
    pub description: String,
    pub author: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataRequest {
    pub target: ModTarget,
    pub info: ModInfoEdit,
}

impl Core {
    /// No running check: the game never reads `modinfo.json`.
    pub fn mods_set_metadata(&self, request: &MetadataRequest) -> Result<Refreshed, CoreError> {
        check_lengths(&request.info)?;
        let ((), refreshed) = self.run_managed(|guard, input, content| {
            let managed = resolve_managed(content, &request.target)?;
            managed.check(ManageAction::Metadata)?;
            let Managed::Row(row) = managed else {
                return Err(ModsError::Stale.into());
            };
            let folder = plain_path(&row.path, input)?;
            let path = modinfo_path(&row.path, &folder)?;
            if needs_backup(&path)? {
                guard.backup_mod_file(&path)?;
            }
            guard.write_mod_file(&path, modinfo_json(&request.info).as_bytes())?;
            Ok(())
        })?;
        Ok(refreshed)
    }
}

/// The exact bytes RemMods writes (PLAN §7). Pure.
pub fn modinfo_json(info: &ModInfoEdit) -> String {
    let values = [
        or_default(&info.version, "1.0"),
        or_default(&info.category, "Misc"),
        or_default(&info.description, ""),
        or_default(&info.author, "Unknown"),
    ];
    let mut out = String::from("{\n");
    for (index, (field, value)) in FIELDS.iter().zip(values).enumerate() {
        let comma = if index + 1 < FIELDS.len() { "," } else { "" };
        let _ = writeln!(
            out,
            "  \"{field}\": \"{}\"{comma}",
            escape_json_value(value)
        );
    }
    out.push('}');
    out
}

/// JSON string escaping for one value (without quotes): `\\`, backspace, tab, newline, form feed
/// and carriage return as short escapes; `"`, `&`, `'`, `+`, `<`, `>`, `` ` `` and everything else
/// outside printable ASCII as uppercase `\uXXXX` UTF-16 units. Pure.
pub fn escape_json_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{C}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            '"' | '&' | '\'' | '+' | '<' | '>' | '`' => push_units(&mut out, c),
            ' '..='~' => out.push(c),
            _ => push_units(&mut out, c),
        }
    }
    out
}

fn push_units(out: &mut String, c: char) {
    let mut units = [0_u16; 2];
    for unit in c.encode_utf16(&mut units) {
        let _ = write!(out, "\\u{unit:04X}");
    }
}

fn or_default<'a>(value: &'a str, default: &'a str) -> &'a str {
    if value.trim().is_empty() {
        default
    } else {
        value
    }
}

fn check_lengths(info: &ModInfoEdit) -> Result<(), ModsError> {
    let fields = [
        ("version", &info.version),
        ("category", &info.category),
        ("description", &info.description),
        ("author", &info.author),
    ];
    for (field, value) in fields {
        if value.chars().count() > MODINFO_FIELD_MAX_CHARS {
            return Err(ModsError::FieldTooLong { field });
        }
    }
    Ok(())
}

/// The exact `modinfo.json`, else the single ASCII-case-insensitive variant, else a new
/// `modinfo.json`; several variants → `ModinfoAmbiguous`.
fn modinfo_path(fs_folder: &Path, plain_folder: &Path) -> Result<PathBuf, ModsError> {
    let entries = Lister::default()
        .list(fs_folder)
        .map_err(|error| FsError::io("list", plain_folder, error))?;
    let matches: Vec<_> = entries
        .iter()
        .filter(|entry| entry.kind.is_file_like())
        .filter(|entry| {
            entry
                .name
                .as_encoded_bytes()
                .eq_ignore_ascii_case(MODINFO_FILE.as_bytes())
        })
        .collect();
    if matches.iter().any(|entry| entry.name == MODINFO_FILE) {
        return Ok(plain_folder.join(MODINFO_FILE));
    }
    match matches.as_slice() {
        [] => Ok(plain_folder.join(MODINFO_FILE)),
        [single] => Ok(plain_folder.join(&single.name)),
        _ => Err(ModsError::ModinfoAmbiguous),
    }
}

/// An existing file that is invalid, too large or has keys the rewrite would drop.
fn needs_backup(path: &Path) -> Result<bool, ModsError> {
    let fs_path =
        crate::platform::fs_path(path).map_err(|error| FsError::io("read", path, error))?;
    let bytes = match std::fs::read(fs_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(FsError::io("read", path, error).into()),
    };
    if bytes.len() > MODINFO_MAX_BYTES {
        return Ok(true);
    }
    let text = decode_modinfo_text(&bytes);
    let plain = match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(serde_json::Value::Object(map)) => map.iter().all(|(key, value)| {
            FIELDS.contains(&key.as_str()) && (value.is_string() || value.is_null())
        }),
        _ => false,
    };
    Ok(!plain)
}
