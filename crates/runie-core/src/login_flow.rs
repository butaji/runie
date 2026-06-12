//! Login flow — multi-step dialog for provider authentication.
//!
//! Steps:
//!   1. Provider picker (choose from known providers)
//!   2. API key input (form field)
//!   3. Model multi-select (toggle items) — pre-populated with the
//!      provider's `default_models`. A background fetch from the provider's
//!      `/models` endpoint enriches the list when it succeeds; failures
//!      show a non-blocking warning and the defaults are kept.
//!   4. Done
//!
//! The flow is **non-blocking**: submitting an API key transitions
//! immediately to the model selector. The user is never gated on a network
//! round-trip, so the UI can never get "stuck" on validation.

use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::provider_registry::{display_name, known_providers};
use crate::Event;
use std::collections::HashSet;

/// Current step in the login flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginStep {
    ProviderPicker,
    KeyInput,
    ModelSelect,
    Done,
}

/// Mutable state for the login dialog flow.
#[derive(Debug, Clone, PartialEq)]
pub struct LoginFlowState {
    pub step: LoginStep,
    pub provider: String,
    pub key: String,
    pub available_models: Vec<String>,
    pub selected_models: HashSet<String>,
}

impl LoginFlowState {
    pub fn new() -> Self {
        Self {
            step: LoginStep::ProviderPicker,
            provider: String::new(),
            key: String::new(),
            available_models: Vec::new(),
            selected_models: HashSet::new(),
        }
    }

    pub fn with_provider(self, provider: String) -> Self {
        Self {
            step: LoginStep::KeyInput,
            provider,
            ..self
        }
    }

    /// Transition to the model selector, pre-populating with the given
    /// default models (typically the provider's `default_models` from the
    /// registry). All provided models are selected by default.
    pub fn with_key_and_defaults(self, key: String, default_models: Vec<String>) -> Self {
        let selected_models: HashSet<String> = default_models.iter().cloned().collect();
        Self {
            step: LoginStep::ModelSelect,
            key,
            available_models: default_models,
            selected_models,
            ..self
        }
    }

    /// Replace the model list with the result of a background fetch.
    /// Models that existed in the previous list keep their selection state;
    /// newly discovered models are selected by default; models that
    /// disappeared are deselected.
    pub fn with_fetched_models(self, fetched: Vec<String>) -> Self {
        let mut new_selected = HashSet::new();
        for m in &fetched {
            if self.available_models.contains(m) {
                // Existed before: preserve the user's toggle.
                if self.selected_models.contains(m) {
                    new_selected.insert(m.clone());
                }
            } else {
                // Newly discovered: select by default.
                new_selected.insert(m.clone());
            }
        }
        Self {
            available_models: fetched,
            selected_models: new_selected,
            ..self
        }
    }

    pub fn toggle_model(&mut self, model: &str) {
        if self.selected_models.contains(model) {
            self.selected_models.remove(model);
        } else {
            self.selected_models.insert(model.to_string());
        }
    }

    pub fn is_done(&self) -> bool {
        self.step == LoginStep::Done
    }
}

impl Default for LoginFlowState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Panel builders
// ============================================================================

/// Build the provider picker panel.
pub fn build_provider_picker() -> Panel {
    let mut panel = Panel::new("login-provider", "Login")
        .header("Choose a provider")
        .keep_open();

    for provider in known_providers() {
        let label = provider.display_name.to_string();
        let evt = Event::LoginFlowSelectProvider {
            provider: provider.key.to_string(),
        };
        panel = panel.item(&label, ItemAction::Emit(evt));
    }

    panel = panel
        .separator()
        .item("Cancel", ItemAction::Emit(Event::LoginFlowCancel));
    panel
}

/// Build the API key input panel for a provider.
pub fn build_key_input(provider_key: &str) -> Panel {
    let name = display_name(provider_key);
    Panel::new("login-key", format!("Login to {}", name))
        .header(format!("Enter your {} API key", name))
        .form_field("API Key", "sk-...", "key")
        .item(
            "_Submit",
            ItemAction::Emit(Event::LoginFlowSubmitKey {
                provider: provider_key.to_string(),
                key: String::new(),
            }),
        )
        .item("_Cancel", ItemAction::Emit(Event::LoginFlowCancel))
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
        let evt = Event::LoginFlowToggleModel {
            model: model.clone(),
        };
        panel = panel.toggle(model, enabled, ItemAction::Emit(evt));
    }

    panel = panel
        .separator()
        .item("_Save", ItemAction::Emit(Event::LoginFlowSave))
        .item("_Cancel", ItemAction::Emit(Event::LoginFlowCancel));
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

/// Build a PanelStack for the current login flow state.
pub fn build_login_stack(state: &LoginFlowState) -> PanelStack {
    match state.step {
        LoginStep::ProviderPicker => PanelStack::new(build_provider_picker()),
        LoginStep::KeyInput => PanelStack::new(build_key_input(&state.provider)),
        LoginStep::ModelSelect => PanelStack::new(build_model_selector(state)),
        LoginStep::Done => PanelStack::new(build_done_panel(
            &state.provider,
            state.selected_models.len(),
        )),
    }
}

// ============================================================================
// Tests
// ============================================================================
//
// Scenario / Flow breakdown for the non-blocking login flow:
//
//   S1. Happy path, network fast
//   S2. Happy path, network slow
//   S3. Network unreachable / timeout
//   S4. Invalid key (401)
//   S5. User toggles model, then fetch returns
//   S6. User saves before fetch returns
//   S7. User cancels before fetch returns
//   S8. Fetch returns empty list
//   S9. Unknown provider
//  S10. Fetch returns subset of defaults
//  S11. Fetch returns superset of defaults (new models selected)
//  S12. Fetch returns disjoint list
//  S13. Empty key submitted (no fetch spawned, defaults shown)
//
// In every scenario the user lands on the model selector **immediately**
// after submitting the key. The network call is a best-effort enrichment.


#[cfg(test)]
mod tests;

