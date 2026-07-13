//! Tests for the config validator (moved from validate.rs).

use super::*;
use crate::config::config_impl::{validate, validate_registry};
use serde_json::json;

#[test]
fn valid_config_passes_validation() {
    let value = json!({
        "provider": "openai",
        "model": "gpt-4",
        "theme": "nord",
        "ui": { "vim_mode": true },
        "models": { "default": "gpt-4o" },
        "model_providers": {
            "openai": {
                "type": "openai",
                "base_url": "https://api.openai.com",
                "api_key": "sk-test"
            }
        },
        "telemetry": { "enabled": false },
        "prompts": { "default": "default" },
        "truncation": { "max_lines": 100, "max_bytes": 50000 }
    });
    let errors = validate(&value);
    assert!(
        errors.is_empty(),
        "valid config should have no errors: {:?}",
        errors
    );
}

#[test]
fn invalid_provider_type_fails_validation() {
    let value = json!({
        "provider": 123
    });
    let errors = validate(&value);
    assert!(!errors.is_empty(), "provider as integer should fail");
    assert!(errors
        .iter()
        .any(|e| e.contains("provider") && e.contains("string")));
}

#[test]
fn invalid_model_type_fails_validation() {
    let value = json!({
        "model": ["not", "a", "string"]
    });
    let errors = validate(&value);
    assert!(!errors.is_empty(), "model as array should fail");
}

#[test]
fn invalid_ui_type_fails_validation() {
    let value = json!({
        "ui": "not an object"
    });
    let errors = validate(&value);
    assert!(!errors.is_empty(), "ui as string should fail");
}

#[test]
fn invalid_vim_mode_type_fails_validation() {
    let value = json!({
        "ui": { "vim_mode": "yes" }
    });
    let errors = validate(&value);
    assert!(!errors.is_empty(), "vim_mode as string should fail");
}

#[test]
fn invalid_truncation_max_lines_fails_validation() {
    let value = json!({
        "truncation": { "max_lines": "many" }
    });
    let errors = validate(&value);
    assert!(!errors.is_empty(), "max_lines as string should fail");
}

#[test]
fn invalid_telemetry_enabled_fails_validation() {
    let value = json!({
        "telemetry": { "enabled": "yes" }
    });
    let errors = validate(&value);
    assert!(!errors.is_empty(), "enabled as string should fail");
}

#[test]
fn invalid_provider_base_url_fails_validation() {
    let value = json!({
        "model_providers": {
            "test": {
                "base_url": 123
            }
        }
    });
    let errors = validate(&value);
    assert!(!errors.is_empty(), "base_url as integer should fail");
}

#[test]
fn invalid_provider_api_key_fails_validation() {
    let value = json!({
        "model_providers": {
            "test": {
                "api_key": 123
            }
        }
    });
    let errors = validate(&value);
    assert!(!errors.is_empty(), "api_key as integer should fail");
}

#[test]
fn invalid_provider_models_item_fails_validation() {
    let value = json!({
        "model_providers": {
            "test": {
                "models": [123, "valid"]
            }
        }
    });
    let errors = validate(&value);
    assert!(
        !errors.is_empty(),
        "models array item as integer should fail"
    );
}

#[test]
fn invalid_hooks_command_fails_validation() {
    let value = json!({
        "hooks": {
            "commands": {
                "on_tool": 123
            }
        }
    });
    let errors = validate(&value);
    assert!(!errors.is_empty(), "hook command as integer should fail");
}

#[test]
fn invalid_hook_command_item_fails_validation() {
    let value = json!({
        "hooks": {
            "commands": {
                "on_tool": [123]
            }
        }
    });
    let errors = validate(&value);
    assert!(
        !errors.is_empty(),
        "hook command item as integer should fail"
    );
}

#[test]
fn unknown_field_produces_warning() {
    let value = json!({
        "provider": "openai",
        "unknown_field": "test"
    });
    let errors = validate(&value);
    assert!(!errors.is_empty(), "unknown field should produce warning");
    assert!(errors.iter().any(|e| e.contains("unknown field")));
}

