//! modinfo.json, read like the original: .NET `File.ReadAllText` decoding, then System.Text.Json
//! semantics (spec §4.6).

use std::fmt;

use serde::de::{self, DeserializeSeed, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};

use super::model::{ModInfo, ModInfoDefaulted, ModInfoOrigin};

/// Larger modinfo.json files are treated as invalid (a RemMods guard).
pub const MODINFO_MAX_BYTES: usize = 1 << 20;

/// System.Text.Json's default MaxDepth, the top-level object included.
const MAX_DEPTH: usize = 64;

const DEFAULT_VERSION: &str = "1.0";
const DEFAULT_CATEGORY: &str = "Misc";
const DEFAULT_DESCRIPTION: &str = "";
const DEFAULT_AUTHOR: &str = "Unknown";

impl ModInfo {
    /// Every field at its default and flagged as such.
    pub fn defaults(origin: ModInfoOrigin) -> Self {
        Self {
            version: DEFAULT_VERSION.to_owned(),
            category: DEFAULT_CATEGORY.to_owned(),
            description: DEFAULT_DESCRIPTION.to_owned(),
            author: DEFAULT_AUTHOR.to_owned(),
            origin,
            defaulted: ModInfoDefaulted {
                version: true,
                category: true,
                description: true,
                author: true,
            },
        }
    }
}

/// Reads a modinfo.json body like the original (.NET `File.ReadAllText` + System.Text.Json).
pub fn parse_modinfo(bytes: &[u8]) -> ModInfo {
    if bytes.len() > MODINFO_MAX_BYTES {
        return ModInfo::defaults(ModInfoOrigin::Invalid);
    }
    let text = decode_modinfo_text(bytes);
    match serde_json::from_str::<ModinfoDoc>(&text) {
        Ok(doc) => doc.into_info(),
        Err(error) => {
            log::debug!("modinfo.json invalid: {error}");
            ModInfo::defaults(ModInfoOrigin::Invalid)
        }
    }
}

/// .NET `File.ReadAllText` decoding: BOM sniffing, invalid sequences become U+FFFD.
pub fn decode_modinfo_text(bytes: &[u8]) -> String {
    match bytes {
        [0xFE, 0xFF, rest @ ..] => decode_utf16(rest, u16::from_be_bytes),
        [0xFF, 0xFE, 0x00, 0x00, rest @ ..] => decode_utf32(rest, u32::from_le_bytes),
        [0xFF, 0xFE, rest @ ..] => decode_utf16(rest, u16::from_le_bytes),
        [0xEF, 0xBB, 0xBF, rest @ ..] => String::from_utf8_lossy(rest).into_owned(),
        [0x00, 0x00, 0xFE, 0xFF, rest @ ..] => decode_utf32(rest, u32::from_be_bytes),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

fn decode_utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> String {
    let (pairs, tail) = bytes.as_chunks::<2>();
    let odd_tail = !tail.is_empty();
    let units = pairs.iter().map(|pair| unit(*pair));
    let mut text: String = char::decode_utf16(units)
        .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect();
    if odd_tail {
        text.push(char::REPLACEMENT_CHARACTER);
    }
    text
}

fn decode_utf32(bytes: &[u8], unit: fn([u8; 4]) -> u32) -> String {
    let (quads, tail) = bytes.as_chunks::<4>();
    let partial_tail = !tail.is_empty();
    let mut text: String = quads
        .iter()
        .map(|quad| char::from_u32(unit(*quad)).unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect();
    if partial_tail {
        text.push(char::REPLACEMENT_CHARACTER);
    }
    text
}

/// The four fields as last seen in the document; `None` = missing or `null`.
#[derive(Default)]
struct ModinfoDoc {
    version: Option<String>,
    category: Option<String>,
    description: Option<String>,
    author: Option<String>,
}

impl ModinfoDoc {
    fn into_info(self) -> ModInfo {
        let (version, version_default) = field(self.version, DEFAULT_VERSION);
        let (category, category_default) = field(self.category, DEFAULT_CATEGORY);
        let (description, description_default) = field(self.description, DEFAULT_DESCRIPTION);
        let (author, author_default) = field(self.author, DEFAULT_AUTHOR);
        ModInfo {
            version,
            category,
            description,
            author,
            origin: ModInfoOrigin::File,
            defaulted: ModInfoDefaulted {
                version: version_default,
                category: category_default,
                description: description_default,
                author: author_default,
            },
        }
    }
}

/// A value is used unless it is missing, `null`, empty or only whitespace (not trimmed).
fn field(value: Option<String>, default: &str) -> (String, bool) {
    match value {
        Some(text) if !text.chars().all(char::is_whitespace) => (text, false),
        _ => (default.to_owned(), true),
    }
}

impl<'de> de::Deserialize<'de> for ModinfoDoc {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(DocVisitor)
    }
}

struct DocVisitor;

impl<'de> Visitor<'de> for DocVisitor {
    type Value = ModinfoDoc;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON object or null")
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(ModinfoDoc::default())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut doc = ModinfoDoc::default();
        while let Some(key) = map.next_key::<String>()? {
            let slot = match key.as_str() {
                "Version" => &mut doc.version,
                "Category" => &mut doc.category,
                "Description" => &mut doc.description,
                "Author" => &mut doc.author,
                _ => {
                    map.next_value_seed(SkipValue { depth: 1 })?;
                    continue;
                }
            };
            *slot = map.next_value::<Option<String>>()?;
        }
        Ok(doc)
    }
}

/// Skips any value while enforcing the nesting limit; `depth` = containers already open.
#[derive(Clone, Copy)]
struct SkipValue {
    depth: usize,
}

impl SkipValue {
    fn enter<E: de::Error>(self) -> Result<Self, E> {
        let depth = self.depth + 1;
        if depth > MAX_DEPTH {
            return Err(E::custom("modinfo.json nests deeper than 64 levels"));
        }
        Ok(Self { depth })
    }
}

impl<'de> DeserializeSeed<'de> for SkipValue {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for SkipValue {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("any JSON value")
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<(), E> {
        Ok(())
    }

    fn visit_i64<E: de::Error>(self, _: i64) -> Result<(), E> {
        Ok(())
    }

    fn visit_u64<E: de::Error>(self, _: u64) -> Result<(), E> {
        Ok(())
    }

    fn visit_f64<E: de::Error>(self, _: f64) -> Result<(), E> {
        Ok(())
    }

    fn visit_str<E: de::Error>(self, _: &str) -> Result<(), E> {
        Ok(())
    }

    fn visit_unit<E: de::Error>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        let inner = self.enter()?;
        while seq.next_element_seed(inner)?.is_some() {}
        Ok(())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let inner = self.enter()?;
        while map.next_key::<IgnoredAny>()?.is_some() {
            map.next_value_seed(inner)?;
        }
        Ok(())
    }
}
