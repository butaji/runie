use super::*;
use crate::types::ThinkingLevel;

#[test]
fn provider_profile_selects_wire_field_from_model_aliases() {
    let cases = [
        ("openai-responses", "", "reasoning_effort"),
        ("", "anthropic", "effort"),
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
fn provider_profiles_are_stable_replay_data() {
    for profile in [
        super::ProviderRequestProfile::OpenAiResponses,
        super::ProviderRequestProfile::OpenAiChat,
        super::ProviderRequestProfile::Anthropic,
        super::ProviderRequestProfile::Gemini,
        super::ProviderRequestProfile::MiniMax,
        super::ProviderRequestProfile::Generic,
    ] {
        let encoded = serde_json::to_string(&profile).unwrap();
        let decoded: super::ProviderRequestProfile = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, profile);
        assert_eq!(
            super::ProviderRequestProfile::from_wire_name(profile.wire_name()),
            Some(profile)
        );
    }
    assert_eq!(
        super::ProviderRequestProfile::from_wire_name("unknown"),
        None
    );
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
    assert_eq!(payload["output_config"]["effort"], "high-wire");
    let explicit = with_model_provider_effort(
        serde_json::json!({"output_config":{"effort":"explicit"}}),
        &model,
        Some(&SimpleStreamOptions {
            reasoning: Some(ThinkingLevel::High),
            ..Default::default()
        }),
    );
    assert_eq!(explicit["output_config"]["effort"], "explicit");
}

#[test]
fn provider_effort_matrix_preserves_shape_and_omits_unsupported_levels() {
    let cases = [
        ("openai-responses", "reasoning_effort", false),
        ("openai-chat", "reasoning_effort", false),
        ("anthropic", "effort", true),
        ("gemini", "reasoning", false),
        ("minimax", "reasoning_effort", false),
        ("generic", "reasoning_effort", false),
    ];
    for (provider, key, nested) in cases {
        let model = Model {
            provider: provider.into(),
            thinking_level_map: Some(crate::types::ThinkingLevelMap {
                high: Some("wire-high".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let options = SimpleStreamOptions {
            reasoning: Some(ThinkingLevel::High),
            ..Default::default()
        };
        let shaped = with_model_provider_effort(serde_json::json!({}), &model, Some(&options));
        let value = if nested {
            shaped["output_config"][key].clone()
        } else {
            shaped[key].clone()
        };
        assert_eq!(value, serde_json::json!("wire-high"), "{provider}");

        assert_unsupported_effort_is_omitted(provider, key, &options);
    }
}

fn assert_unsupported_effort_is_omitted(provider: &str, key: &str, options: &SimpleStreamOptions) {
    let unsupported = with_model_provider_effort(
        serde_json::json!({}),
        &Model {
            provider: provider.into(),
            ..Default::default()
        },
        Some(options),
    );
    assert!(unsupported.get(key).is_none());
    assert!(unsupported.get("output_config").is_none());
}