#[test]
fn null_values_are_strictly_rejected_for_non_nullable_fields() {
    // Fields with serde defaults (models, telemetry, ui, truncation, prompts,
    // hooks, model_providers) are not nullable in the schema; null is only valid
    // for explicitly Option<T> fields (provider, model, theme).
    // This is a stricter-than-original behavior: the hand-written validator
    // ignored null for defaulted fields; jsonschema enforces the schema.
    let value = json!({
        "provider": null,
        "model": null,
        "theme": null,
        "ui": null,
        "models": null,
        "model_providers": null,
        "telemetry": null,
        "prompts": null,
        "truncation": null,
        "hooks": null
    });
    let errors = validate(&value);
    // Explicitly-null non-nullable fields produce validation errors.
    // provider/model/theme are Option<T> so they tolerate null; the rest do not.
    assert!(!errors.is_empty(), "non-nullable null fields should fail");
    // Errors should be about type mismatch, not unknown fields.
    assert!(
        errors
            .iter()
            .all(|e| e.contains("not of type") || e.contains(": unknown")),
        "errors should be type errors or unknown fields: {:?}",
        errors
    );
}

#[test]
fn empty_object_is_valid() {
    let value = json!({});
    let errors = validate(&value);
    assert!(
        errors.is_empty(),
        "empty object should be valid: {:?}",
        errors
    );
}

// ============================================================================
// Registry-based validation tests
// ============================================================================

#[test]
fn registry_validation_accepts_known_provider_and_model() {
    let config = Config {
        provider: Some("openai".to_string()),
        models: crate::config::ModelsSection {
            default: Some("gpt-4o".to_string()),
            scoped: None,
            thinking: Default::default(),
        },
        ..Config::default()
    };
    let errors = validate_registry(&config);
    assert!(
        errors.is_empty(),
        "known provider/model should pass: {:?}",
        errors
    );
}

#[test]
fn registry_validation_rejects_unknown_provider() {
    let config = Config {
        provider: Some("fake-provider".to_string()),
        ..Config::default()
    };
    let errors = validate_registry(&config);
    assert!(!errors.is_empty(), "unknown provider should fail");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("fake-provider") && e.contains("unknown provider")),
        "error should mention unknown provider: {:?}",
        errors
    );
}

#[test]
fn registry_validation_accepts_unlisted_model_for_known_provider() {
    // A known provider with a default model the bundled registry does not list
    // must be accepted: the registry model list is advisory, not exhaustive.
    let config = Config {
        provider: Some("openai".to_string()),
        models: crate::config::ModelsSection {
            default: Some("nonexistent-model".to_string()),
            scoped: None,
            thinking: Default::default(),
        },
        ..Config::default()
    };
    let errors = validate_registry(&config);
    assert!(
        errors.is_empty(),
        "unlisted model for a known provider should be accepted: {:?}",
        errors
    );
}

#[test]
fn registry_validation_rejects_wrong_provider_prefix() {
    // Model name with provider prefix that doesn't match
    let config = Config {
        provider: Some("openai".to_string()),
        models: crate::config::ModelsSection {
            default: Some("anthropic/claude-3".to_string()),
            scoped: None,
            thinking: Default::default(),
        },
        ..Config::default()
    };
    let errors = validate_registry(&config);
    assert!(!errors.is_empty(), "mismatched provider prefix should fail");
}

#[test]
fn registry_validation_rejects_unknown_configured_provider() {
    let mut config = Config::default();
    config.model_providers.insert(
        "fake-provider".to_string(),
        crate::config::ModelProvider {
            provider_type: Some("fake".to_string()),
            base_url: "https://fake.example.com".to_string(),
            models: vec![],
        },
    );
    let errors = validate_registry(&config);
    assert!(
        !errors.is_empty(),
        "unknown configured provider should fail"
    );
    assert!(
        errors
            .iter()
            .any(|e| e.contains("fake-provider") && e.contains("unknown provider")),
        "error should mention unknown provider: {:?}",
        errors
    );
}

