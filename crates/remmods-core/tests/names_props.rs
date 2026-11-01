//! Property tests for the name rules (spec §8.1, strategies from the names research).

use std::ffi::OsStr;

use proptest::prelude::*;
use remmods_core::platform::RawName;
use remmods_core::safe_fs::names::*;
use unicode_normalization::UnicodeNormalization;

const FORBIDDEN: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
const BIDI: [char; 12] = [
    '\u{061C}', '\u{200E}', '\u{200F}', '\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}',
    '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}',
];
const DEVICES: [&str; 12] = [
    "CON",
    "PRN",
    "AUX",
    "NUL",
    "CONIN$",
    "CONOUT$",
    "COM1",
    "COM9",
    "LPT1",
    "LPT\u{B9}",
    "COM\u{B2}",
    "LPT\u{B3}",
];

fn clean_name() -> impl Strategy<Value = String> {
    "[A-Za-z0-9_\\-\u{E0}-\u{FF}\u{3B1}-\u{3C9}\u{672C}]{1,40}"
        .prop_filter("valid names only", |name| validate_name(name).is_ok())
}

fn any_text() -> impl Strategy<Value = String> {
    "\\PC{0,24}"
}

fn random_case(text: &str, mask: &[bool]) -> String {
    text.chars()
        .zip(mask.iter().cycle())
        .map(|(c, upper)| {
            if *upper {
                c.to_ascii_uppercase()
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig { failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn fold_key_is_idempotent(text in any_text()) {
        let once = fold_key(&text);
        prop_assert_eq!(fold_key(&once), once);
    }

    #[test]
    fn fold_key_ignores_normalisation_form(text in any_text()) {
        let nfc: String = text.nfc().collect();
        let nfd: String = text.nfd().collect();
        prop_assert_eq!(fold_key(&nfc), fold_key(&nfd));
    }

    #[test]
    fn fold_key_ignores_lowercasing(text in any_text()) {
        prop_assert_eq!(fold_key(&text.to_lowercase()), fold_key(&text));
    }

    #[test]
    fn fold_key_ignores_uppercasing(text in any_text()) {
        let nfd: String = text.nfd().collect();
        prop_assume!(!text.contains('\u{131}') && !nfd.contains('\u{345}'));
        prop_assert_eq!(fold_key(&text.to_uppercase()), fold_key(&text));
    }

    #[test]
    fn inserted_bad_char_fails(name in clean_name(), at in 0usize..40, pick in 0usize..100) {
        let bad: Vec<char> = FORBIDDEN
            .iter()
            .copied()
            .chain(('\u{0}'..='\u{1F}').chain('\u{7F}'..='\u{9F}'))
            .chain(BIDI.iter().copied())
            .collect();
        let c = bad[pick % bad.len()];
        let mut chars: Vec<char> = name.chars().collect();
        let at = at.min(chars.len());
        chars.insert(at, c);
        let spoiled: String = chars.into_iter().collect();
        prop_assert!(validate_name(&spoiled).is_err());
    }

    #[test]
    fn trailing_dot_or_space_fails(name in clean_name(), tail in "[. ]{1,3}") {
        let spoiled = format!("{name}{tail}");
        prop_assert_eq!(validate_name(&spoiled), Err(NameIssue::TrailingDotOrSpace));
    }

    #[test]
    fn device_names_fail(
        pick in 0usize..12,
        mask in proptest::collection::vec(any::<bool>(), 8),
        spaces in 0usize..3,
        extension in proptest::option::of("[a-z]{1,4}"),
    ) {
        let device = random_case(DEVICES[pick], &mask);
        let mut name = format!("{device}{}", " ".repeat(spaces));
        if let Some(extension) = extension {
            name.push('.');
            name.push_str(&extension);
        } else if spaces > 0 {
            name.push_str(".x");
        }
        prop_assert!(is_reserved_device_name(&name), "{name:?}");
        prop_assert_eq!(validate_name(&name), Err(NameIssue::ReservedDeviceName));
    }

    #[test]
    fn too_long_iff_over_255_bytes(len in 200usize..320, wide in any::<bool>()) {
        let unit = if wide { "\u{E9}" } else { "a" };
        let name: String = unit.repeat(len);
        let too_long = name.len() > NAME_MAX_BYTES;
        prop_assert_eq!(name_issues(&name).contains(&NameIssue::TooLong), too_long);
    }

    #[test]
    fn byte_escapes_round_trip(raw in proptest::collection::vec(any::<u8>(), 0..24)) {
        let key = escape_bytes(&raw);
        prop_assert_eq!(unescape_key(&key, KeyForm::Bytes), Some(RawName::Bytes(raw)));
    }

    #[test]
    fn byte_escapes_are_injective(
        a in proptest::collection::vec(any::<u8>(), 0..12),
        b in proptest::collection::vec(any::<u8>(), 0..12),
    ) {
        prop_assert_eq!(escape_bytes(&a) == escape_bytes(&b), a == b);
    }

    #[test]
    fn wide_escapes_round_trip(raw in proptest::collection::vec(any::<u16>(), 0..24)) {
        let key = escape_wide(&raw);
        prop_assert_eq!(unescape_key(&key, KeyForm::Wide), Some(RawName::Wide(raw)));
    }

    #[test]
    fn wide_escapes_are_injective(
        a in proptest::collection::vec(any::<u16>(), 0..12),
        b in proptest::collection::vec(any::<u16>(), 0..12),
    ) {
        prop_assert_eq!(escape_wide(&a) == escape_wide(&b), a == b);
    }

    #[test]
    fn unicode_names_are_their_own_key(text in any_text()) {
        prop_assume!(!text.contains('\\'));
        prop_assert_eq!(key_text(OsStr::new(&text)), text);
    }

    #[test]
    fn temp_names_are_recognised(stem in "[A-Za-z0-9 _]{0,20}", hex in "[0-9a-fA-F]{32}") {
        let name = format!("_{stem}_tmp_{hex}");
        prop_assert!(is_install_temp_name(OsStr::new(&name)));
    }

    #[test]
    fn disabled_prefix_in_any_ascii_case(
        mask in proptest::collection::vec(any::<bool>(), 10),
        rest in "[A-Za-z0-9]{0,10}",
    ) {
        let name = format!("{}{rest}", random_case(DISABLED_PREFIX, &mask));
        prop_assert!(has_disabled_prefix_ascii(OsStr::new(&name)));
        prop_assert!(has_disabled_prefix_folded(&name));
    }

    #[test]
    fn order_key_equality_is_string_equality(a in any_text(), b in any_text()) {
        prop_assert_eq!(order_key(&a) == order_key(&b), a == b);
    }
}
