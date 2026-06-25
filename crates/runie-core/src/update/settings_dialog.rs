//! Settings dialog item builder.
//!
//! Settings rows are rendered as a generic panel and mutated through
//! `dialog_panel::apply_panel_setting`, not through this module.

use crate::dialog::{ItemAction, PanelItem};
use crate::model::{AppState, DeliveryMode};
use crate::settings::{SettingItem, SettingValue, SettingsCategory};

pub fn handle_settings_category(state: &mut AppState, _category: SettingsCategory) {
    state.view_mut().dirty = true;
}

/// Build settings items grouped by category, already mapped to panel items.
pub fn build_setting_categories(state: &AppState) -> Vec<(SettingsCategory, Vec<PanelItem>)> {
    let items = build_setting_items(state);
    let mut categories: Vec<(SettingsCategory, Vec<PanelItem>)> = Vec::new();
    for item in items {
        let panel_items = setting_value_to_panel_items(&item.key, &item.label, &item.value);
        if let Some(last) = categories.last_mut() {
            if last.0 == item.category {
                last.1.extend(panel_items);
                continue;
            }
        }
        categories.push((item.category, panel_items));
    }
    categories
}

fn setting_value_to_panel_items(
    key: &str,
    label: &str,
    value: &SettingValue,
) -> Vec<PanelItem> {
    match value {
        SettingValue::Bool(v) => vec![PanelItem::Toggle {
            label: label.into(),
            value: *v,
            action: ItemAction::Toggle(key.into()),
        }],
        SettingValue::Cycle { current, options } => vec![PanelItem::Select {
            label: label.into(),
            current: current.clone(),
            options: options.clone(),
            key: key.into(),
        }],
        SettingValue::Action(evt) => vec![PanelItem::Action {
            label: label.into(),
            action: ItemAction::Emit(evt.clone()),
        }],
        SettingValue::MultiSelect { current, options } => {
            let mut items = vec![PanelItem::Header(format!("{} models", label))];
            let selected: std::collections::HashSet<String> =
                current.iter().cloned().collect();
            for option in options {
                let toggle_key = format!("edit_provider:{}:{}", key, option);
                items.push(PanelItem::Toggle {
                    label: option.clone(),
                    value: selected.contains(option),
                    action: ItemAction::Toggle(toggle_key),
                });
            }
            items
        }
    }
}

pub fn build_setting_items(state: &AppState) -> Vec<SettingItem> {
    vec![
        provider_item(state),
        model_item(state),
        provider_models_item(state),
        theme_item(state),
        thinking_level_item(state),
        read_only_item(state),
        steering_mode_item(state),
        follow_up_mode_item(state),
        vim_mode_item(state),
        telemetry_item(state),
        truncation_max_lines_item(state),
        truncation_max_bytes_item(state),
    ]
}

fn provider_item(state: &AppState) -> SettingItem {
    SettingItem::new(
        "provider",
        "Provider",
        SettingValue::Cycle {
            current: state.config().current_provider.clone(),
            options: provider_options(state),
        },
        "LLM provider",
        SettingsCategory::Models,
    )
}

fn model_item(state: &AppState) -> SettingItem {
    SettingItem::new(
        "model",
        "Model",
        SettingValue::Cycle {
            current: state.config().current_model.clone(),
            options: model_options(state, &state.config().current_provider),
        },
        "Active model",
        SettingsCategory::Models,
    )
}

fn provider_models_item(state: &AppState) -> SettingItem {
    let provider = state.config().current_provider.clone();
    let (saved, available) = provider_model_lists(state, &provider);
    SettingItem::new(
        &provider,
        "Provider Models",
        SettingValue::MultiSelect {
            current: saved,
            options: available,
        },
        "Enabled models for the current provider",
        SettingsCategory::Models,
    )
}

pub(crate) fn provider_model_lists(state: &AppState, provider: &str) -> (Vec<String>, Vec<String>) {
    let saved = state
        .config_cache
        .as_ref()
        .map(|c| c.models_for_provider(provider))
        .unwrap_or_default();
    let mut available = saved.clone();
    if let Some(meta) = crate::provider::find_provider(provider) {
        for model in meta.models {
            let name = model.name.to_string();
            if !available.contains(&name) {
                available.push(name);
            }
        }
    }
    available.sort();
    (saved, available)
}

