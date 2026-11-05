//! Character and slot hints from asset paths (SPEC-P2B §4.6). Only mappings with local evidence
//! are named; every other code stays raw. Character names come from the game's own texts
//! (`TEXT_UI_*_CharaPanel_<code>_*` spells the name), slot `ex0` from its item names (weapons and
//! other extras).

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

const SLOTS: [(&str, Slot); 20] = [
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
    ("ex0", Slot::Extra),
];

const CHARACTERS: [(&str, &str); 42] = [
    ("aml", "Jun"),
    ("ant", "Jin"),
    ("bbn", "Raven"),
    ("bee", "Heihachi"),
    ("bsn", "Steve"),
    ("cat", "Azucena"),
    ("cbr", "Lidia"),
    ("ccn", "Jack-8"),
    ("cht", "Bryan"),
    ("cml", "Yoshimitsu"),
    ("crw", "Zafina"),
    ("ctr", "Claudio"),
    ("der", "Asuka"),
    ("dog", "Eddy"),
    ("ghp", "Leo"),
    ("grf", "Paul"),
    ("grl", "Kazuya"),
    ("hms", "Lili"),
    ("hrs", "Shaheen"),
    ("jly", "Leroy"),
    ("kal", "Nina"),
    ("ker", "Kunimitsu"),
    ("kgr", "Anna"),
    ("klw", "Feng"),
    ("kmd", "Dragunov"),
    ("knk", "Armor King"),
    ("lon", "Victor"),
    ("lzd", "Lars"),
    ("mnt", "Alisa"),
    ("okm", "Clive"),
    ("pgn", "King"),
    ("pig", "Law"),
    ("rat", "Xiaoyu"),
    ("rbt", "Kuma"),
    ("snk", "Hwoarang"),
    ("swl", "Devil Jin"),
    ("tgr", "Fahkumram"),
    ("ttr", "Panda"),
    ("usi", "Bob"),
    ("wkz", "Miary Zo"),
    ("wlf", "Lee"),
    ("zbr", "Reina"),
];

/// The names also searched for in folder and file names: most others are ordinary words there
/// (a `Bob` hair cut, `King`, `Law`).
const NAME_TOKEN_CHARACTERS: [(&str, &str); 2] = [("ker", "Kunimitsu"), ("mnt", "Alisa")];

const COMMON_CODE: &str = "cmn";
/// Where a shared menu picture sits instead: everyone, then women's and men's items.
const COMMON_PREFIXES: [&str; 3] = [COMMON_CODE, "cf0", "cm0"];
const PICTURE_STEM: &str = "t_ui_cus_ch_item_";
const PREFAB_STEM: &str = "ip_";
const PREFAB_FOLDER: &str = "item_prefab";
const ASSET_EXTENSION: &str = ".uasset";

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

/// One game item, all lowercase: the parts of `IP_<character>_<slot>_<name>`. For a menu picture
/// the `character` can be a shared prefix (`cmn`, `cf0`, `cm0`).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemRef {
    pub character: String,
    pub slot: String,
    pub name: String,
}

impl ItemRef {
    /// `<character>_<slot>_<name>`, already lowercase.
    pub fn parse(text: &str) -> Option<Self> {
        let (character, rest) = text.split_once('_')?;
        let (slot, name) = rest.split_once('_')?;
        (is_code(character) && is_code(slot) && !name.is_empty()).then(|| Self {
            character: character.to_owned(),
            slot: slot.to_owned(),
            name: name.to_owned(),
        })
    }

    /// `<character>_<slot>_<name>`.
    pub fn key(&self) -> String {
        format!("{}_{}_{}", self.character, self.slot, self.name)
    }

    /// The keys that can hold this item's picture or text, best first.
    pub fn lookup_keys(&self) -> impl Iterator<Item = String> {
        let tail = format!("{}_{}", self.slot, self.name);
        std::iter::once(self.key()).chain(
            COMMON_PREFIXES
                .iter()
                .map(move |prefix| format!("{prefix}_{tail}")),
        )
    }
}

/// A `.uasset`'s lowercase stem and its folders as written.
fn asset_stem(display_path: &str) -> Option<(String, &str)> {
    let (dirs, file) = display_path.rsplit_once('/')?;
    let lower = file.to_ascii_lowercase();
    Some((lower.strip_suffix(ASSET_EXTENSION)?.to_owned(), dirs))
}

/// The item whose prefab sits at this path (`…/Item_Prefab/…/IP_<character>_<slot>_<name>.uasset`).
pub fn prefab_item(display_path: &str) -> Option<ItemRef> {
    let (stem, dirs) = asset_stem(display_path)?;
    dirs.split('/')
        .any(|dir| dir.eq_ignore_ascii_case(PREFAB_FOLDER))
        .then_some(())?;
    ItemRef::parse(stem.strip_prefix(PREFAB_STEM)?)
}

/// The item shown by the menu picture at this path (`T_UI_CUS_CH_item_<prefix>_<slot>_<name>`).
pub fn picture_item(display_path: &str) -> Option<ItemRef> {
    let (stem, _) = asset_stem(display_path)?;
    ItemRef::parse(stem.strip_prefix(PICTURE_STEM)?)
}

/// `IP_<character>_<slot>_<name>` as the game's item list spells a prefab.
pub fn prefab_key(prefab: &str) -> Option<String> {
    let lower = prefab.to_ascii_lowercase();
    ItemRef::parse(lower.strip_prefix(PREFAB_STEM)?).map(|item| item.key())
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
        if let Some(&(code, _)) = NAME_TOKEN_CHARACTERS
            .iter()
            .find(|(_, name)| token.eq_ignore_ascii_case(name))
        {
            hint.offer(Some(code), None);
            return;
        }
    }
}
