#![allow(clippy::expect_used)]

use proptest::prelude::*;
use remmods_core::game::{Vdf, parse_vdf, parse_vdf_bytes};

fn escaped(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

fn render(entries: &[(String, Vdf)], out: &mut String) {
    for (key, value) in entries {
        out.push_str(&format!("\"{}\" ", escaped(key)));
        match value {
            Vdf::Str(text) => out.push_str(&format!("\"{}\"\n", escaped(text))),
            Vdf::Obj(children) => {
                out.push_str("{\n");
                render(children, out);
                out.push_str("}\n");
            }
        }
    }
}

/// Keys are non-empty; texts avoid the escapes the reader turns into control characters.
fn text(min: usize) -> impl Strategy<Value = String> {
    proptest::string::string_regex(&format!("[a-zA-Z0-9 _./:\"\\\\{{}}äß-]{{{min},12}}"))
        .expect("valid regex")
        .prop_filter("no reader escapes", |text| {
            !["\\n", "\\t", "\\r", "\\'", "\\?"]
                .iter()
                .any(|escape| text.contains(escape))
        })
}

fn tree() -> impl Strategy<Value = Vec<(String, Vdf)>> {
    let leaf = text(0).prop_map(Vdf::Str);
    let value = leaf.prop_recursive(4, 32, 4, |inner| {
        proptest::collection::vec((text(1), inner), 0..4).prop_map(Vdf::Obj)
    });
    proptest::collection::vec((text(1), value), 0..5)
}

proptest! {
    #![proptest_config(ProptestConfig { failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn arbitrary_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
        let _ = parse_vdf_bytes(&bytes);
    }

    #[test]
    fn arbitrary_strings_never_panic(text in "\\PC{0,256}") {
        let _ = parse_vdf(&text);
    }

    #[test]
    fn vdf_like_strings_never_panic(text in "[\"{}\\\\/ \\na-z\\[\\]$!#]{0,128}") {
        let _ = parse_vdf(&text);
    }

    #[test]
    fn rendered_trees_parse_back_equal(root in text(1), entries in tree()) {
        let mut rendered = format!("\"{}\"\n{{\n", escaped(&root));
        render(&entries, &mut rendered);
        rendered.push_str("}\n");
        let (parsed_root, parsed) = parse_vdf(&rendered).map_err(|error| {
            TestCaseError::fail(format!("{error:?} in {rendered:?}"))
        })?;
        prop_assert_eq!(parsed_root, root);
        prop_assert_eq!(parsed, Vdf::Obj(entries));
    }
}
