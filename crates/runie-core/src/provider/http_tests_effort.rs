use super::*;
use crate::types::ThinkingLevel;

#[test]
fn provider_profile_selects_wire_field_from_model_aliases() {
    let cases = [
        ("openai-responses", "", "reasoning_effort"),
        ("", "anthropic", "reasoning"),
        ("", "google", "reasoning"),
        ("", "minimax", "reasoning_effort"),
        ("unknown", "unknown", "reasoning_effort"),
    ];
    for (api, provider, expected) in cases {
        let model = Model {
            api: api.into(),
            provider: provider.into(),
            ..Default::default()
        };
        assert_eq!(
            super::ProviderRequestProfile::for_model(&model)
                .effort_field()
                .key(),
            expected
        );
    }
}

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

#[test]
fn model_provider_effort_uses_the_profile_wire_field() {
    let model = Model {
        provider: "anthropic".into(),
        thinking_level_map: Some(crate::types::ThinkingLevelMap {
            high: Some("high-wire".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let payload = with_model_provider_effort(
        serde_json::json!({}),
        &model,
        Some(&SimpleStreamOptions {
            reasoning: Some(ThinkingLevel::High),
            ..Default::default()
        }),
    );
    assert_eq!(payload["reasoning"], "high-wire");
}
