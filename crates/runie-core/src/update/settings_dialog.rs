//! Settings dialog update logic

use crate::model::{AppState, DeliveryMode};
use crate::settings::{SettingItem, SettingValue, SettingsCategory};
use crate::Event;

pub(crate) fn update(state: &mut AppState, event: Event) {
    use crate::commands::DialogState;
    match event {
        Event::ToggleSettingsDialog => {
            if matches!(state.open_dialog, Some(DialogState::Settings(_))) {
                state.open_dialog = None;
                state.mark_dirty();
            } else {
                state.open_settings_dialog();
            }
        }
        Event::SettingsUp
        | Event::SettingsDown
        | Event::SettingsLeft
        | Event::SettingsRight
        | Event::SettingsSelect
        | Event::SettingsClose => {}
        Event::PaletteFilter(_)
        | Event::PaletteBackspace
        | Event::PaletteUp
        | Event::PaletteDown
        | Event::PaletteSelect
        | Event::PaletteClose => {}
        Event::ModelSelectorFilter(_)
        | Event::ModelSelectorBackspace
        | Event::ModelSelectorUp
        | Event::ModelSelectorDown
        | Event::ModelSelectorSelect
        | Event::ModelSelectorClose => {}
        _ => {}
    }
}

pub fn build_setting_items(state: &AppState) -> Vec<SettingItem> {
    vec![
        SettingItem::new(
            "provider",
            "Provider",
            SettingValue::Enum {
                current: state.config.current_provider.clone(),
                options: vec![
                    "mock".into(),
                    "openai".into(),
                    "anthropic".into(),
                    "google".into(),
                ],
            },
            "LLM provider",
            SettingsCategory::Models,
        ),
        SettingItem::new(
            "model",
            "Model",
            SettingValue::Enum {
                current: state.config.current_model.clone(),
                options: state
                    .config
                    .scoped_models
                    .iter()
                    .map(|m| m.name.clone())
                    .collect(),
            },
            "Active model",
            SettingsCategory::Models,
        ),
        SettingItem::new(
            "theme",
            "Theme",
            SettingValue::Enum {
                current: state.config.theme_name.clone(),
                options: vec![
                    "runie".into(),
                    "silkcircuit-neon".into(),
                    "dracula".into(),
                    "nord".into(),
                ],
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
                current: match state.steering_mode {
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
                current: match state.follow_up_mode {
                    DeliveryMode::OneAtATime => "one-at-a-time",
                    DeliveryMode::All => "all",
                }
                .to_string(),
                options: vec!["one-at-a-time".into(), "all".into()],
            },
            "How follow-up messages are delivered",
            SettingsCategory::Behavior,
        ),
    ]
}