fn theme_item(state: &AppState) -> SettingItem {
    SettingItem::new(
        "theme",
        "Theme",
        SettingValue::Cycle {
            current: state.config().theme_name.clone(),
            options: theme_options(),
        },
        "UI theme",
        SettingsCategory::Appearance,
    )
}

fn thinking_level_item(state: &AppState) -> SettingItem {
    SettingItem::new(
        "thinking_level",
        "Thinking Level",
        SettingValue::Cycle {
            current: state.config().thinking_level.as_str().to_string(),
            options: vec!["off".into(), "low".into(), "medium".into(), "high".into()],
        },
        "Agent reasoning depth",
        SettingsCategory::Behavior,
    )
}

fn read_only_item(state: &AppState) -> SettingItem {
    SettingItem::new(
        "read_only",
        "Read-Only",
        SettingValue::Bool(state.config().read_only),
        "Restrict to safe tools",
        SettingsCategory::Safety,
    )
}

fn steering_mode_item(state: &AppState) -> SettingItem {
    SettingItem::new(
        "steering_mode",
        "Steering Mode",
        SettingValue::Cycle {
            current: delivery_mode_str(state.config().steering_mode).to_string(),
            options: vec!["one-at-a-time".into(), "all".into()],
        },
        "How steering messages are delivered",
        SettingsCategory::Behavior,
    )
}

fn follow_up_mode_item(state: &AppState) -> SettingItem {
    SettingItem::new(
        "follow_up_mode",
        "Follow-Up Mode",
        SettingValue::Cycle {
            current: delivery_mode_str(state.config().follow_up_mode).to_string(),
            options: vec!["one-at-a-time".into(), "all".into()],
        },
        "How follow-up messages are delivered",
        SettingsCategory::Behavior,
    )
}

fn delivery_mode_str(mode: DeliveryMode) -> &'static str {
    match mode {
        DeliveryMode::OneAtATime => "one-at-a-time",
        DeliveryMode::All => "all",
    }
}

fn vim_mode_item(state: &AppState) -> SettingItem {
    SettingItem::new(
        "vim_mode",
        "Vim Navigation",
        SettingValue::Bool(state.config().vim_mode),
        "Press Esc from the input box to navigate the feed with j/k/g/G",
        SettingsCategory::Behavior,
    )
}

fn telemetry_item(state: &AppState) -> SettingItem {
    SettingItem::new(
        "telemetry_enabled",
        "Telemetry",
        SettingValue::Bool(state.config().telemetry.is_enabled()),
        "Anonymous usage analytics",
        SettingsCategory::Safety,
    )
}

fn truncation_max_lines_item(state: &AppState) -> SettingItem {
    SettingItem::new(
        "truncation_max_lines",
        "Truncation Max Lines",
        SettingValue::Cycle {
            current: state.config().truncation.max_lines.to_string(),
            options: truncation_lines_options(),
        },
        "Max lines kept from a single tool output",
        SettingsCategory::Behavior,
    )
}

fn truncation_max_bytes_item(state: &AppState) -> SettingItem {
    SettingItem::new(
        "truncation_max_bytes",
        "Truncation Max Bytes",
        SettingValue::Cycle {
            current: state.config().truncation.max_bytes.to_string(),
            options: truncation_bytes_options(),
        },
        "Max bytes kept from a single tool output",
        SettingsCategory::Behavior,
    )
}

fn provider_options(state: &AppState) -> Vec<String> {
    let configured: Vec<String> = state
        .configured_providers()
        .into_iter()
        .map(|(name, _, _)| name)
        .collect();
    if !configured.is_empty() {
        return configured;
    }
    crate::provider::known_providers()
        .iter()
        .map(|p| p.key.to_string())
        .collect()
}

fn model_options(state: &AppState, provider: &str) -> Vec<String> {
    let configured: Vec<String> = state
        .configured_providers()
        .into_iter()
        .filter(|(name, _, _)| name == provider)
        .flat_map(|(_, _, models)| models)
        .collect();
    if !configured.is_empty() {
        return configured;
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_setting_items_includes_provider_models_multi_select() {
        let mut state = AppState::default();
        state.config_mut().current_provider = "openai".into();
        let items = build_setting_items(&state);
        let edit = items
            .iter()
            .find(|i| i.label == "Provider Models")
            .expect("Provider Models setting should exist");
        assert_eq!(edit.key, "openai");
        assert!(
            matches!(edit.value, SettingValue::MultiSelect { .. }),
            "Provider Models should be a multi-select, got {:?}",
            edit.value
        );
    }
}
