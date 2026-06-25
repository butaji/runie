//! Hand-written validator for config JSON values.
//!
//! Checks that required top-level fields have correct types.
//! This replaces the jsonschema-based validation to remove the jsonschema dependency.

use serde_json::Value;

/// Validate a JSON value against the expected config schema.
/// Returns a list of error messages.
pub fn validate(value: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    validate_object(value, &[
        "provider", "model", "theme", "ui", "models",
        "model_providers", "telemetry", "prompts", "truncation",
        "keybindings", "hooks",
    ], "", &mut errors);
    errors
}

fn validate_object(value: &Value, allowed_keys: &[&str], prefix: &str, errors: &mut Vec<String>) {
    let Value::Object(map) = value else {
        errors.push(format!("{}expected object, got {}", prefix, value));
        return;
    };

    for key in map.keys() {
        if !allowed_keys.contains(&key.as_str()) {
            let suffix = if prefix.is_empty() { " (ignored)" } else { "" };
            errors.push(format!("{}: unknown field: {}{}", prefix, key, suffix));
        }
    }

    validate_provider(map, prefix, errors);
    validate_string_field(map, "model", prefix, errors);
    validate_string_field(map, "theme", prefix, errors);
    validate_ui(map, prefix, errors);
    validate_models(map, prefix, errors);
    validate_providers(map, prefix, errors);
    validate_telemetry(map, prefix, errors);
    validate_prompts(map, prefix, errors);
    validate_truncation(map, prefix, errors);
    validate_keybindings(map, prefix, errors);
    validate_hooks(map, prefix, errors);
}

fn validate_provider(map: &serde_json::Map<String, Value>, prefix: &str, errors: &mut Vec<String>) {
    if let Some(v) = map.get("provider") {
        if !v.is_null() && !v.is_string() {
            errors.push(format!("{}provider must be a string", prefix));
        }
    }
}

fn validate_string_field(map: &serde_json::Map<String, Value>, key: &str, prefix: &str, errors: &mut Vec<String>) {
    if let Some(v) = map.get(key) {
        if !v.is_null() && !v.is_string() {
            errors.push(format!("{}{} must be a string", prefix, key));
        }
    }
}

fn validate_ui(map: &serde_json::Map<String, Value>, prefix: &str, errors: &mut Vec<String>) {
    let Some(ui_val) = map.get("ui") else { return };
    if let Value::Object(ui_map) = ui_val {
        if let Some(v) = ui_map.get("vim_mode") {
            if !v.is_boolean() {
                errors.push(format!("{}ui.vim_mode must be a boolean", prefix));
            }
        }
    } else if !ui_val.is_null() {
        errors.push(format!("{}ui must be an object", prefix));
    }
}

fn validate_models(map: &serde_json::Map<String, Value>, prefix: &str, errors: &mut Vec<String>) {
    let Some(models_val) = map.get("models") else { return };
    if let Value::Object(models_map) = models_val {
        validate_string_field(models_map, "default", prefix, errors);
        if let Some(v) = models_map.get("scoped") {
            if let Some(arr) = v.as_array() {
                for (i, item) in arr.iter().enumerate() {
                    if !item.is_string() {
                        errors.push(format!("{}models.scoped[{}] must be a string", prefix, i));
                    }
                }
            } else if !v.is_null() {
                errors.push(format!("{}models.scoped must be an array", prefix));
            }
        }
    } else if !models_val.is_null() {
        errors.push(format!("{}models must be an object", prefix));
    }
}

fn validate_providers(map: &serde_json::Map<String, Value>, prefix: &str, errors: &mut Vec<String>) {
    let Some(providers_val) = map.get("model_providers") else { return };
    if let Value::Object(providers_map) = providers_val {
        for (name, provider_val) in providers_map {
            let pfx = format!("{}model_providers.{}: ", prefix, name);
            validate_single_provider(provider_val, &pfx, errors);
        }
    } else if !providers_val.is_null() {
        errors.push(format!("{}model_providers must be an object", prefix));
    }
}

fn validate_single_provider(provider_val: &Value, prefix: &str, errors: &mut Vec<String>) {
    let Value::Object(provider_map) = provider_val else {
        errors.push(format!("{}must be an object", prefix));
        return;
    };
    validate_string_field(provider_map, "type", prefix, errors);
    validate_string_field(provider_map, "base_url", prefix, errors);
    validate_string_field(provider_map, "api_key", prefix, errors);
    validate_provider_models(provider_map, prefix, errors);
}

