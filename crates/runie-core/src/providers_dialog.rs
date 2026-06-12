//! Providers dialog — unified interface for managing provider connections.
//!
//! Shows configured providers with their models, allows selecting the active
//! model, adding new providers (via the guided login flow), and disconnecting
//! existing providers.

use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::login_config::list_configured_providers;

/// Build the root providers dialog panel.
pub fn build_providers_dialog(current_provider: &str, current_model: &str) -> PanelStack {
    let configured = list_configured_providers();
    let has_providers = !configured.is_empty();

    let mut panel = Panel::new("providers", "Providers")
        .header(if has_providers {
            "Select a model to activate"
        } else {
            "No providers configured — add one to get started"
        })
        .keep_open();

    if has_providers {
        // List each configured provider with its models.
        for (provider_name, _, models) in &configured {
            let is_active = *provider_name == current_provider;
            let provider_label = if is_active {
                format!("{} (active)", provider_name)
            } else {
                provider_name.clone()
            };

            // Provider header (non-selectable, visual grouping).
            panel = panel.header(&format!("── {} ──", provider_label));

            // Models for this provider.
            for model in models {
                let is_current = is_active && *model == current_model;
                let label = if is_current {
                    format!("  {} ←", model)
                } else {
                    format!("  {}", model)
                };
                let evt = crate::Event::ProvidersSelectModel {
                    provider: provider_name.clone(),
                    model: model.clone(),
                };
                panel = panel.item(&label, ItemAction::Emit(evt));
            }

            // Disconnect option for this provider.
            let disconnect_label = format!("  ✕ Disconnect");
            let evt = crate::Event::ProvidersDisconnect {
                provider: provider_name.clone(),
            };
            panel = panel.item(&disconnect_label, ItemAction::Emit(evt));

            panel = panel.separator();
        }
    }

    // Add new provider option.
    panel = panel.item("+ Add provider", ItemAction::Emit(crate::Event::ProvidersAdd));

    panel = panel.item("Close", ItemAction::Close);

    PanelStack::new(panel)
}
