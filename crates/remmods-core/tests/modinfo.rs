//! modinfo.json decoding and parsing like .NET File.ReadAllText + System.Text.Json (spec §4.6).

use remmods_core::mods::{
    MODINFO_MAX_BYTES, ModInfo, ModInfoDefaulted, ModInfoOrigin, decode_modinfo_text, parse_modinfo,
};

fn invalid() -> ModInfo {
    ModInfo::defaults(ModInfoOrigin::Invalid)
}

fn version_of(json: &str) -> String {
    parse_modinfo(json.as_bytes()).version
}

#[test]
fn no_file_gives_flagged_defaults() {
    let info = ModInfo::defaults(ModInfoOrigin::Missing);
    assert_eq!(info.version, "1.0");
    assert_eq!(info.category, "Misc");
    assert_eq!(info.description, "");
    assert_eq!(info.author, "Unknown");
    assert_eq!(
        info.defaulted,
        ModInfoDefaulted {
            version: true,
            category: true,
            description: true,
            author: true
        }
    );
}

#[test]
fn all_four_fields_are_used() {
    let info = parse_modinfo(
        br#"{"Version":"2.0","Category":"Skin","Description":"Nice","Author":"Rem"}"#,
    );
    assert_eq!(
        (
            info.version.as_str(),
            info.category.as_str(),
            info.description.as_str(),
            info.author.as_str()
        ),
        ("2.0", "Skin", "Nice", "Rem")
    );
    assert_eq!(info.origin, ModInfoOrigin::File);
    assert_eq!(info.defaulted, ModInfoDefaulted::default());
}

#[test]
fn blank_values_fall_back_to_defaults() {
    let info = parse_modinfo(br#"{"Version":"","Category":"Skin"}"#);
    assert_eq!(info.version, "1.0");
    assert!(info.defaulted.version);
    assert_eq!(info.category, "Skin");
    assert!(!info.defaulted.category);
    assert_eq!(version_of(r#"{"Version":"  "}"#), "1.0");
    assert_eq!(version_of(r#"{"Version":" 2.0 "}"#), " 2.0 ");
    assert_eq!(version_of(r#"{"Version":null}"#), "1.0");
    assert_eq!(version_of(r#"{"version":"3"}"#), "1.0");
}

#[test]
fn malformed_documents_are_invalid() {
    for text in [
        "not valid json {{",
        r#"{"Version":"1",}"#,
        "{\"Version\":\"1\" /* c */}",
        "",
        r#"["Version"]"#,
        r#"{"Version":1}"#,
        r#"{"Version":1,"Version":"2"}"#,
        r#"{"Version":"2","Version":1}"#,
        r#"{"Version":"1"} x"#,
    ] {
        let info = parse_modinfo(text.as_bytes());
        assert_eq!(info, invalid(), "{text:?}");
    }
}

#[test]
fn top_level_null_applies_nothing() {
    let info = parse_modinfo(b"null");
    assert_eq!(info.origin, ModInfoOrigin::File);
    assert_eq!(info.version, "1.0");
    assert!(info.defaulted.author);
}

#[test]
fn duplicate_keys_last_wins() {
    assert_eq!(version_of(r#"{"Version":"1","Version":"2"}"#), "2");
}

#[test]
fn nesting_limit_is_64() {
    let nested = |depth: usize| format!("{{\"X\":{}{}}}", "[".repeat(depth), "]".repeat(depth));
    assert_eq!(
        parse_modinfo(nested(63).as_bytes()).origin,
        ModInfoOrigin::File
    );
    assert_eq!(parse_modinfo(nested(64).as_bytes()), invalid());
}

#[test]
fn byte_order_marks() {
    let json = r#"{"Version":"7"}"#;
    let utf16 = |big: bool| -> Vec<u8> {
        json.encode_utf16()
            .flat_map(|unit| {
                if big {
                    unit.to_be_bytes()
                } else {
                    unit.to_le_bytes()
                }
            })
            .collect()
    };
    let utf32 = |big: bool| -> Vec<u8> {
        json.chars()
            .flat_map(|c| {
                if big {
                    u32::from(c).to_be_bytes()
                } else {
                    u32::from(c).to_le_bytes()
                }
            })
            .collect()
    };
    let cases: [(&[u8], Vec<u8>); 5] = [
        (&[0xEF, 0xBB, 0xBF], json.as_bytes().to_vec()),
        (&[0xFF, 0xFE], utf16(false)),
        (&[0xFE, 0xFF], utf16(true)),
        (&[0xFF, 0xFE, 0x00, 0x00], utf32(false)),
        (&[0x00, 0x00, 0xFE, 0xFF], utf32(true)),
    ];
    for (bom, body) in cases {
        let bytes = [bom, body.as_slice()].concat();
        assert_eq!(decode_modinfo_text(&bytes), json, "BOM {bom:02X?}");
        assert_eq!(parse_modinfo(&bytes).version, "7", "BOM {bom:02X?}");
    }
}

#[test]
fn invalid_utf8_becomes_replacement_character() {
    let info = parse_modinfo(b"{\"Author\":\"R\xFFm\"}");
    assert_eq!(info.author, "R\u{FFFD}m");
}

#[test]
fn larger_than_one_mebibyte_is_invalid() {
    let padding = " ".repeat(MODINFO_MAX_BYTES);
    let text = format!(r#"{{"Version":"9"}}{padding}"#);
    assert_eq!(parse_modinfo(text.as_bytes()), invalid());
}
