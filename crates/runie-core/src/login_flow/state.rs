//! Login flow state machine.

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
    /// Whether the API key has been validated by a successful /models call.
    pub validated: bool,
}

impl LoginFlowState {
    pub fn new() -> Self {
        Self {
            step: LoginStep::ProviderPicker,
            provider: String::new(),
            key: String::new(),
            available_models: Vec::new(),
            selected_models: HashSet::new(),
            validated: false,
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
    /// default models (typically the provider's `models` list from the
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
