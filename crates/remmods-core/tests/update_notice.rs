use remmods_core::game::{UpdateDecision, decide_update};
use remmods_core::settings::LastSeenBuild;

fn stored(build: &str) -> LastSeenBuild {
    LastSeenBuild {
        game_root: "/games/TEKKEN 8".to_owned(),
        build_id: build.to_owned(),
    }
}

#[test]
fn decision_table() {
    let old = stored("100");
    assert_eq!(
        decide_update(None, false, Some("100")),
        UpdateDecision::StoreSilently
    );
    assert_eq!(
        decide_update(None, true, Some("100")),
        UpdateDecision::StoreSilently
    );
    assert_eq!(
        decide_update(Some(&old), false, Some("200")),
        UpdateDecision::StoreSilently
    );
    assert_eq!(
        decide_update(Some(&old), true, Some("100")),
        UpdateDecision::Unchanged
    );
    assert_eq!(
        decide_update(Some(&old), true, Some("200")),
        UpdateDecision::Notice {
            from: "100".to_owned(),
            to: "200".to_owned()
        }
    );
    assert_eq!(
        decide_update(Some(&old), true, None),
        UpdateDecision::Unavailable
    );
    assert_eq!(
        decide_update(None, false, None),
        UpdateDecision::Unavailable
    );
}
