#![allow(clippy::expect_used)]

use remmods_ipc::Notices;
use serde_json::{Value, json};

#[test]
fn notices_serialize_with_the_three_text_keys() {
    let notices = Notices {
        license: "licence text".into(),
        rust: "rust notices".into(),
        npm: "npm notices".into(),
    };
    let value: Value = serde_json::to_value(&notices).expect("serialize");
    assert_eq!(
        value,
        json!({"license": "licence text", "rust": "rust notices", "npm": "npm notices"})
    );
}

#[test]
fn notices_round_trip() {
    let notices = Notices {
        license: "a\nb".into(),
        rust: String::new(),
        npm: "\u{fc}".into(),
    };
    let text = serde_json::to_string(&notices).expect("serialize");
    let back: Notices = serde_json::from_str(&text).expect("deserialize");
    assert_eq!(back, notices);
}