fn validate_provider_models(map: &serde_json::Map<String, Value>, prefix: &str, errors: &mut Vec<String>) {
    let Some(v) = map.get("models") else { return };
    match v {
        Value::Array(arr) => {
            for (i, item) in arr.iter().enumerate() {
                if !item.is_string() {
                    errors.push(format!("{}models[{}] must be a string", prefix, i));
                }
            }
        }
        Value::Null => {}
        _ => errors.push(format!("{}models must be an array", prefix)),
    }
}

fn validate_telemetry(map: &serde_json::Map<String, Value>, prefix: &str, errors: &mut Vec<String>) {
    let Some(telemetry_val) = map.get("telemetry") else { return };
    if let Value::Object(telemetry_map) = telemetry_val {
        if let Some(v) = telemetry_map.get("enabled") {
            if !v.is_boolean() {
                errors.push(format!("{}telemetry.enabled must be a boolean", prefix));
            }
        }
    } else if !telemetry_val.is_null() {
        errors.push(format!("{}telemetry must be an object", prefix));
    }
}

fn validate_prompts(map: &serde_json::Map<String, Value>, prefix: &str, errors: &mut Vec<String>) {
    let Some(prompts_val) = map.get("prompts") else { return };
    if let Value::Object(prompts_map) = prompts_val {
        validate_string_field(prompts_map, "default", prefix, errors);
        validate_string_field(prompts_map, "custom", prefix, errors);
    } else if !prompts_val.is_null() {
        errors.push(format!("{}prompts must be an object", prefix));
    }
}

fn validate_truncation(map: &serde_json::Map<String, Value>, prefix: &str, errors: &mut Vec<String>) {
    let Some(trunc_val) = map.get("truncation") else { return };
    if let Value::Object(trunc_map) = trunc_val {
        if let Some(v) = trunc_map.get("max_lines") {
            if !v.is_u64() {
                errors.push(format!("{}truncation.max_lines must be an integer", prefix));
            }
        }
        if let Some(v) = trunc_map.get("max_bytes") {
            if !v.is_u64() {
                errors.push(format!("{}truncation.max_bytes must be an integer", prefix));
            }
        }
    } else if !trunc_val.is_null() {
        errors.push(format!("{}truncation must be an object", prefix));
    }
}

fn validate_keybindings(map: &serde_json::Map<String, Value>, prefix: &str, errors: &mut Vec<String>) {
    let Some(kb_val) = map.get("keybindings") else { return };
    if let Value::Object(kb_map) = kb_val {
        for (key, val) in kb_map {
            if !val.is_string() {
                errors.push(format!("{}keybindings.{} must be a string", prefix, key));
            }
        }
    } else if !kb_val.is_null() {
        errors.push(format!("{}keybindings must be an object", prefix));
    }
}

fn validate_hooks(map: &serde_json::Map<String, Value>, prefix: &str, errors: &mut Vec<String>) {
    let Some(hooks_val) = map.get("hooks") else { return };
    if let Value::Object(hooks_map) = hooks_val {
        validate_hook_commands(hooks_map, prefix, errors);
    } else if !hooks_val.is_null() {
        errors.push(format!("{}hooks must be an object", prefix));
    }
}

fn validate_hook_commands(map: &serde_json::Map<String, Value>, prefix: &str, errors: &mut Vec<String>) {
    let Some(commands_val) = map.get("commands") else { return };
    let Value::Object(commands_map) = commands_val else {
        errors.push(format!("{}hooks.commands must be an object", prefix));
        return;
    };
    for (name, val) in commands_map {
        validate_hook_command_list(val, name, prefix, errors);
    }
}

fn validate_hook_command_list(val: &Value, name: &str, prefix: &str, errors: &mut Vec<String>) {
    match val {
        Value::Array(arr) => {
            for (i, item) in arr.iter().enumerate() {
                if !item.is_string() {
                    errors.push(format!("{}hooks.commands.{}[{}] must be a string", prefix, name, i));
                }
            }
        }
        Value::Null => {}
        _ => errors.push(format!("{}hooks.commands.{} must be an array", prefix, name)),
    }
}
