//! Character and slot hints from asset paths (SPEC-P2B §4.6). Only mappings with local evidence
//! are named; every other code stays raw.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Slot {
    Upper,
    Lower,
    Bottom,
    FullBody,
    Hair,
    Head,
    FullHead,
    Hat,
    Face,
    FacePaint,
    FaceHair,
    EyeMakeup,
    Eyes,
    Cheek,
    Lips,
    Glasses,
    Accessory,
    Shoes,
    Extra,
}

const SLOTS: [(&str, Slot); 19] = [
    ("bdu", Slot::Upper),
    ("bdl", Slot::Lower),
    ("btm", Slot::Bottom),
    ("bdf", Slot::FullBody),
    ("har", Slot::Hair),
    ("hed", Slot::Head),
    ("hef", Slot::FullHead),
    ("hat", Slot::Hat),
    ("fac", Slot::Face),
    ("fap", Slot::FacePaint),
    ("fah", Slot::FaceHair),
    ("eym", Slot::EyeMakeup),
    ("eye", Slot::Eyes),
    ("chk", Slot::Cheek),
    ("lip", Slot::Lips),
    ("gla", Slot::Glasses),
    ("acc", Slot::Accessory),
    ("sho", Slot::Shoes),
    ("exp", Slot::Extra),
];

const CHARACTERS: [(&str, &str); 2] = [("ker", "Kunimitsu"), ("mnt", "Alisa")];

const COMMON_CODE: &str = "cmn";

const ITEM_CATEGORIES: [&str; 5] = [
    "Item_Prefab",
    "Base_Item",
    "Base_Character_Item",
    "Customize_Item",
    "Base_Eye_Item",
];

const STEM_PREFIXES: [&str; 5] = ["IP", "BI", "BCI", "CI", "BEI"];

pub fn slot(code: &str) -> Option<Slot> {
    SLOTS.iter().find(|(c, _)| *c == code).map(|&(_, s)| s)
}

pub fn character_name(code: &str) -> Option<&'static str> {
    CHARACTERS.iter().find(|(c, _)| *c == code).map(|&(_, n)| n)
}

pub fn is_common(code: &str) -> bool {
    code == COMMON_CODE
}

/// Raw codes; `None` when no rule matched.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssetHint {
    pub character: Option<String>,
    pub slot: Option<String>,
}

impl AssetHint {
    fn offer(&mut self, character: Option<&str>, slot: Option<&str>) {
        if self.character.is_none() {
            self.character = character.map(str::to_owned);
        }
        if self.slot.is_none() {
            self.slot = slot.map(str::to_owned);
        }
    }
}

/// `^[a-z][a-z0-9]{2}$`, case-sensitive so mod folders such as `SPY` never match.
fn is_code(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 3
        && bytes[0].is_ascii_lowercase()
        && bytes[1..]
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

fn code<'a>(text: Option<&&'a str>) -> Option<&'a str> {
    text.copied().filter(|t| is_code(t))
}

fn starts_with_words(segments: &[&str], words: &[&str]) -> bool {
    segments.len() >= words.len()
        && segments
            .iter()
            .zip(words)
            .all(|(s, w)| s.eq_ignore_ascii_case(w))
}

fn stem(file_name: &str) -> &str {
    match file_name.rfind('.') {
        Some(dot) if dot > 0 => &file_name[..dot],
        _ => file_name,
    }
}

/// §4.6 R1–R5; the first match per field wins.
pub fn hint_for(display_path: &str) -> AssetHint {
    let segments: Vec<&str> = display_path.split('/').collect();
    let Some((file_name, dirs)) = segments.split_last() else {
        return AssetHint::default();
    };
    let mut hint = AssetHint::default();
    rule_category_folders(dirs, &mut hint);
    rule_rep_texture(dirs, &mut hint);
    rule_stem_prefixes(stem(file_name), &mut hint);
    rule_model_folders(dirs, &mut hint);
    rule_name_tokens(dirs, stem(file_name), &mut hint);
    hint
}

fn rule_category_folders(dirs: &[&str], hint: &mut AssetHint) {
    if !starts_with_words(dirs, &["Polaris", "Content", "Character", "Item"]) {
        return;
    }
    let Some(category) = dirs.get(4) else { return };
    if !ITEM_CATEGORIES
        .iter()
        .any(|c| category.eq_ignore_ascii_case(c))
    {
        return;
    }
    let Some(character) = code(dirs.get(5)) else {
        return;
    };
    hint.offer(Some(character), code(dirs.get(6)));
}

fn rule_rep_texture(dirs: &[&str], hint: &mut AssetHint) {
    let words = ["Polaris", "Content", "UI", "Rep_Texture", "CUS_CH_Item"];
    if starts_with_words(dirs, &words) {
        hint.offer(code(dirs.get(5)), None);
    }
}

fn rule_stem_prefixes(stem: &str, hint: &mut AssetHint) {
    let tokens: Vec<&str> = stem.split('_').collect();
    let (character, slot) = if tokens.len() >= 4 && STEM_PREFIXES.contains(&tokens[0]) {
        (tokens.get(1), tokens.get(2))
    } else if tokens.len() >= 8 && tokens[..5] == ["T", "UI", "CUS", "CH", "item"] {
        (tokens.get(5), tokens.get(6))
    } else {
        return;
    };
    if let (Some(c), Some(s)) = (code(character), code(slot)) {
        hint.offer(Some(c), Some(s));
    }
}

fn rule_model_folders(dirs: &[&str], hint: &mut AssetHint) {
    for start in 0..dirs.len() {
        let rest = &dirs[start..];
        if !starts_with_words(rest, &["Character", "Item", "model"]) {
            continue;
        }
        let (folder, common) = if starts_with_words(&rest[3..], &["unique"]) {
            (rest.get(5), false)
        } else if starts_with_words(&rest[3..], &["common"]) {
            (rest.get(6), true)
        } else {
            continue;
        };
        let Some(folder) = folder else { continue };
        let tokens: Vec<&str> = folder.split('_').collect();
        if tokens.len() < 3 {
            continue;
        }
        let character = if common {
            Some(tokens[0]).filter(|t| is_common(t))
        } else {
            code(tokens.first())
        };
        if let (Some(c), Some(s)) = (character, code(tokens.get(1))) {
            hint.offer(Some(c), Some(s));
            return;
        }
    }
}

fn rule_name_tokens(dirs: &[&str], stem: &str, hint: &mut AssetHint) {
    if hint.character.is_some() {
        return;
    }
    let tokens = dirs
        .iter()
        .chain(std::iter::once(&stem))
        .flat_map(|segment| segment.split(['_', '-']));
    for token in tokens {
        if let Some(&(code, _)) = CHARACTERS
            .iter()
            .find(|(_, name)| token.eq_ignore_ascii_case(name))
        {
            hint.offer(Some(code), None);
            return;
        }
    }
}
