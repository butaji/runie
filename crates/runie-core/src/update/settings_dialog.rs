//! Settings dialog item builder.
//!
//! Settings rows are rendered as a generic panel and mutated through
//! `dialog_panel::apply_panel_setting`, not through this module.

use crate::model::{AppState, DeliveryMode};
use crate::settings::{SettingItem, SettingValue, SettingsCategory};

pub fn handle_settings_category(state: &mut AppState, _category: SettingsCategory) {
    state.mark_dirty();
}

pub fn build_setting_items(state: &AppState) -> Vec<SettingItem> {
    vec![
        SettingItem::new(
            "provider",
            "Provider",
            SettingValue::Enum {
                current: state.config.current_provider.clone(),
                options: provider_options(),
            },
            "LLM provider",
            SettingsCategory::Models,
        ),
        SettingItem::new(
            "model",
            "Model",
            SettingValue::Enum {
                current: state.config.current_model.clone(),
                options: model_options(&state.config.current_provider),
            },
            "Active model",
            SettingsCategory::Models,
        ),
        SettingItem::new(
            "theme",
            "Theme",
            SettingValue::Enum {
                current: state.config.theme_name.clone(),
                options: theme_options(),
            },
            "UI theme",
            SettingsCategory::Appearance,
        ),
        SettingItem::new(
            "thinking_level",
            "Thinking Level",
            SettingValue::Enum {
                current: state.config.thinking_level.as_str().to_string(),
                options: vec!["off".into(), "low".into(), "medium".into(), "high".into()],
            },
            "Agent reasoning depth",
            SettingsCategory::Behavior,
        ),
        SettingItem::new(
            "read_only",
            "Read-Only",
            SettingValue::Bool(state.config.read_only),
            "Restrict to safe tools",
            SettingsCategory::Safety,
        ),
        SettingItem::new(
            "steering_mode",
            "Steering Mode",
            SettingValue::Enum {
                current: match state.config.steering_mode {
                    DeliveryMode::OneAtATime => "one-at-a-time",
                    DeliveryMode::All => "all",
                }
                .to_string(),
                options: vec!["one-at-a-time".into(), "all".into()],
            },
            "How steering messages are delivered",
            SettingsCategory::Behavior,
        ),
        SettingItem::new(
            "follow_up_mode",
            "Follow-Up Mode",
            SettingValue::Enum {
                current: match state.config.follow_up_mode {
                    DeliveryMode::OneAtATime => "one-at-a-time",
                    DeliveryMode::All => "all",
                }
                .to_string(),
                options: vec!["one-at-a-time".into(), "all".into()],
            },
            "How follow-up messages are delivered",
            SettingsCategory::Behavior,
        ),
        SettingItem::new(
            "vim_mode",
            "Vim Navigation",
            SettingValue::Bool(state.config.vim_mode),
            "Press Esc from the input box to navigate the feed with j/k/g/G",
            SettingsCategory::Behavior,
        ),
        SettingItem::new(
            "telemetry_enabled",
            "Telemetry",
            SettingValue::Bool(state.config.telemetry.is_enabled()),
            "Anonymous usage analytics",
            SettingsCategory::Safety,
        ),
        SettingItem::new(
            "truncation_max_lines",
            "Truncation Max Lines",
            SettingValue::Enum {
                current: state.config.truncation.max_lines.to_string(),
                options: truncation_lines_options(),
            },
            "Max lines kept from a single tool output",
            SettingsCategory::Behavior,
        ),
        SettingItem::new(
            "truncation_max_bytes",
            "Truncation Max Bytes",
            SettingValue::Enum {
                current: state.config.truncation.max_bytes.to_string(),
                options: truncation_bytes_options(),
            },
            "Max bytes kept from a single tool output",
            SettingsCategory::Behavior,
        ),
    ]
}

fn provider_options() -> Vec<String> {
    crate::provider_registry::known_providers()
        .iter()
        .map(|p| p.key.to_string())
        .collect()
}

fn model_options(provider: &str) -> Vec<String> {
    crate::model_catalog::model_catalog()
        .iter()
        .filter(|m| m.provider == provider)
        .map(|m| m.name.clone())
        .collect()
}

fn theme_options() -> Vec<String> {
    crate::themes::BUILTIN_THEMES
        .iter()
        .map(|t| t.to_string())
        .collect()
}

fn truncation_lines_options() -> Vec<String> {
    vec!["1000".into(), "2000".into(), "5000".into(), "10000".into()]
}

fn truncation_bytes_options() -> Vec<String> {
    vec![
        "10240".into(),  // 10 KiB
        "51200".into(),  // 50 KiB (default)
        "102400".into(), // 100 KiB
        "512000".into(), // 500 KiB
    ]
}
