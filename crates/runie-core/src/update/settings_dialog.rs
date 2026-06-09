//! Settings dialog update logic

use crate::model::{AppState, DeliveryMode};
use crate::settings::{SettingItem, SettingValue, SettingsCategory};
use crate::Event;
use crate::commands::DialogState;

pub fn update_settings_dialog(state: &mut AppState, event: Event, category: SettingsCategory, selected: usize) {
    let items = build_setting_items(state);
    let category_items: Vec<_> = items.iter().filter(|i| i.category == category).collect();

    match event {
        Event::Abort | Event::SettingsClose | Event::ToggleSettingsDialog => {
            state.open_dialog = None;
            state.mark_dirty();
        }
        Event::HistoryPrev | Event::SettingsUp => {
            let new_sel = if selected == 0 {
                category_items.len().saturating_sub(1)
            } else {
                selected - 1
            };
            state.open_dialog = Some(DialogState::Settings { category, selected: new_sel });
            state.mark_dirty();
        }
        Event::HistoryNext | Event::SettingsDown => {
            let new_sel = if category_items.is_empty() {
                0
            } else {
                (selected + 1) % category_items.len()
            };
            state.open_dialog = Some(DialogState::Settings { category, selected: new_sel });
            state.mark_dirty();
        }
        Event::CursorLeft | Event::SettingsLeft => {
            let cats = SettingsCategory::all();
            let cat_idx = cats.iter().position(|&c| c == category).unwrap_or(0);
            let new_cat = if cat_idx == 0 {
                cats[cats.len() - 1]
            } else {
                cats[cat_idx - 1]
            };
            state.open_dialog = Some(DialogState::Settings { category: new_cat, selected: 0 });
            state.mark_dirty();
        }
        Event::CursorRight | Event::SettingsRight => {
            let cats = SettingsCategory::all();
            let cat_idx = cats.iter().position(|&c| c == category).unwrap_or(0);
            let new_cat = cats[(cat_idx + 1) % cats.len()];
            state.open_dialog = Some(DialogState::Settings { category: new_cat, selected: 0 });
            state.mark_dirty();
        }
        Event::Submit | Event::SettingsSelect => {
            if let Some(item) = category_items.get(selected) {
                apply_setting(state, &item.key);
            }
            state.open_dialog = Some(DialogState::Settings { category, selected });
            state.mark_dirty();
        }
        _ => {
            state.open_dialog = Some(DialogState::Settings { category, selected });
        }
    }
}

fn apply_setting(state: &mut AppState, key: &str) {
    match key {
        "read_only" => {
            state.config.read_only = !state.config.read_only;
            let status = if state.config.read_only { "enabled" } else { "disabled" };
            state.add_system_msg(format!("Read-only mode {}", status));
        }
        "steering_mode" => {
            state.steering_mode = match state.steering_mode {
                DeliveryMode::OneAtATime => DeliveryMode::All,
                DeliveryMode::All => DeliveryMode::OneAtATime,
            };
        }
        "follow_up_mode" => {
            state.follow_up_mode = match state.follow_up_mode {
                DeliveryMode::OneAtATime => DeliveryMode::All,
                DeliveryMode::All => DeliveryMode::OneAtATime,
            };
        }
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
                options: vec!["mock".into(), "openai".into(), "anthropic".into(), "google".into()],
            },
            "LLM provider",
            SettingsCategory::Models,
        ),
        SettingItem::new(
            "model",
            "Model",
            SettingValue::Enum {
                current: state.config.current_model.clone(),
                options: state.config.scoped_models.iter().map(|m| m.name.clone()).collect(),
            },
            "Active model",
            SettingsCategory::Models,
        ),
        SettingItem::new(
            "theme",
            "Theme",
            SettingValue::Enum {
                current: state.config.theme_name.clone(),
                options: vec!["silkcircuit-neon".into(), "dracula".into(), "nord".into()],
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
                }.to_string(),
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
                }.to_string(),
                options: vec!["one-at-a-time".into(), "all".into()],
            },
            "How follow-up messages are delivered",
            SettingsCategory::Behavior,
        ),
    ]
}
