use remmods_core::platform::RawName;
use remmods_core::safe_fs::names::NameIssue::*;
use remmods_core::safe_fs::names::*;

const VALIDATE_CASES: &[(&str, Option<NameIssue>)] = &[
    ("NeonSkin", None),
    ("Neon Skin", None),
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
    ("_private_pack", None),
    ("pack_tmp_notes", None),
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
        "_Neon Skin_tmp_5f3ccf7c5cff4a1e9b20d6e8c1a7b394",
        Some(TempName),
    ),
    (
        "_Neon Skin_tmp_5F3CCF7C5CFF4A1E9B20D6E8C1A7B394",
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
    ("NeonSkin", "neonskin"),
    ("NEONSKIN", "neonskin"),
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
        (b"Neon", "Neon"),
        (b"Neon\xFFSkin", r"Neon\xFFSkin"),
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
        "_Neon Skin_tmp_5f3ccf7c5cff4a1e9b20d6e8c1a7b394"
    )));
    assert!(!is_install_temp_name(OsStr::new("Neon Skin")));
    assert!(has_disabled_prefix_ascii(OsStr::new(".DISABLED_x")));
    assert!(has_disabled_prefix_ascii(OsStr::new(".disabled_")));
    assert!(!has_disabled_prefix_ascii(OsStr::new(".di\u{17F}abled_x")));
    assert!(!has_disabled_prefix_ascii(OsStr::new(".disabled")));
}

#[test]
fn new_name_rules_do_not_apply_to_existing_names() {
    assert_eq!(windows_name_issue(".disabled_X"), None);
    assert_eq!(
        windows_name_issue("_x_tmp_5f3ccf7c5cff4a1e9b20d6e8c1a7b394"),
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

#[test]
fn os_str_cut_removes_ascii_ends_only() {
    use remmods_core::platform::{os_str_cut, os_string, raw_name};
    use std::ffi::{OsStr, OsString};

    assert_eq!(
        os_str_cut(OsStr::new(".disabled_X"), 10, 0),
        Some(OsString::from("X"))
    );
    assert_eq!(
        os_str_cut(OsStr::new("Neon.pak-x"), 0, 2),
        Some(OsString::from("Neon.pak"))
    );
    assert_eq!(
        os_str_cut(OsStr::new("\u{E9}t\u{E9}.pak-x"), 0, 2),
        Some(OsString::from("\u{E9}t\u{E9}.pak"))
    );
    assert_eq!(os_str_cut(OsStr::new("\u{E9}x"), 1, 0), None);
    assert_eq!(os_str_cut(OsStr::new("x\u{E9}"), 0, 1), None);
    assert_eq!(os_str_cut(OsStr::new("ab"), 2, 1), None);
    assert_eq!(os_str_cut(OsStr::new("ab"), 1, 1), Some(OsString::new()));

    let name = OsStr::new("Caf\u{E9}.pak");
    assert_eq!(os_string(&raw_name(name)).as_deref(), Some(name));
    let foreign = match raw_name(name) {
        RawName::Bytes(_) => RawName::Wide(vec![0x41]),
        RawName::Wide(_) => RawName::Bytes(vec![0x41]),
    };
    assert_eq!(os_string(&foreign), None);
}

/// S1: only paths Win32 keeps as they are may reach the shell.
#[test]
fn shell_safe_path_table() {
    use std::path::Path;

    use remmods_core::platform::{OsKind, os_kind};

    let base = r"C:\Games\Paks\Mods";
    let cases: &[(String, bool)] = &[
        (format!(r"{base}\Neon"), true),
        (r"\\?\C:\Games\Mods\Neon".to_owned(), true),
        (r"\\?\UNC\srv\share\Mods\Neon".to_owned(), true),
        (r"\\srv\share\Mods\Neon".to_owned(), true),
        ("/tmp/x/Mods/Neon".to_owned(), true),
        (format!(r"{base}\Foo."), false),
        (format!(r"{base}\Foo "), false),
        (format!(r"{base}\CON"), false),
        (format!(r"{base}\con.txt"), false),
        (format!(r"{base}\CONIN$"), false),
        (format!("{base}\\COM\u{B9}"), false),
        (format!(r"{base}\a:b"), false),
        (format!(r"{base}\..\x"), false),
        (format!(r"{base}\.\x"), false),
        (r"\\?\Volume{0}\Mods\Neon".to_owned(), false),
        (r"\\.\C:\Mods\Neon".to_owned(), false),
        (format!(r"{base}\Neon\"), false),
    ];
    for (path, safe) in cases {
        assert_eq!(shell_safe_path(Path::new(path)), *safe, "{path}");
    }

    let at_limit = |units: usize| {
        let head = r"C:\Mods\";
        format!("{head}{}", "a".repeat(units - head.len()))
    };
    assert_eq!(utf16_len(&at_limit(259)), 259);
    assert!(shell_safe_path(Path::new(&at_limit(259))));
    assert!(!shell_safe_path(Path::new(&at_limit(260))));
    let verbatim = format!(r"\\?\{}", at_limit(259));
    assert!(
        shell_safe_path(Path::new(&verbatim)),
        "the plain form counts"
    );

    if os_kind() == OsKind::Linux {
        let name = remmods_core::platform::os_string(&RawName::Bytes(b"Neon\xFF".to_vec()))
            .expect("bytes are an OS name on Linux");
        assert!(!shell_safe_path(&Path::new("/tmp/Mods").join(name)));
    }
}
