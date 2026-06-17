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
        .header(providers_header(has_providers))
        .keep_open();

    if has_providers {
        for (provider_name, _, models) in &configured {
            panel = add_provider_section(
                panel,
                provider_name,
                models,
                current_provider,
                current_model,
            );
        }
    }

    panel = panel
        .item(
            "+ Add provider",
            ItemAction::Emit(crate::event::DialogEvent::ProvidersAdd),
        )
        .item("Close", ItemAction::Close);

    PanelStack::new(panel)
}

fn providers_header(has_providers: bool) -> &'static str {
    if has_providers {
        "Select a model to activate"
    } else {
        "No providers configured — add one to get started"
    }
}

fn add_provider_section(
    mut panel: Panel,
    provider_name: &str,
    models: &[String],
    current_provider: &str,
    current_model: &str,
) -> Panel {
    let is_active = provider_name == current_provider;
    let provider_label = provider_label(provider_name, is_active);
    panel = panel.header(format!("── {} ──", provider_label));

    let provider = provider_name.to_string();
    for model in models {
        let label = model_label(model, is_active, model == current_model);
        let evt = crate::event::DialogEvent::ProvidersSelectModel {
            provider: provider.clone(),
            model: model.clone(),
        };
        panel = panel.item(label, ItemAction::Emit(evt));
    }

    let disconnect_evt = crate::event::DialogEvent::ProvidersDisconnect { provider };
    panel
        .item("  ✕ Disconnect", ItemAction::Emit(disconnect_evt))
        .separator()
}

fn provider_label(name: &str, is_active: bool) -> String {
    if is_active {
        format!("{} (active)", name)
    } else {
        name.to_string()
    }
}

fn model_label(model: &str, is_active: bool, is_current: bool) -> String {
    if is_active && is_current {
        format!("  {} ←", model)
    } else {
        format!("  {}", model)
    }
}
