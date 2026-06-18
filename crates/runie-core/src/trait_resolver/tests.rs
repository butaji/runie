use crate::model_catalog::{ModelCapabilities, ModelInfo};
use crate::orchestrator::ModelTrait;
use crate::trait_resolver::{ModelProfile, ModelResolver, ResolverError};

fn resolver(profiles: Vec<ModelProfile>) -> ModelResolver {
    ModelResolver::new(profiles)
}

fn p(provider: &str, model: &str, traits: Vec<ModelTrait>) -> ModelProfile {
    ModelProfile::new(provider, model, traits)
}

// ── Exact trait wins ──────────────────────────────────────────────────

#[test]
fn exact_trait_wins() {
    let r = resolver(vec![
        p(
            "openai",
            "gpt-4o",
            vec![ModelTrait::General, ModelTrait::Vision],
        ),
        p("anthropic", "o3-mini", vec![ModelTrait::Reasoning]),
    ]);
    assert_eq!(r.resolve(ModelTrait::Reasoning).unwrap().model, "o3-mini");
}

#[test]
fn general_trait_falls_back_to_any() {
    let r = resolver(vec![p("openai", "gpt-4o", vec![ModelTrait::General])]);
    assert_eq!(r.resolve(ModelTrait::General).unwrap().model, "gpt-4o");
}

#[test]
fn general_falls_back_to_any_with_multiple() {
    let r = resolver(vec![
        p("openai", "gpt-4o", vec![ModelTrait::General]),
        p(
            "anthropic",
            "claude-3",
            vec![ModelTrait::Vision, ModelTrait::General],
        ),
    ]);
    // gpt-4o: tier 0 exact [General]; claude: tier 2 [General,Vision] requesting General.
    // Tier 0 < tier 2 → gpt-4o wins.
    assert_eq!(r.resolve(ModelTrait::General).unwrap().model, "gpt-4o");
}

// ── Partial match ──────────────────────────────────────────────────────

#[test]
fn most_matching_traits_wins_on_partial_match() {
    let r = resolver(vec![
        p("fast", "coder", vec![ModelTrait::General]),
        p("capable", "coder-plus", vec![ModelTrait::Reasoning]),
    ]);
    // Both have General; coder-plus has Reasoning (extra non-General trait).
    // When requesting General: coder=tier 0 (exact), coder-plus=tier 2 (partial).
    // Tier 0 wins. This test verifies exact General beats partial General.
    assert_eq!(r.resolve(ModelTrait::General).unwrap().model, "coder");

    // But when requesting Reasoning: both match, coder=tier 2, coder-plus=tier 1.
    // Tier 1 wins → coder-plus wins (more specialized for this request).
    assert_eq!(
        r.resolve(ModelTrait::Reasoning).unwrap().model,
        "coder-plus"
    );
}

// ── Priority breaks ties ─────────────────────────────────────────────

#[test]
fn priority_breaks_ties() {
    let r = resolver(vec![
        p("openai", "first", vec![ModelTrait::General]),
        p("anthropic", "second", vec![ModelTrait::General]),
    ])
    .with_priority(vec!["openai/first".into(), "anthropic/second".into()]);

    assert_eq!(r.resolve(ModelTrait::General).unwrap().model, "first");
}

#[test]
fn priority_with_reverse_order() {
    let r = resolver(vec![
        p("openai", "first", vec![ModelTrait::General]),
        p("anthropic", "second", vec![ModelTrait::General]),
    ])
    .with_priority(vec!["anthropic/second".into(), "openai/first".into()]);

    assert_eq!(r.resolve(ModelTrait::General).unwrap().model, "second");
}

// ── No match ──────────────────────────────────────────────────────────

#[test]
fn no_match_returns_error() {
    let r = resolver(vec![p("fast", "tiny", vec![ModelTrait::Fast])]);
    let err = r.resolve(ModelTrait::Vision).unwrap_err();
    assert!(matches!(
        err,
        ResolverError::NoMatch {
            trait_: ModelTrait::Vision
        }
    ));
}

