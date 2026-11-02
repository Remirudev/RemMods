//! P4 §8.1 X1: the Python-crafted zip fixtures through core. A no-op unless
//! `REMMODS_ARCHIVE_FIXTURES` names a folder made by `scripts/make-archive-fixtures.py`.
#![allow(clippy::expect_used)]

mod archive_support;

use std::path::Path;

use archive_support::{RecordingTarget, extract_all, full_plan};
use remmods_core::archive::{
    self, ArchiveError, EntryKind, Limits, NameEncoding, entry_is_unsafe, normalize,
};
use serde_json::Value;

#[test]
fn x1_python_fixtures_match_their_manifest() {
    let Some(dir) = std::env::var_os("REMMODS_ARCHIVE_FIXTURES") else {
        return;
    };
    let dir = Path::new(&dir);
    let manifest = std::fs::read_to_string(dir.join("manifest.json")).expect("manifest.json");
    let items: Vec<Value> = serde_json::from_str(&manifest).expect("manifest is a JSON list");
    assert!(!items.is_empty(), "empty manifest");
    for item in &items {
        check_item(dir, item);
    }
}

fn check_item(dir: &Path, item: &Value) {
    let file = item["file"].as_str().expect("file");
    let expect = item["expect"].as_str().expect("expect");
    let limits = Limits::default();
    let opened = archive::open(&dir.join(file), &limits);
    match expect {
        "ok" => {
            let reader = opened.unwrap_or_else(|error| panic!("{file}: {error:?}"));
            let entries = normalize(reader.listing(), NameEncoding::Auto, &limits);
            assert!(!entries.iter().any(entry_is_unsafe), "{file}: unsafe entry");
            if let Some(files) = item.get("files").and_then(Value::as_array) {
                let mut expected: Vec<String> = files
                    .iter()
                    .map(|path| path.as_str().expect("file path").to_owned())
                    .collect();
                let mut actual: Vec<String> = entries
                    .iter()
                    .filter(|entry| entry.kind == EntryKind::File && entry.problem.is_none())
                    .map(|entry| entry.path.join("/"))
                    .collect();
                expected.sort();
                actual.sort();
                assert_eq!(actual, expected, "{file}");
            }
        }
        "unsafe" => {
            let reader = opened.unwrap_or_else(|error| panic!("{file}: {error:?}"));
            let entries = normalize(reader.listing(), NameEncoding::Auto, &limits);
            let problems: Vec<String> = entries
                .iter()
                .filter(|entry| entry_is_unsafe(entry))
                .filter_map(|entry| entry.problem.map(|problem| format!("{problem:?}")))
                .collect();
            assert!(!problems.is_empty(), "{file}: expected an unsafe entry");
            if let Some(wanted) = item.get("problems").and_then(Value::as_array) {
                for problem in wanted {
                    let name = problem.as_str().expect("problem name");
                    assert!(
                        problems
                            .iter()
                            .any(|found| found.eq_ignore_ascii_case(name)),
                        "{file}: {name} not in {problems:?}"
                    );
                }
            }
        }
        "error" => {
            let error = match opened {
                Err(error) => error,
                Ok(mut reader) => {
                    let plan = full_plan(reader.as_ref());
                    let mut target = RecordingTarget::new();
                    let result = extract_all(reader.as_mut(), &plan, &mut target);
                    assert!(target.within_declared(), "{file}");
                    result.expect_err(file)
                }
            };
            if let Some(name) = item.get("error").and_then(Value::as_str) {
                assert!(
                    error_matches(&error, name),
                    "{file}: {error:?} is not {name}"
                );
            }
        }
        other => panic!("{file}: unknown expect {other}"),
    }
}

/// `name` is the variant (`Corrupt`, `encrypted`) or, for `Unsupported`, the reason
/// (`MultiVolume`); case-insensitive.
fn error_matches(error: &ArchiveError, name: &str) -> bool {
    let debug = format!("{error:?}");
    let variant = debug.split([' ', '{', '(']).next().unwrap_or_default();
    let reason = match error {
        ArchiveError::Unsupported { reason } => format!("{reason:?}"),
        _ => String::new(),
    };
    variant.eq_ignore_ascii_case(name) || reason.eq_ignore_ascii_case(name)
}
