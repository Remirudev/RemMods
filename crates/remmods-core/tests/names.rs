use remmods_core::platform::RawName;
use remmods_core::safe_fs::names::NameIssue::*;
use remmods_core::safe_fs::names::*;

const VALIDATE_CASES: &[(&str, Option<NameIssue>)] = &[
    ("CoolSkin", None),
    ("Cool Skin", None),
    (".hidden", None),
    ("..x", None),
    ("x..y", None),
    (" Leading", None),
    ("a\u{A0}", None),
    ("a\u{3000}", None),
    ("a\u{200B}b", None),
    ("a\u{200D}b", None),
    ("a\u{FEFF}b", None),
    ("a\u{2028}b", None),
    ("\u{DC}n\u{EF}c\u{F6}d\u{E9} Mod", None),
    ("\u{65E5}\u{672C}\u{8A9E}MOD", None),
    ("\u{1F600} skin", None),
    ("PROGRA~1", None),
    ("COM0", None),
    ("LPT0", None),
    ("COM10", None),
    ("LPT", None),
    ("COM\u{2074}", None),
    ("COM\u{2081}", None),
    ("COM\u{FF11}", None),
    ("CONERR$", None),
    ("NUL0", None),
    (" NUL", None),
    ("NULL", None),
    ("CONSOLE", None),
    ("bar.com9", None),
    ("x.NUL", None),
    ("CON_", None),
    ("C\u{41E}N", None),
    ("disabled_X", None),
    ("x.disabled_y", None),
    (".disabled", None),
    ("_hidden_mod", None),
    ("my_tmp_mod", None),
    ("", Some(Empty)),
    (".", Some(DotName)),
    ("..", Some(DotName)),
    ("...", Some(TrailingDotOrSpace)),
    ("Trailing ", Some(TrailingDotOrSpace)),
    ("Trailing.", Some(TrailingDotOrSpace)),
    ("Trailing. ", Some(TrailingDotOrSpace)),
    ("a<b", Some(ForbiddenChar('<'))),
    ("a>b", Some(ForbiddenChar('>'))),
    ("a:b", Some(ForbiddenChar(':'))),
    ("a\"b", Some(ForbiddenChar('"'))),
    ("a/b", Some(ForbiddenChar('/'))),
    ("a\\b", Some(ForbiddenChar('\\'))),
    ("a|b", Some(ForbiddenChar('|'))),
    ("a?b", Some(ForbiddenChar('?'))),
    ("a*b", Some(ForbiddenChar('*'))),
    ("a\u{0}b", Some(ControlChar('\u{0}'))),
    ("a\u{9}b", Some(ControlChar('\u{9}'))),
    ("a\u{1F}b", Some(ControlChar('\u{1F}'))),
    ("a\u{7F}b", Some(ControlChar('\u{7F}'))),
    ("a\u{85}b", Some(ControlChar('\u{85}'))),
    ("a\u{9F}b", Some(ControlChar('\u{9F}'))),
    ("evil\u{202E}gpj.exe", Some(BidiControl('\u{202E}'))),
    ("a\u{200E}", Some(BidiControl('\u{200E}'))),
    ("a\u{200F}", Some(BidiControl('\u{200F}'))),
    ("a\u{61C}", Some(BidiControl('\u{61C}'))),
    ("a\u{2066}b\u{2069}", Some(BidiControl('\u{2066}'))),
    ("a\u{202A}", Some(BidiControl('\u{202A}'))),
    ("CON", Some(ReservedDeviceName)),
    ("con", Some(ReservedDeviceName)),
    ("Con.txt", Some(ReservedDeviceName)),
    ("NUL", Some(ReservedDeviceName)),
    ("nul.tar.gz", Some(ReservedDeviceName)),
    ("NUL .txt", Some(ReservedDeviceName)),
    ("NUL  ", Some(TrailingDotOrSpace)),
    ("AUX", Some(ReservedDeviceName)),
    ("PRN.pak", Some(ReservedDeviceName)),
    ("COM1", Some(ReservedDeviceName)),
    ("com9.pak", Some(ReservedDeviceName)),
    ("COM\u{B9}", Some(ReservedDeviceName)),
    ("lpt\u{B2}.txt", Some(ReservedDeviceName)),
    ("LPT\u{B3}", Some(ReservedDeviceName)),
    ("CONIN$", Some(ReservedDeviceName)),
    ("conout$.log", Some(ReservedDeviceName)),
    (".disabled_X", Some(DisabledPrefix)),
    (".DISABLED_X", Some(DisabledPrefix)),
    (".Disabled_X", Some(DisabledPrefix)),
    (".di\u{17F}abled_x", Some(DisabledPrefix)),
    (".disabled_", Some(DisabledPrefix)),
    (
        "_Cool Skin_tmp_0123456789abcdef0123456789abcdef",
        Some(TempName),
    ),
    (
        "_Cool Skin_tmp_0123456789ABCDEF0123456789ABCDEF",
        Some(TempName),
    ),
    ("_tmp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", Some(TempName)),
    ("__tmp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", Some(TempName)),
    (
        "_a_tmp_b_tmp_cccccccccccccccccccccccccccccccc",
        Some(TempName),
    ),
    ("_x_tmp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", None),
    ("_x_tmp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", None),
    ("x_tmp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", None),
    ("_x_TMP_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", None),
    ("_x_tmp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaag", None),
];
const FOLD_CASES: &[(&str, &str)] = &[
    ("CoolSkin", "coolskin"),
    ("COOLSKIN", "coolskin"),
    ("Stra\u{DF}e", "strasse"),
    ("STRASSE", "strasse"),
    ("\u{1E9E}", "ss"),
    ("Caf\u{E9}", "caf\u{E9}"),
    ("Cafe\u{301}", "caf\u{E9}"),
    ("CAF\u{C9}", "caf\u{E9}"),
    ("\u{130}stanbul", "i\u{307}stanbul"),
    ("\u{131}stanbul", "\u{131}stanbul"),
    ("ISTANBUL", "istanbul"),
    ("\u{3A3}\u{391}\u{3A3}", "\u{3C3}\u{3B1}\u{3C3}"),
    ("\u{3C3}\u{3B1}\u{3C2}", "\u{3C3}\u{3B1}\u{3C3}"),
    ("\u{FB01}le", "file"),
    ("\u{1C4}", "\u{1C6}"),
    ("\u{1C5}", "\u{1C6}"),
    ("\u{212A}", "k"),
    ("\u{212B}", "\u{E5}"),
    ("\u{1FBC}", "\u{3B1}\u{3B9}"),
    ("\u{1FB3}", "\u{3B1}\u{3B9}"),
    ("\u{390}", "\u{390}"),
    ("\u{1F0}", "\u{1F0}"),
    ("\u{149}", "\u{2BC}n"),
    ("\u{13A0}", "\u{13A0}"),
    ("\u{AB70}", "\u{13A0}"),
    ("\u{FF21}", "\u{FF41}"),
    ("\u{2460}", "\u{2460}"),
    ("\u{345}", "\u{3B9}"),
    ("\u{3B1}\u{345}\u{301}", "\u{3AC}\u{3B9}"),
    ("\u{3B1}\u{301}\u{345}", "\u{3AC}\u{3B9}"),
    ("\u{2126}", "\u{3C9}"),
    ("\u{17F}", "s"),
    ("\u{FB05}", "st"),
    ("\u{216B}", "\u{217B}"),
    ("\u{24B6}", "\u{24D0}"),
    ("\u{10400}", "\u{10428}"),
    ("\u{A7DD}", "\u{277}"),
    ("\u{1DF95}", "ss"),
    ("\u{3B1}\u{300}\u{300}\u{345}", "\u{1F70}\u{300}\u{3B9}"),
    ("\u{391}\u{300}\u{300}\u{345}", "\u{1F70}\u{300}\u{3B9}"),
    (".DISABLED_X", ".disabled_x"),
    (".di\u{17F}abled_x", ".disabled_x"),
    ("Mods", "mods"),
    ("~MODS", "~mods"),
    ("LogicMods", "logicmods"),
];