#[test]
fn empty_resolver_returns_no_models() {
    let r: ModelResolver = resolver(Vec::new());
    let err = r.resolve(ModelTrait::General).unwrap_err();
    assert!(matches!(err, ResolverError::NoModelsConfigured));
}

// ── ModelProfile helpers ────────────────────────────────────────────

#[test]
fn profile_key() {
    let p = ModelProfile::new("openai", "gpt-4o", vec![ModelTrait::General]);
    assert_eq!(p.key(), "openai/gpt-4o");
}

#[test]
fn profile_has_trait() {
    let p = ModelProfile::new(
        "anthropic",
        "claude-3",
        vec![ModelTrait::Vision, ModelTrait::General],
    );
    assert!(p.has_trait(ModelTrait::Vision));
    assert!(p.has_trait(ModelTrait::General));
    assert!(!p.has_trait(ModelTrait::Reasoning));
}

#[test]
fn model_profile_from_info() {
    let info = ModelInfo {
        name: "gpt-4o".into(),
        provider: "openai".into(),
        display_name: "GPT-4o".into(),
        cost_prompt: None,
        cost_completion: None,
        supports_thinking: false,
        supports_vision: true,
        tokenizer: None,
        context_window: Some(128_000),
        capabilities: ModelCapabilities {
            streaming: true,
            supports_vision: true,
            supports_tools: true,
            supports_reasoning: false,
            max_context_tokens: 128_000,
            max_output_tokens: 16_384,
            cache_control: false,
        },
    };
    let profile = ModelProfile::from_info(&info);
    assert_eq!(profile.provider, "openai");
    assert_eq!(profile.model, "gpt-4o");
    assert!(profile.has_trait(ModelTrait::Vision));
    assert!(profile.has_trait(ModelTrait::Fast)); // supports_tools && !reasoning
    assert!(profile.has_trait(ModelTrait::General));
    assert!(!profile.has_trait(ModelTrait::Reasoning));
    assert!(!profile.has_trait(ModelTrait::LongContext)); // context_window <= 200k
}

#[test]
fn reasoning_model_derives_correct_traits() {
    let info = ModelInfo {
        name: "o3-mini".into(),
        provider: "anthropic".into(),
        display_name: "o3-mini".into(),
        cost_prompt: None,
        cost_completion: None,
        supports_thinking: true,
        supports_vision: false,
        tokenizer: None,
        context_window: Some(200_000),
        capabilities: ModelCapabilities {
            streaming: false,
            supports_vision: false,
            supports_tools: true,
            supports_reasoning: true,
            max_context_tokens: 200_000,
            max_output_tokens: 100_000,
            cache_control: true,
        },
    };
    let profile = ModelProfile::from_info(&info);
    assert!(profile.has_trait(ModelTrait::Reasoning));
    assert!(profile.has_trait(ModelTrait::LongContext)); // max_context_tokens > 200k
    assert!(profile.has_trait(ModelTrait::General));
    assert!(!profile.has_trait(ModelTrait::Fast)); // has reasoning → not fast
    assert!(!profile.has_trait(ModelTrait::Vision));
}

// ── Resolve all ─────────────────────────────────────────────────────

#[test]
fn resolve_all_deduplicates() {
    let r = resolver(vec![p(
        "anthropic",
        "claude",
        vec![ModelTrait::General, ModelTrait::Vision],
    )]);
    let results = r.resolve_all(&[ModelTrait::General, ModelTrait::Vision]);
    assert_eq!(results.len(), 2);
    assert!(results[0].is_ok());
    assert!(results[1].is_ok());
    assert_eq!(results[0].as_ref().unwrap().model, "claude");
    assert_eq!(results[1].as_ref().unwrap().model, "claude");
}

// ── Error display ────────────────────────────────────────────────────

#[test]
fn resolver_error_display() {
    let err = ResolverError::NoMatch {
        trait_: ModelTrait::Vision,
    };
    assert!(err.to_string().contains("vision"));
    assert!(err.to_string().contains("no model"));

    let err2 = ResolverError::NoModelsConfigured;
    assert!(err2.to_string().contains("no models"));
}
