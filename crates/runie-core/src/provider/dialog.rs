//! Providers dialog — unified interface for managing provider connections.
//!
//! Shows configured providers with their models, allows selecting the active
//! model, adding new providers (via the guided login flow), and disconnecting
//! existing providers.

use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::model::AppState;

/// Build the root providers dialog panel.
pub fn build_providers_dialog(state: &AppState) -> PanelStack {
    let mut configured = state.configured_providers();
    // Include mock provider when enabled (dev-only).
    if crate::provider::is_mock_enabled() && !configured.iter().any(|(p, _, _)| p == "mock") {
        configured.push((
            "mock".to_owned(),
            "http://localhost/mock".to_owned(),
            vec!["echo".to_owned()],
        ));
    }
    let current_provider = &state.config().current_provider;
    let current_model = &state.config().current_model;
    let has_providers = !configured.is_empty();

    let mut panel = Panel::new("providers", "Providers").header(providers_header(has_providers));

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
            ItemAction::Emit(crate::Event::ProvidersAdd),
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

    let provider = provider_name.to_owned();
    for model in models {
        let label = model_label(model, is_active, model == current_model);
        let evt = crate::Event::ProvidersSelectModel {
            provider: provider.clone(),
            model: model.clone(),
        };
        panel = panel.item(label, ItemAction::Emit(evt));
    }

    let edit_evt = crate::Event::ProvidersEditModels {
        provider: provider.clone(),
    };
    let disconnect_evt = crate::Event::ProvidersDisconnect { provider };
    panel
        .item("  ✎ Edit models", ItemAction::Emit(edit_evt))
        .item("  ✕ Disconnect", ItemAction::Emit(disconnect_evt))
        .separator()
}

fn provider_label(name: &str, is_active: bool) -> String {
    if is_active {
        format!("{} (active)", name)
    } else {
        name.to_owned()
    }
}

fn model_label(model: &str, is_active: bool, is_current: bool) -> String {
    if is_active && is_current {
        format!("  {} ←", model)
    } else {
        format!("  {}", model)
    }
}

/// Build a dedicated panel for toggling which models are enabled for a provider.
pub fn build_provider_models_editor(state: &AppState, provider: &str) -> PanelStack {
    let (saved, available) = crate::update::settings_dialog::provider_model_lists(state, provider);
    let saved: std::collections::HashSet<String> = saved.into_iter().collect();
    let mut panel =
        Panel::new("provider-models", format!(" Edit {} models ", provider)).with_filter();
    for model in available {
        let key = format!("edit_provider:{}:{}", provider, model);
        panel = panel.toggle(&model, saved.contains(&model), ItemAction::Toggle(key));
    }
    PanelStack::new(panel)
}
