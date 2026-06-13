use crate::config::{Config, ModelProvider};
use runie_core::ProviderError;
use std::collections::HashMap;

#[test]
fn config_defaults_empty() {
    let cfg = Config::default();
    assert!(cfg.default_model().is_none());
    assert!(cfg.model_providers.is_empty());
}

#[test]
fn config_parses_legacy_fields() {
    let toml = r#"
provider = "openai"
model = "gpt-4o"
"#;
    let cfg: Config = toml::from_str(toml).unwrap();
    assert_eq!(cfg.provider, Some("openai".to_string()));
    assert_eq!(cfg.model, Some("gpt-4o".to_string()));
}

#[test]
fn config_parses_model_providers() {
    let toml = r#"
[models]
default = "glm-4.7"

[model_providers.openrouter]
base_url = "https://openrouter.ai/api/v1"
api_key = "sk-or-..."

[model_providers.local]
base_url = "http://localhost:11434/v1"
api_key = "ollama"
"#;
    let cfg: Config = toml::from_str(toml).unwrap();
    assert_eq!(cfg.models.default, Some("glm-4.7".to_string()));
    assert_eq!(cfg.model_providers.len(), 2);

    let openrouter = cfg.model_providers.get("openrouter").unwrap();
    assert_eq!(openrouter.base_url, "https://openrouter.ai/api/v1");
    assert_eq!(openrouter.api_key, "sk-or-...");

    let local = cfg.model_providers.get("local").unwrap();
    assert_eq!(local.base_url, "http://localhost:11434/v1");
    assert_eq!(local.api_key, "ollama");
}

#[test]
fn config_round_trip() {
    let mut cfg = Config::default();
    cfg.models.default = Some("glm-4.7".to_string());
    let mut providers = HashMap::new();
    providers.insert(
        "local".to_string(),
        ModelProvider {
            provider_type: None,
            base_url: "http://localhost:11434/v1".to_string(),
            api_key: "ollama".to_string(),
        },
    );
    cfg.model_providers = providers;

    let serialized = toml::to_string_pretty(&cfg).unwrap();
    let parsed: Config = toml::from_str(&serialized).unwrap();
    assert_eq!(parsed.models.default, Some("glm-4.7".to_string()));
    assert_eq!(
        parsed.model_providers.get("local").unwrap().base_url,
        "http://localhost:11434/v1"
    );
}

#[test]
fn config_get_provider_uses_model_prefix() {
    let toml = r#"
[models]
default = "openrouter/anthropic/claude-sonnet-4-6"

[model_providers.openrouter]
base_url = "https://openrouter.ai/api/v1"
api_key = "sk-or-..."
"#;
    let cfg: Config = toml::from_str(toml).unwrap();
    let provider = cfg
        .provider_for_model("openrouter/anthropic/claude-sonnet-4-6")
        .unwrap();
    assert_eq!(provider.base_url, "https://openrouter.ai/api/v1");
    assert_eq!(provider.api_key, "sk-or-...");
}

#[test]
fn config_provider_type_determines_api() {
    let toml = r#"
[model_providers.custom]
type = "openai-compatible"
base_url = "http://localhost:8080/v1"
api_key = "dummy"
"#;
    let cfg: Config = toml::from_str(toml).unwrap();
    let provider = cfg.model_providers.get("custom").unwrap();
    assert_eq!(provider.provider_type.as_deref(), Some("openai-compatible"));
}

#[test]
fn dyn_provider_from_registry_key() {
    // openai requires OPENAI_API_KEY; without it and without RUNIE_MOCK, we get MissingApiKey.
    // Save and restore so the test is environment-independent.
    let saved_key = std::env::var("OPENAI_API_KEY").ok();
    let saved_mock = std::env::var("RUNIE_MOCK").ok();
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("RUNIE_MOCK");
    let result = crate::build_provider_with_warning("openai", "gpt-4o");
    if let Some(v) = saved_key {
        std::env::set_var("OPENAI_API_KEY", v);
    }
    if let Some(v) = saved_mock {
        std::env::set_var("RUNIE_MOCK", v);
    }
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ProviderError::MissingApiKey(_)));
}

#[test]
fn dyn_provider_unknown_key_returns_error() {
    let result = crate::build_provider_with_warning("nonexistent-provider", "model-x");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ProviderError::UnknownProvider(k) if k == "nonexistent-provider"));
}
