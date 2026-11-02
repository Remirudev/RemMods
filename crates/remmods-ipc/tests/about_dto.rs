#![allow(clippy::expect_used)]

use remmods_ipc::Notices;
use serde_json::{Value, json};

#[test]
fn notices_serialize_with_the_two_text_keys() {
    let notices = Notices {
        rust: "rust notices".into(),
        npm: "npm notices".into(),
    };
    let value: Value = serde_json::to_value(&notices).expect("serialize");
    assert_eq!(value, json!({"rust": "rust notices", "npm": "npm notices"}));
}

#[test]
fn notices_round_trip() {
    let notices = Notices {
        rust: "a\nb".into(),
        npm: "\u{fc}".into(),
    };
    let text = serde_json::to_string(&notices).expect("serialize");
    let back: Notices = serde_json::from_str(&text).expect("deserialize");
    assert_eq!(back, notices);
}