#[test]
fn validate_vectors() {
    for (input, expected) in VALIDATE_CASES {
        assert_eq!(validate_name(input).err(), *expected, "input {input:?}");
    }
    assert_eq!(validate_name(&"a".repeat(255)), Ok(()));
    assert_eq!(validate_name(&"a".repeat(256)), Err(TooLong));
    assert_eq!(validate_name(&"\u{672C}".repeat(85)), Ok(()));
    assert_eq!(validate_name(&"\u{672C}".repeat(86)), Err(TooLong));
    assert_eq!(
        validate_name(&format!("{}abc", "\u{1F600}".repeat(63))),
        Ok(())
    );
    assert_eq!(validate_name(&"\u{1F600}".repeat(64)), Err(TooLong));
}

#[test]
fn fold_vectors() {
    for (input, expected) in FOLD_CASES {
        assert_eq!(fold_key(input), *expected, "input {input:?}");
    }
}

#[test]
fn escape_vectors() {
    let bytes: &[(&[u8], &str)] = &[
        (b"Cool", "Cool"),
        (b"Cool\xFFSkin", r"Cool\xFFSkin"),
        (b"caf\xE9", r"caf\xE9"),
        (b"\xED\xA0\x80", r"\xED\xA0\x80"),
        (b"a\\b", r"a\\b"),
        (b"a\\xFF", r"a\\xFF"),
        (b"a\xFF", r"a\xFF"),
        (b"\xC3", r"\xC3"),
        (b"\xF0\x9F\x98", r"\xF0\x9F\x98"),
        (b"\xF0\x9F\x98\x80", "\u{1F600}"),
        (b"\x80\x80", r"\x80\x80"),
    ];
    for (raw, key) in bytes {
        assert_eq!(escape_bytes(raw), *key);
        assert_eq!(
            unescape_key(key, KeyForm::Bytes),
            Some(RawName::Bytes(raw.to_vec()))
        );
    }
    let wide: &[(&[u16], &str)] = &[
        (&[0x41, 0x42], "AB"),
        (&[0x41, 0xD800, 0x42], r"A\u{D800}B"),
        (&[0xDC00], r"\u{DC00}"),
        (&[0xD83D, 0xDE00], "\u{1F600}"),
        (&[0xDE00, 0xD83D], r"\u{DE00}\u{D83D}"),
        (&[0x5C, 0xD800], r"\\\u{D800}"),
        (&[0xD800, 0xD800], r"\u{D800}\u{D800}"),
    ];
    for (raw, key) in wide {
        assert_eq!(escape_wide(raw), *key);
        assert_eq!(
            unescape_key(key, KeyForm::Wide),
            Some(RawName::Wide(raw.to_vec()))
        );
    }
    assert_eq!(
        String::from_utf8_lossy(b"a\xFFb"),
        String::from_utf8_lossy(b"a\xFEb")
    );
    assert_eq!(
        display_escaped("evil\u{202E}gpj.exe"),
        r"evil\u{202E}gpj.exe"
    );
}

