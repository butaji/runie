//! Settings application (toggles, checkboxes, selects).

use crate::actors::ConfigMsg;
use crate::dialog::{ItemAction, PanelItem, PanelStack};
use crate::model::{AppState, DeliveryMode, ThinkingLevel};

/// Apply the panel setting change.
pub fn apply_panel_setting(state: &mut AppState, stack: &mut PanelStack, key: &str) {
    if apply_checkbox_setting(state, key) {
        return;
    }
    match key {
        "steering_mode" => {
            state.config_mut().steering_mode = cycle_delivery_mode(state.config().steering_mode)
        }
        "follow_up_mode" => {
            state.config_mut().follow_up_mode = cycle_delivery_mode(state.config().follow_up_mode)
        }
        "provider" | "model" | "theme" | "thinking_level" => {
            apply_select_setting(state, stack, key);
        }
        "truncation_max_lines" | "truncation_max_bytes" => {
            apply_truncation_setting(state, stack, key);
        }
        // intentionally ignored: other settings are handled by their specific handlers
        _ => {}
    }
}

/// Apply a checkbox/toggle setting.
pub fn apply_checkbox_setting(state: &mut AppState, key: &str) -> bool {
    if let Some((provider, model)) = super::super::toggles::parse_provider_model_toggle(key) {
        super::super::toggles::toggle_provider_model(state, provider, model);
        return true;
    }
    match key {
        "read_only" => state.toggle_read_only(),
        "vim_mode" => toggle_vim_mode(state),
        "telemetry_enabled" => toggle_telemetry(state),
        _ => return false,
    }
    true
}

/// Apply a select/choice setting.
pub fn apply_select_setting(state: &mut AppState, stack: &mut PanelStack, key: &str) {
    let Some(value) = selected_select_value(stack) else {
        return;
    };
    match key {
        "provider" => {
            state.set_provider(&value);
            state.view_mut().cached_settings_valid = false;
        }
        "model" => {
            state.set_model(&value);
            state.view_mut().cached_settings_valid = false;
        }
        "theme" => state.switch_theme(value),
        "thinking_level" => {
            if let Ok(level) = value.parse::<ThinkingLevel>() {
                state.set_thinking_level(level);
            }
        }
        // intentionally ignored: other settings have specific handlers above
        _ => {}
    }
}

/// Apply a truncation limit setting.
pub fn apply_truncation_setting(state: &mut AppState, stack: &mut PanelStack, key: &str) {
    let Some(value) = selected_select_value(stack) else {
        return;
    };
    let Ok(n) = value.parse::<usize>() else {
        return;
    };
    let mut truncation = state.config().truncation.clone();
    match key {
        "truncation_max_lines" => truncation.max_lines = n,
        "truncation_max_bytes" => truncation.max_bytes = n,
        _ => return,
    }
    state.config_mut().truncation = truncation.clone();
    // Persist to config.toml via ConfigActor (fire-and-forget).
    // In tests without handles, mutation is already applied above.
    if let Some(h) = state.actor_handles() {
        let _ = h.config.try_send(ConfigMsg::SetTruncation { limits: truncation });
    }
    state.view_mut().cached_settings_valid = false;
}

/// Cycle through delivery modes.
fn cycle_delivery_mode(mode: DeliveryMode) -> DeliveryMode {
    match mode {
        DeliveryMode::OneAtATime => DeliveryMode::All,
        DeliveryMode::All => DeliveryMode::OneAtATime,
    }
}

/// Get the selected value from a select panel item.
fn selected_select_value(stack: &mut PanelStack) -> Option<String> {
    stack
        .current_mut()
        .and_then(|p| p.selected_item_mut())
        .and_then(|item| match item {
            PanelItem::Select { current, .. } => Some(current.clone()),
            _ => None,
        })
}

/// Toggle vim mode.
fn toggle_vim_mode(state: &mut AppState) {
    let new_value = !state.config().vim_mode;
    state.config_mut().vim_mode = new_value;
    // Persist to config.toml via ConfigActor (fire-and-forget).
    // In tests without handles, mutation is already applied above.
    if let Some(h) = state.actor_handles() {
        let _ = h.config.try_send(ConfigMsg::SetVimMode { enabled: new_value });
    }
    state.view_mut().cached_settings_valid = false;
}

/// Toggle telemetry.
fn toggle_telemetry(state: &mut AppState) {
    let new_enabled = !state.config().telemetry_enabled();
    state.config_mut().telemetry.enabled = new_enabled;
    // Persist to config.toml via ConfigActor (fire-and-forget).
    // In tests without handles, mutation is already applied above.
    if let Some(h) = state.actor_handles() {
        let _ = h.config.try_send(ConfigMsg::SetTelemetry { enabled: new_enabled });
    }
    state.view_mut().cached_settings_valid = false;
}

/// Toggle a checkbox item and apply its side effect.
pub fn toggle_checkbox_item(state: &mut AppState, item: &mut PanelItem) -> bool {
    if let PanelItem::Toggle { value, action, .. } = item {
        *value = !*value;
        match action {
            ItemAction::Toggle(key) => {
                apply_checkbox_setting(state, key);
            }
            ItemAction::Emit(evt) => {
                state.update(evt.clone());
            }
            // intentionally ignored: other item actions are handled elsewhere
            _ => {}
        }
        state.view_mut().dirty = true;
        true
    } else {
        false
    }
}
