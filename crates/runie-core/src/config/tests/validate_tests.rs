//! Tests for the hand-written config validator.

use super::*;
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
    let errors = validate::validate(&value);
    assert!(errors.is_empty(), "valid config should have no errors: {:?}", errors);
}

#[test]
fn invalid_provider_type_fails_validation() {
    let value = json!({
        "provider": 123
    });
    let errors = validate::validate(&value);
    assert!(!errors.is_empty(), "provider as integer should fail");
    assert!(errors.iter().any(|e| e.contains("provider") && e.contains("string")));
}

#[test]
fn invalid_model_type_fails_validation() {
    let value = json!({
        "model": ["not", "a", "string"]
    });
    let errors = validate::validate(&value);
    assert!(!errors.is_empty(), "model as array should fail");
}

#[test]
fn invalid_ui_type_fails_validation() {
    let value = json!({
        "ui": "not an object"
    });
    let errors = validate::validate(&value);
    assert!(!errors.is_empty(), "ui as string should fail");
}

#[test]
fn invalid_vim_mode_type_fails_validation() {
    let value = json!({
        "ui": { "vim_mode": "yes" }
    });
    let errors = validate::validate(&value);
    assert!(!errors.is_empty(), "vim_mode as string should fail");
}

#[test]
fn invalid_truncation_max_lines_fails_validation() {
    let value = json!({
        "truncation": { "max_lines": "many" }
    });
    let errors = validate::validate(&value);
    assert!(!errors.is_empty(), "max_lines as string should fail");
}

#[test]
fn invalid_telemetry_enabled_fails_validation() {
    let value = json!({
        "telemetry": { "enabled": "yes" }
    });
    let errors = validate::validate(&value);
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
    let errors = validate::validate(&value);
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
    let errors = validate::validate(&value);
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
    let errors = validate::validate(&value);
    assert!(!errors.is_empty(), "models array item as integer should fail");
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
    let errors = validate::validate(&value);
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
    let errors = validate::validate(&value);
    assert!(!errors.is_empty(), "hook command item as integer should fail");
}

#[test]
fn unknown_field_produces_warning() {
    let value = json!({
        "provider": "openai",
        "unknown_field": "test"
    });
    let errors = validate::validate(&value);
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
    let errors = validate::validate(&value);
    // Explicitly-null non-nullable fields produce validation errors.
    // provider/model/theme are Option<T> so they tolerate null; the rest do not.
    assert!(!errors.is_empty(), "non-nullable null fields should fail");
    // Errors should be about type mismatch, not unknown fields.
    assert!(errors.iter().all(|e| e.contains("not of type") || e.contains(": unknown")),
        "errors should be type errors or unknown fields: {:?}", errors);
}

#[test]
fn empty_object_is_valid() {
    let value = json!({});
    let errors = validate::validate(&value);
    assert!(errors.is_empty(), "empty object should be valid: {:?}", errors);
}
