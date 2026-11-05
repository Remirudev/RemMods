//! Spec P5 G21: the graphics DTOs on the wire.
#![allow(clippy::expect_used)]

use remmods_core::graphics as core;
use remmods_ipc::{GraphicsMode, GraphicsState, GraphicsStep};
use serde_json::{Value, json};

fn to_json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialize")
}

#[test]
fn modes_are_camel_case_both_ways() {
    let cases = [
        (GraphicsMode::Auto, "auto", core::GraphicsMode::Auto),
        (
            GraphicsMode::ForceShm,
            "forceShm",
            core::GraphicsMode::ForceShm,
        ),
        (
            GraphicsMode::Software,
            "software",
            core::GraphicsMode::Software,
        ),
    ];
    for (mode, text, core_mode) in cases {
        assert_eq!(to_json(&mode), json!(text));
        let parsed: GraphicsMode = serde_json::from_value(json!(text)).expect("deserialize");
        assert_eq!(parsed, mode);
        assert_eq!(core::GraphicsMode::from(mode), core_mode);
        assert_eq!(GraphicsMode::from(core_mode), mode);
    }
}

#[test]
fn state_json() {
    let state = GraphicsState {
        supported: true,
        mode: GraphicsMode::ForceShm,
        effective: GraphicsMode::Software,
        safe_flag: true,
        user_env: false,
        step: Some(GraphicsStep {
            from: GraphicsMode::Auto,
            to: GraphicsMode::ForceShm,
        }),
        read_only: false,
    };
    assert_eq!(
        to_json(&state),
        json!({
            "supported": true,
            "mode": "forceShm",
            "effective": "software",
            "safeFlag": true,
            "userEnv": false,
            "step": {"from": "auto", "to": "forceShm"},
            "readOnly": false
        })
    );
    assert_eq!(
        to_json(&GraphicsState::unsupported()),
        json!({
            "supported": false,
            "mode": "auto",
            "effective": "auto",
            "safeFlag": false,
            "userEnv": false,
            "step": null,
            "readOnly": false
        })
    );
}

#[test]
fn step_from_core_drops_the_time() {
    let step = core::GraphicsStep {
        from: core::GraphicsMode::ForceShm,
        to: core::GraphicsMode::Software,
        at: "2026-11-02T10:00:00+01:00".to_owned(),
    };
    assert_eq!(
        GraphicsStep::from(&step),
        GraphicsStep {
            from: GraphicsMode::ForceShm,
            to: GraphicsMode::Software,
        }
    );
}
