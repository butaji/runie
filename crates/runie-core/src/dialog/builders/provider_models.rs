//! Provider-models dialog builder.
//!
//! Lets the user toggle which models are enabled for a connected provider.

use std::collections::HashSet;

use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::event::DialogEvent;

const PANEL_ID: &str = "provider-models";

/// Build a toggle panel for enabling/disabling models for a provider.
///
/// `available` is the union of known provider models and any models already
/// saved in config, so user-added or previously selected models are preserved.
/// `selected` is the set of currently enabled models.
pub fn provider_models(
    provider: &str,
    available: &[String],
    selected: &HashSet<String>,
) -> PanelStack {
    let title = format!("{} Models", crate::provider_registry::display_name(provider));
    let mut panel = Panel::new(PANEL_ID, &title)
        .form()
        .header(format!("Toggle models for {}", provider))
        .keep_open();

    for model in available {
        let enabled = selected.contains(model);
        let evt = DialogEvent::ProviderModelsToggle {
            provider: provider.to_string(),
            model: model.clone(),
        };
        panel = panel.toggle(model, enabled, ItemAction::Emit(evt));
    }

    panel = panel
        .separator()
        .item(
            "_Save",
            ItemAction::Emit(DialogEvent::ProviderModelsSave {
                provider: provider.to_string(),
                models: Vec::new(),
            }),
        )
        .item("_Cancel", ItemAction::Emit(DialogEvent::ProviderModelsClose));

    PanelStack::new(panel)
}

/// Returns true if the current panel stack is the provider-models dialog.
pub fn is_provider_models_stack(stack: &PanelStack) -> bool {
    stack.current().map(|p| p.id.as_str()) == Some(PANEL_ID)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog::PanelItem;

    #[test]
    fn builder_creates_toggle_per_model() {
        let stack = provider_models(
            "minimax",
            &["M3".into(), "M2".into()],
            &["M3".into()].into_iter().collect(),
        );
        let panel = stack.current().expect("panel");

        let toggles: Vec<_> = panel
            .items
            .iter()
            .filter_map(|i| match i {
                PanelItem::Toggle { label, value, .. } => Some((label.as_str(), *value)),
                _ => None,
            })
            .collect();

        assert_eq!(toggles, vec![("M3", true), ("M2", false)]);
    }

    #[test]
    fn builder_includes_save_and_cancel_actions() {
        let stack = provider_models("minimax", &[], &HashSet::new());
        let panel = stack.current().expect("panel");

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
    fn is_provider_models_stack_detects_id() {
        let stack = provider_models("minimax", &[], &HashSet::new());
        assert!(is_provider_models_stack(&stack));

        let other = PanelStack::new(Panel::new("other", "Other"));
        assert!(!is_provider_models_stack(&other));
    }
}
