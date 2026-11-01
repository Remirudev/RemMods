#![allow(clippy::expect_used)]

mod game_support;

use game_support::{steam_fixture, templated};
use remmods_core::game::{
    LibraryFileError, Vdf, VdfError, VdfErrorKind, parse_library_folders, parse_vdf,
    parse_vdf_bytes,
};

fn library_paths(bytes: &[u8]) -> Vec<String> {
    parse_library_folders(bytes).expect("library list parses")
}

fn vdf_error(text: &str) -> VdfErrorKind {
    parse_vdf(text).expect_err("must fail").kind
}

#[test]
fn new_format_lists_root_and_library() {
    let text = templated("libraryfolders.new.vdf", "/steam", "/lib1");
    assert_eq!(library_paths(text.as_bytes()), ["/steam", "/lib1"]);
}

#[test]
fn templating_escapes_backslashes_and_quotes() {
    let text = templated("libraryfolders.new.vdf", r"C:\Steam", r#"D:\a"b"#);
    assert_eq!(library_paths(text.as_bytes()), [r"C:\Steam", r#"D:\a"b"#]);
}

#[test]
fn format_2021_skips_stats_and_unmounted() {
    let paths = library_paths(&steam_fixture("libraryfolders.new-2021.vdf"));
    assert_eq!(paths, [r"C:\Program Files (x86)\Steam", r"D:\SteamLibrary"]);
}

#[test]
fn old_format_lists_string_values() {
    let paths = library_paths(&steam_fixture("libraryfolders.old.vdf"));
    assert_eq!(paths, [r"D:\Steam", r"E:\Games\SteamLibrary"]);
}

#[test]
fn broken_file_reports_line_14() {
    let error = parse_library_folders(&steam_fixture("libraryfolders.broken.vdf"))
        .expect_err("broken file fails");
    let LibraryFileError::Vdf(VdfError { kind, line }) = error else {
        panic!("expected a VDF error, got {error:?}");
    };
    assert_eq!((kind, line), (VdfErrorKind::UnterminatedString, 14));
    assert_eq!(error.code(), "unterminatedString");
    assert_eq!(error.line(), Some(14));
}

#[test]
fn small_inputs_give_the_listed_errors() {
    let cases = [
        ("", VdfErrorKind::UnexpectedEof),
        ("   // only a comment\n", VdfErrorKind::UnexpectedEof),
        ("\"libraryfolders\"", VdfErrorKind::ExpectedOpenBrace),
        ("\"libraryfolders\" \"x\"", VdfErrorKind::ExpectedOpenBrace),
        ("\"libraryfolders\" {", VdfErrorKind::UnexpectedEof),
        ("\"libraryfolders\" { \"0\" {", VdfErrorKind::UnexpectedEof),
        (
            "\"libraryfolders\" { \"0\" }",
            VdfErrorKind::UnexpectedCloseBrace,
        ),
        ("\"libraryfolders\" { \"\" \"x\" }", VdfErrorKind::EmptyKey),
        ("}", VdfErrorKind::UnexpectedCloseBrace),
        ("\"a\" { \"b\" \"c", VdfErrorKind::UnterminatedString),
    ];
    for (text, expected) in cases {
        assert_eq!(vdf_error(text), expected, "input {text:?}");
    }
}

#[test]
fn deep_nesting_is_too_deep() {
    let mut text = String::from("\"root\" {");
    for _ in 0..70 {
        text.push_str(" \"k\" {");
    }
    for _ in 0..71 {
        text.push_str(" }");
    }
    assert_eq!(vdf_error(&text), VdfErrorKind::TooDeep);
}

#[test]
fn app_state_root_is_the_wrong_root_for_libraries() {
    let error =
        parse_library_folders(&steam_fixture("appmanifest_1778820.acf")).expect_err("wrong root");
    assert!(matches!(error, LibraryFileError::WrongRoot));
    assert_eq!(error.code(), "wrongRoot");
    assert_eq!(error.line(), None);
}

#[test]
fn invalid_utf8_and_oversized_input_are_rejected() {
    let not_utf8 = parse_vdf_bytes(b"\"a\" { \"b\" \"\xff\" }").expect_err("not UTF-8");
    assert_eq!(not_utf8.kind, VdfErrorKind::NotUtf8);
    let mut large = b"\"a\" { \"b\" \"".to_vec();
    large.resize(1 << 21, b'x');
    assert_eq!(
        parse_vdf_bytes(&large).expect_err("too large").kind,
        VdfErrorKind::TooLarge
    );
}

#[test]
fn error_codes_are_camel_case() {
    let codes = [
        (VdfErrorKind::UnexpectedEof, "unexpectedEof"),
        (VdfErrorKind::UnterminatedString, "unterminatedString"),
        (VdfErrorKind::ExpectedOpenBrace, "expectedOpenBrace"),
        (VdfErrorKind::UnexpectedCloseBrace, "unexpectedCloseBrace"),
        (VdfErrorKind::EmptyKey, "emptyKey"),
        (VdfErrorKind::TooDeep, "tooDeep"),
        (VdfErrorKind::NotUtf8, "notUtf8"),
        (VdfErrorKind::TooLarge, "tooLarge"),
    ];
    for (kind, code) in codes {
        assert_eq!(kind.code(), code);
    }
}

#[test]
fn token_rules_of_appendix_b7() {
    let text = "#base \"other.vdf\"\r\n// comment\r\nRoot [$WIN32] {\r\n\tKey Value // trailing\r\n\t\"Quoted\" \"a\\\\b\\\"c\\td\\qe\"\r\n\t\"Multi\" \"line one\r\nline two\"\r\n\t\"osx\" \"1\" [$OSX]\r\n\t\"cond\" [!$X360] \"2\"\r\n\t\"KEY\" \"dup\"\r\n\tSub { inner \"x\" } [$LINUX]\r\n}\r\n";
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    let (root, tree) = parse_vdf_bytes(&bytes).expect("token test parses");
    assert_eq!(root, "Root");
    let expected = vec![
        ("Key".to_owned(), Vdf::Str("Value".to_owned())),
        ("Quoted".to_owned(), Vdf::Str("a\\b\"c\td\\qe".to_owned())),
        (
            "Multi".to_owned(),
            Vdf::Str("line one\r\nline two".to_owned()),
        ),
        ("osx".to_owned(), Vdf::Str("1".to_owned())),
        ("cond".to_owned(), Vdf::Str("2".to_owned())),
        ("KEY".to_owned(), Vdf::Str("dup".to_owned())),
        (
            "Sub".to_owned(),
            Vdf::Obj(vec![("inner".to_owned(), Vdf::Str("x".to_owned()))]),
        ),
    ];
    assert_eq!(tree.entries(), expected.as_slice());
    assert_eq!(tree.str("key"), Some("Value"));
    assert_eq!(tree.get("sub").and_then(|sub| sub.str("INNER")), Some("x"));
}