#[test]
fn order_vector() {
    let mut names = vec![
        "b", "A", "a", "B", "\u{E4}", "\u{C4}", "\u{DF}", "ss", "SS", "a\u{308}", "Z", "_x", ".x",
        "10", "9",
    ];
    names.sort_by_key(|n| order_key(n));
    assert_eq!(
        names,
        [
            ".x", "10", "9", "_x", "A", "a", "B", "b", "SS", "ss", "\u{DF}", "Z", "a\u{308}",
            "\u{C4}", "\u{E4}"
        ]
    );
}

// Spec §8.1 additions (not Appendix D vectors).

use std::ffi::OsStr;

#[test]
fn unicode_data_pin() {
    assert_eq!(unicode_normalization::UNICODE_VERSION, (18, 0, 0));
    assert_eq!(fold_key("\u{A7DD}"), "\u{277}");
}

#[test]
fn temp_and_prefix_on_os_str() {
    assert!(is_install_temp_name(OsStr::new(
        "_Cool Skin_tmp_0123456789abcdef0123456789abcdef"
    )));
    assert!(!is_install_temp_name(OsStr::new("Cool Skin")));
    assert!(has_disabled_prefix_ascii(OsStr::new(".DISABLED_x")));
    assert!(has_disabled_prefix_ascii(OsStr::new(".disabled_")));
    assert!(!has_disabled_prefix_ascii(OsStr::new(".di\u{17F}abled_x")));
    assert!(!has_disabled_prefix_ascii(OsStr::new(".disabled")));
}

#[test]
fn new_name_rules_do_not_apply_to_existing_names() {
    assert_eq!(windows_name_issue(".disabled_X"), None);
    assert_eq!(
        windows_name_issue("_x_tmp_0123456789abcdef0123456789abcdef"),
        None
    );
}

#[test]
fn win32_and_portable_split() {
    let cjk_100 = "\u{672C}".repeat(100);
    assert_eq!(windows_name_issue(&cjk_100), None);
    assert_eq!(portable_name_issue(&cjk_100, 255), Some(TooLong));
    assert_eq!(windows_name_issue(&"\u{672C}".repeat(256)), Some(TooLong));
    for name in ["a\u{7F}b", "a\u{85}b"] {
        assert_eq!(windows_name_issue(name), None, "{name:?}");
        assert!(matches!(
            portable_name_issue(name, 255),
            Some(ControlChar(_))
        ));
    }
    assert_eq!(windows_name_issue("a\u{1F}b"), Some(ControlChar('\u{1F}')));
    assert_eq!(windows_name_issue("evil\u{202E}x"), None);
    assert_eq!(
        portable_name_issue("evil\u{202E}x", 255),
        Some(BidiControl('\u{202E}'))
    );
    let payload = format!("{}.pak", "x".repeat(250));
    assert_eq!(payload.len(), 254);
    assert_eq!(
        portable_name_issue(&payload, PAYLOAD_NAME_MAX_BYTES),
        Some(TooLong)
    );
    assert_eq!(portable_name_issue(&payload, 255), None);
    let logic = "y".repeat(246);
    assert_eq!(
        portable_name_issue(&logic, LOGIC_NAME_MAX_BYTES),
        Some(TooLong)
    );
}
