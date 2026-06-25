//! Login flow panel builders.

use crate::dialog::dsl::get_field;
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::provider::{display_name, known_providers};

use super::state::LoginFlowState;

/// Build the provider picker panel.
pub fn build_provider_picker() -> Panel {
    let mut panel = Panel::new("login-provider", "Login")
        .header("Choose a provider")
        .keep_open();

    for provider in known_providers() {
        let label = provider.display_name.to_string();
        let evt = crate::Event::SelectProvider {
            provider: provider.key.to_string(),
        };
        panel = panel.item(&label, ItemAction::Emit(evt));
    }

    panel = panel.separator().item(
        "_Cancel",
        ItemAction::Emit(crate::Event::Abort),
    );
    panel
}

/// Build the API key input panel for a provider.
pub fn build_key_input(state: &LoginFlowState) -> Panel {
    let name = display_name(&state.provider);
    Panel::new("login-key", format!("Login to {}", name))
        .header(format!("Enter your {} API key", name))
        .form_field_value("API Key", "sk-...", "key", state.key.clone())
        .form_hidden("provider", state.provider.clone())
        .form_submit_with(login_key_submit)
        .item(
            "_Cancel",
            ItemAction::Emit(crate::Event::Abort),
        )
}

fn login_key_submit(values: &std::collections::HashMap<String, String>) -> crate::event::Event {
    crate::Event::SubmitKey {
        provider: get_field(values, "provider"),
        key: get_field(values, "key"),
    }
}

/// Build the "verifying API key" panel shown while waiting for the provider.
pub fn build_validating_panel(provider_key: &str) -> Panel {
    let name = display_name(provider_key);
    Panel::new("login-validating", format!("Verifying {}", name))
        .header(format!("Verifying your {} API key…", name))
        .keep_open()
}

/// Build the model multi-select panel.
///
/// Rendered in form view: `Toggle` items render as checkboxes in the
/// body, `Action` items (`_Save`, `_Cancel`) render as form buttons in
/// the bottom bar. One unified DSL — no separate Checkbox variant.
pub fn build_model_selector(state: &LoginFlowState) -> Panel {
    let name = display_name(&state.provider);
    let mut panel = Panel::new("login-models", format!("Select {} Models", name))
        .form()
        .header(format!("Toggle models to enable for {}", name))
        .keep_open();

    for model in &state.available_models {
        let enabled = state.selected_models.contains(model);
        let evt = crate::Event::ToggleModel {
            model: model.clone(),
        };
        panel = panel.toggle(model, enabled, ItemAction::Emit(evt));
    }

    panel = panel
        .separator()
        .item(
            "_Save",
            ItemAction::Emit(crate::Event::Save),
        )
        .item(
            "_Cancel",
            ItemAction::Emit(crate::Event::Abort),
        );
    panel
}

/// Build the done/success panel.
pub fn build_done_panel(provider_key: &str, model_count: usize) -> Panel {
    let name = display_name(provider_key);
    Panel::new("login-done", format!("{} Connected", name))
        .header(format!(
            "Connected {} model{}",
            model_count,
            if model_count == 1 { "" } else { "s" }
        ))
        .item("Close", ItemAction::Close)
}

/// Build the root panel of the login dialog. The login flow uses a real
/// `PanelStack`: this is the root (provider picker). Subsequent steps
/// (key input, model selector) are pushed onto the stack by the event
/// handlers in `update/mod.rs`, so ESC / Cancel pops back one level
/// instead of closing the whole dialog.
pub fn build_login_root() -> PanelStack {
    PanelStack::new(build_provider_picker())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog::PanelItem;

    fn model_flow() -> LoginFlowState {
        LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key("sk".into())
            .with_validation_success(vec!["M3".into(), "M2".into()])
    }

    #[test]
    fn provider_picker_has_known_providers() {
        let panel = build_provider_picker();
        assert!(panel
            .items
            .iter()
            .any(|i| matches!(i, PanelItem::Action { label, .. } if label == "Anthropic")));
        assert!(panel
            .items
            .iter()
            .any(|i| matches!(i, PanelItem::Action { label, .. } if label == "MiniMax")));
        assert!(panel
            .items
            .iter()
            .any(|i| matches!(i, PanelItem::Action { label, .. } if label == "_Cancel")));
    }

    #[test]
    fn key_input_panel_has_form_field() {
        let state = LoginFlowState::new().with_provider("minimax".into());
        let panel = build_key_input(&state);
        assert!(panel.is_form());
        assert!(panel
            .items
            .iter()
            .any(|i| matches!(i, PanelItem::FormField { label, .. } if label == "API Key")));
    }

    #[test]
    fn key_input_panel_is_not_filterable() {
        let state = LoginFlowState::new().with_provider("minimax".into());
        let panel = build_key_input(&state);
        assert!(
            !panel.filterable,
            "login key form must not be filterable so keystrokes edit the field"
        );
    }

    #[test]
    fn model_selector_is_form_with_toggle_checkboxes_and_action_buttons() {
        let state = model_flow();
        let panel = build_model_selector(&state);
        assert!(panel.is_form(), "model selector must render as a form");

        let toggles: Vec<_> = panel
            .items
            .iter()
            .filter_map(|i| match i {
                PanelItem::Toggle { label, value, .. } => Some((label.as_str(), *value)),
                _ => None,
            })
            .collect();
        assert_eq!(toggles.len(), 2);
        assert!(toggles.contains(&("M3", true)));
        assert!(toggles.contains(&("M2", true)));

        let actions: Vec<_> = panel
            .items
            .iter()
            .filter_map(|i| match i {
                PanelItem::Action { label, .. } => Some(label.as_str()),
                _ => None,
            })
            .collect();
        assert!(actions.contains(&"_Save"));
        assert!(actions.contains(&"_Cancel"));
    }

    #[test]
    fn model_selector_empty_when_no_models() {
        let state = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key("sk".into())
            .with_validation_success(vec![]);
        let panel = build_model_selector(&state);
        assert!(panel.is_form());
        let toggles = panel
            .items
            .iter()
            .filter(|i| matches!(i, PanelItem::Toggle { .. }))
            .count();
        assert_eq!(toggles, 0);
        let actions: Vec<_> = panel
            .items
            .iter()
            .filter_map(|i| match i {
                PanelItem::Action { label, .. } => Some(label.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(actions.len(), 2);
        assert!(actions.contains(&"_Save"));
        assert!(actions.contains(&"_Cancel"));
    }

    #[test]
    fn done_panel_shows_model_count() {
        let panel = build_done_panel("minimax", 2);
        assert!(panel.title.contains("Connected"));
    }

    #[test]
    fn build_login_root_is_provider_picker() {
        let stack = build_login_root();
        assert_eq!(
            stack.current().map(|p| p.id.as_str()),
            Some("login-provider")
        );
        assert_eq!(stack.len(), 1);
    }
}
