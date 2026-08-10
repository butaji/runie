use super::*;
use crate::types::ThinkingLevel;

#[test]
fn every_declared_effort_level_maps_without_inventing_unsupported_values() {
    let model = Model {
        thinking_level_map: Some(crate::types::ThinkingLevelMap {
            off: Some("off-wire".into()),
            minimal: Some("minimal-wire".into()),
            low: Some("low-wire".into()),
            medium: Some("medium-wire".into()),
            high: Some("high-wire".into()),
            xhigh: Some("xhigh-wire".into()),
            max: Some("max-wire".into()),
        }),
        ..Default::default()
    };
    for (level, expected) in [
        (ThinkingLevel::Off, "off-wire"),
        (ThinkingLevel::Minimal, "minimal-wire"),
        (ThinkingLevel::Low, "low-wire"),
        (ThinkingLevel::Medium, "medium-wire"),
        (ThinkingLevel::High, "high-wire"),
        (ThinkingLevel::XHigh, "xhigh-wire"),
        (ThinkingLevel::Max, "max-wire"),
    ] {
        let options = SimpleStreamOptions {
            reasoning: Some(level),
            ..Default::default()
        };
        assert_eq!(
            mapped_reasoning(&model, Some(&options)).as_deref(),
            Some(expected)
        );
    }
    let options = SimpleStreamOptions {
        reasoning: Some(ThinkingLevel::Max),
        ..Default::default()
    };
    assert_eq!(mapped_reasoning(&Model::default(), Some(&options)), None);
}