#[test]
fn registry_validation_accepts_minimax_provider() {
    let config = Config {
        provider: Some("minimax".to_string()),
        models: crate::config::ModelsSection {
            default: Some("MiniMax-M3".to_string()),
            scoped: None,
            thinking: Default::default(),
        },
        ..Config::default()
    };
    let errors = validate_registry(&config);
    assert!(errors.is_empty(), "minimax should be valid: {:?}", errors);
}

#[test]
fn registry_validation_accepts_full_model_format() {
    let config = Config {
        provider: Some("openai".to_string()),
        models: crate::config::ModelsSection {
            default: Some("openai/gpt-4o".to_string()),
            scoped: None,
            thinking: Default::default(),
        },
        ..Config::default()
    };
    let errors = validate_registry(&config);
    assert!(
        errors.is_empty(),
        "full model format should pass: {:?}",
        errors
    );
}

#[test]
fn registry_validation_accepts_minimax_prefixed_model() {
    // Regression: "minimax/MiniMax-M3" was rejected because find_model only
    // matched bare model names or coincidental openrouter prefixed names.
    let config = Config {
        provider: Some("minimax".to_string()),
        models: crate::config::ModelsSection {
            default: Some("minimax/MiniMax-M3".to_string()),
            scoped: Some(vec!["minimax/MiniMax-M3".to_string()]),
            thinking: Default::default(),
        },
        ..Config::default()
    };
    let errors = validate_registry(&config);
    assert!(
        errors.is_empty(),
        "minimax prefixed model should pass: {:?}",
        errors
    );
}

#[test]
fn registry_validation_accepts_mock_prefixed_model_when_mock_enabled() {
    crate::provider::registry::set_mock_enabled(true);
    let config = Config {
        provider: Some("mock".to_string()),
        models: crate::config::ModelsSection {
            default: Some("mock/echo".to_string()),
            scoped: Some(vec!["mock/echo".to_string()]),
            thinking: Default::default(),
        },
        ..Config::default()
    };
    let errors = validate_registry(&config);
    crate::provider::registry::set_mock_enabled(false);
    assert!(
        errors.is_empty(),
        "mock prefixed model should pass when mock enabled: {:?}",
        errors
    );
}

#[test]
fn registry_validation_accepts_unlisted_prefixed_model_for_known_provider() {
    // Correctly-prefixed but unlisted model for a known provider is accepted.
    let config = Config {
        provider: Some("minimax".to_string()),
        models: crate::config::ModelsSection {
            default: Some("minimax/not-a-real-model".to_string()),
            scoped: None,
            thinking: Default::default(),
        },
        ..Config::default()
    };
    let errors = validate_registry(&config);
    assert!(
        errors.is_empty(),
        "unlisted prefixed model for a known provider should be accepted: {:?}",
        errors
    );
}

#[test]
fn registry_validation_accepts_minimax_highspeed_default_model() {
    // Regression: "MiniMax-M2.7-highspeed" is a real upstream model (returned by
    // the API) that the bundled registry did not list; it must not fail config
    // validation and block model switching / reloads.
    let config = Config {
        provider: Some("minimax".to_string()),
        models: crate::config::ModelsSection {
            default: Some("MiniMax-M2.7-highspeed".to_string()),
            scoped: None,
            thinking: Default::default(),
        },
        ..Config::default()
    };
    let errors = validate_registry(&config);
    assert!(
        errors.is_empty(),
        "MiniMax-M2.7-highspeed default should be accepted: {:?}",
        errors
    );
}

#[test]
fn config_validate_registry_method() {
    let config = Config {
        provider: Some("openai".to_string()),
        ..Config::default()
    };
    assert!(config.validate_registry().is_ok());

    let bad_config = Config {
        provider: Some("nonexistent".to_string()),
        ..Config::default()
    };
    assert!(bad_config.validate_registry().is_err());
}

#[test]
fn config_validate_full_method() {
    // Good config should pass full validation
    let config = Config {
        provider: Some("openai".to_string()),
        ..Config::default()
    };
    assert!(config.validate_full().is_ok());

    // Unknown provider should fail full validation
    let bad_config = Config {
        provider: Some("nonexistent".to_string()),
        ..Config::default()
    };
    let result = bad_config.validate_full();
    assert!(result.is_err());
}
