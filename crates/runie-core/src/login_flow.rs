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
        .item("_Cancel", ItemAction::Emit(Event::LoginFlowCancel));
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

/// Build the root panel of the login dialog. The login flow uses a real
/// `PanelStack`: this is the root (provider picker). Subsequent steps
/// (key input, model selector) are pushed onto the stack by the event
/// handlers in `update/mod.rs`, so ESC / Cancel pops back one level
/// instead of closing the whole dialog.
pub fn build_login_root() -> PanelStack {
    PanelStack::new(build_provider_picker())
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

<<<<<<< HEAD
    // -----------------------------------------------------------------------
    // Layer 1: pure state transitions
    // -----------------------------------------------------------------------

    #[test]
    fn login_flow_starts_at_provider_picker() {
        let flow = LoginFlowState::new();
        assert_eq!(flow.step, LoginStep::ProviderPicker);
        assert!(flow.provider.is_empty());
        assert!(flow.key.is_empty());
        assert!(flow.available_models.is_empty());
        assert!(flow.selected_models.is_empty());
    }

    #[test]
    fn login_flow_select_provider() {
        let flow = LoginFlowState::new().with_provider("minimax".into());
        assert_eq!(flow.step, LoginStep::KeyInput);
        assert_eq!(flow.provider, "minimax");
    }

    #[test]
    fn login_flow_submit_key_goes_straight_to_model_select() {
        let flow = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key_and_defaults(
                "sk-test".into(),
                vec!["MiniMax-M3".into(), "MiniMax-M2.7".into()],
            );
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert_eq!(flow.key, "sk-test");
        assert_eq!(flow.available_models.len(), 2);
        assert!(flow.selected_models.contains("MiniMax-M3"));
        assert!(flow.selected_models.contains("MiniMax-M2.7"));
    }

    #[test]
    fn login_flow_submit_key_with_empty_defaults() {
        let flow = LoginFlowState::new()
            .with_provider("unknown".into())
            .with_key_and_defaults("k".into(), vec![]);
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert!(flow.available_models.is_empty());
        assert!(flow.selected_models.is_empty());
    }

    #[test]
    fn login_flow_toggle_model() {
        let mut flow = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key_and_defaults(
                "sk-test".into(),
                vec!["MiniMax-M3".into(), "MiniMax-M2.7".into()],
            );
        assert!(flow.selected_models.contains("MiniMax-M3"));
        flow.toggle_model("MiniMax-M3");
        assert!(!flow.selected_models.contains("MiniMax-M3"));
        flow.toggle_model("MiniMax-M3");
        assert!(flow.selected_models.contains("MiniMax-M3"));
    }

    #[test]
    fn login_flow_is_done() {
        let mut flow = LoginFlowState::new();
        assert!(!flow.is_done());
        flow.step = LoginStep::Done;
        assert!(flow.is_done());
    }

    // -----------------------------------------------------------------------
    // Layer 1: with_fetched_models semantics
    // -----------------------------------------------------------------------

    #[test]
    fn fetched_models_replaces_list_and_selects_new() {
        // S11: superset — new models are auto-selected.
        let flow = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key_and_defaults("k".into(), vec!["A".into(), "B".into()]);
        let flow = flow.with_fetched_models(vec!["A".into(), "B".into(), "C".into()]);
        assert_eq!(flow.available_models, vec!["A", "B", "C"]);
        assert!(flow.selected_models.contains("A"));
        assert!(flow.selected_models.contains("B"));
        assert!(flow.selected_models.contains("C"));
    }

    #[test]
    fn fetched_models_preserves_user_toggle_on_existing() {
        // S5: user deselected "A" before fetch returned.
        let mut flow = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key_and_defaults("k".into(), vec!["A".into(), "B".into()]);
        flow.toggle_model("A"); // user deselects A
        let flow = flow.with_fetched_models(vec!["A".into(), "B".into()]);
        assert!(
            !flow.selected_models.contains("A"),
            "user deselect must be preserved"
        );
        assert!(flow.selected_models.contains("B"));
    }

    #[test]
    fn fetched_models_drops_models_no_longer_returned() {
        // S10: subset — "B" is no longer available, must be deselected.
        let flow = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key_and_defaults("k".into(), vec!["A".into(), "B".into()]);
        let flow = flow.with_fetched_models(vec!["A".into()]);
        assert_eq!(flow.available_models, vec!["A"]);
        assert!(flow.selected_models.contains("A"));
        assert!(!flow.selected_models.contains("B"));
    }

    #[test]
    fn fetched_models_empty_list_clears_selection() {
        // S8: fetch returns no models.
        let flow = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key_and_defaults("k".into(), vec!["A".into()]);
        let flow = flow.with_fetched_models(vec![]);
        assert!(flow.available_models.is_empty());
        assert!(flow.selected_models.is_empty());
    }

    #[test]
    fn fetched_models_disjoint_list() {
        // S12: completely different models.
        let flow = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key_and_defaults("k".into(), vec!["A".into()]);
        let flow = flow.with_fetched_models(vec!["X".into(), "Y".into()]);
        assert_eq!(flow.available_models, vec!["X", "Y"]);
        assert!(flow.selected_models.contains("X"));
        assert!(flow.selected_models.contains("Y"));
    }

    // -----------------------------------------------------------------------
    // Layer 3: panel construction
    // -----------------------------------------------------------------------

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
    fn provider_picker_emits_select_event() {
        let panel = build_provider_picker();
        let minimax_item = panel
            .items
            .iter()
            .find(|i| matches!(i, PanelItem::Action { label, .. } if label == "MiniMax"));
        assert!(matches!(
            minimax_item,
            Some(PanelItem::Action { action: ItemAction::Emit(Event::LoginFlowSelectProvider { provider }), .. })
            if provider == "minimax"
        ));
    }

    #[test]
    fn key_input_panel_has_form_field() {
        let panel = build_key_input("minimax");
        assert!(panel.is_form());
        assert!(panel
            .items
            .iter()
            .any(|i| matches!(i, PanelItem::FormField { label, .. } if label == "API Key")));
    }

    #[test]
    fn model_selector_is_form_with_toggle_checkboxes_and_action_buttons() {
        // The model selector is a form: checkboxes for models in the body,
        // Save/Cancel as form buttons in the bottom bar. No list-of-strings
        // hack with "[✓]"/"[ ]" prefixes.
        let state = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key_and_defaults(
                "sk-test".into(),
                vec!["MiniMax-M3".into(), "MiniMax-M2.7".into()],
            );
        let panel = build_model_selector(&state);
        assert!(panel.is_form(), "model selector must render as a form");
        // Exactly two Toggle items (one per model), with the correct labels
        // and checked states derived from selected_models.
        let toggles: Vec<_> = panel
            .items
            .iter()
            .filter_map(|i| match i {
                PanelItem::Toggle { label, value, .. } => Some((label.as_str(), *value)),
                _ => None,
            })
            .collect();
        assert_eq!(toggles.len(), 2);
        assert!(toggles.contains(&("MiniMax-M3", true)));
        assert!(toggles.contains(&("MiniMax-M2.7", true)));
        // Save and Cancel are Action items (form buttons), not Toggles.
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
    fn model_selector_toggle_carries_toggle_model_event() {
        // Activating a toggle must emit LoginFlowToggleModel, not a generic
        // settings key. This is the event-based decoupling: the panel
        // doesn't know about login state; the app handles the event.
        let state = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key_and_defaults("sk".into(), vec!["M3".into(), "M2".into()]);
        let panel = build_model_selector(&state);
        let m3_action = panel
            .items
            .iter()
            .find_map(|i| match i {
                PanelItem::Toggle { label, action, .. } if label == "M3" => Some(action),
                _ => None,
            })
            .expect("M3 toggle must exist");
        match m3_action {
            ItemAction::Emit(Event::LoginFlowToggleModel { model }) => {
                assert_eq!(model, "M3");
            }
            other => panic!("M3 toggle must emit LoginFlowToggleModel, got {:?}", other),
        }
    }

    #[test]
    fn model_selector_save_emits_login_flow_save() {
        let state = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key_and_defaults("sk".into(), vec!["M3".into()]);
        let panel = build_model_selector(&state);
        let save_action = panel
            .items
            .iter()
            .find_map(|i| match i {
                PanelItem::Action { label, action } if label == "_Save" => Some(action),
                _ => None,
            })
            .expect("Save button must exist");
        assert!(matches!(
            save_action,
            ItemAction::Emit(Event::LoginFlowSave)
        ));
    }

    #[test]
    fn model_selector_empty_when_no_models() {
        let state = LoginFlowState::new()
            .with_provider("minimax".into())
            .with_key_and_defaults("sk".into(), vec![]);
        let panel = build_model_selector(&state);
        assert!(panel.is_form());
        // No toggles, just Save + Cancel as form buttons.
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
        // The login dialog opens with the root panel (provider picker).
        // Subsequent steps push panels onto this stack rather than
        // rebuilding a new stack per step.
        let stack = build_login_root();
        assert_eq!(
            stack.current().map(|p| p.id.as_str()),
            Some("login-provider")
        );
        assert_eq!(stack.len(), 1);
    }

    // -----------------------------------------------------------------------
    // Layer 2: event handling through AppState
    // -----------------------------------------------------------------------

    /// User-reported bug regression: `/login` → choose provider → key
    /// input appears → press Esc. Expected: go back to provider picker.
    /// Actual (before fix): dialog closed entirely.
    ///
    /// Root causes were:
    /// 1. Default keybinding mapped `escape` to `Abort` (force-close),
    ///    overriding the Esc → DialogBack mapping.
    /// 2. `apply_form_action(Submit)` closed the dialog, which made
    ///    the key input vanish from the back stack when submitted.
    #[test]
    fn key_input_esc_pops_to_provider_picker_not_close() {
        let mut state = AppState::default();
        state.update(Event::LoginFlowStart);
        state.update(Event::LoginFlowSelectProvider {
            provider: "minimax".into(),
        });
        // Now the key input should be on top of the stack.
        match &state.open_dialog {
            Some(crate::commands::DialogState::PanelStack(s)) => {
                assert_eq!(s.len(), 2, "stack should be [provider, key_input]");
                assert_eq!(s.current().unwrap().id, "login-key");
            }
            other => panic!("expected PanelStack, got {other:?}"),
        }
        // Esc on the key input must pop to the provider picker (root),
        // NOT close the dialog.
        state.update(Event::DialogBack);
        match &state.open_dialog {
            Some(crate::commands::DialogState::PanelStack(s)) => {
                assert_eq!(s.len(), 1, "Esc must pop to root, not close");
                assert_eq!(s.current().unwrap().id, "login-provider");
            }
            other => panic!("Esc on key input must leave dialog open at root, got {other:?}"),
        }
    }

    /// Helper: drive the login flow to the model selector with the given
    /// provider and a known-defaults key. Returns the resulting state.
    fn drive_to_model_select(provider: &str) -> AppState {
        let mut state = AppState::default();
        state.update(Event::LoginFlowStart);
        state.update(Event::LoginFlowSelectProvider {
            provider: provider.into(),
        });
        let defaults = crate::provider_registry::find_provider(provider)
            .map(|m| m.default_models.to_vec())
            .unwrap_or_default();
        state.update(Event::LoginFlowSubmitKey {
            provider: provider.into(),
            key: "sk-test".into(),
        });
        // SubmitKey handler must populate defaults itself.
        let flow = state.login_flow.as_ref().unwrap();
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert_eq!(flow.available_models, defaults);
        assert!(flow.selected_models.len() == defaults.len());
        state
    }

    // S1: happy path, network fast
    #[test]
    fn s1_submit_key_immediately_shows_defaults() {
        let state = drive_to_model_select("minimax");
        let flow = state.login_flow.as_ref().unwrap();
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert!(!flow.available_models.is_empty());
    }

    // S1 continued: background fetch success replaces the list
    #[test]
    fn s1_models_fetched_event_replaces_list() {
        let mut state = drive_to_model_select("minimax");
        state.update(Event::LoginFlowModelsFetched {
            provider: "minimax".into(),
            key: "sk-test".into(),
            models: vec!["new-A".into(), "new-B".into()],
        });
        let flow = state.login_flow.as_ref().unwrap();
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert_eq!(flow.available_models, vec!["new-A", "new-B"]);
        assert!(flow.selected_models.contains("new-A"));
        assert!(flow.selected_models.contains("new-B"));
    }

    // S2: happy path, slow network — defaults shown first, then replaced
    #[test]
    fn s2_slow_fetch_user_can_toggle_before_it_returns() {
        let mut state = drive_to_model_select("minimax");
        // User deselects one of the defaults
        let first_default = state.login_flow.as_ref().unwrap().available_models[0].clone();
        state.update(Event::LoginFlowToggleModel {
            model: first_default.clone(),
        });
        assert!(!state
            .login_flow
            .as_ref()
            .unwrap()
            .selected_models
            .contains(&first_default));

        // Fetch returns later with the same list
        let defaults = state.login_flow.as_ref().unwrap().available_models.clone();
        state.update(Event::LoginFlowModelsFetched {
            provider: "minimax".into(),
            key: "sk-test".into(),
            models: defaults.clone(),
        });
        let flow = state.login_flow.as_ref().unwrap();
        // The user's deselect is preserved.
        assert!(!flow.selected_models.contains(&first_default));
        for m in &defaults {
            if m != &first_default {
                assert!(flow.selected_models.contains(m));
            }
        }
    }

    // S3: network unreachable / timeout — transient warning, step unchanged
    #[test]
    fn s3_validation_failed_does_not_block_user() {
        let mut state = drive_to_model_select("minimax");
        state.update(Event::LoginFlowValidationFailed {
            provider: "minimax".into(),
            key: "sk-test".into(),
            error: "connection refused".into(),
        });
        let flow = state.login_flow.as_ref().unwrap();
        // Step must remain ModelSelect — the user is NOT pushed to an error panel.
        assert_eq!(flow.step, LoginStep::ModelSelect);
        // The transient message is set.
        let transient = state.transient_message.as_ref();
        assert!(transient.is_some(), "transient warning should be set");
        let content = transient.unwrap();
        assert!(content.contains("verify") || content.contains("refused"));
        // Defaults are still available.
        assert!(!flow.available_models.is_empty());
    }

    // S4: invalid key (401) — same as S3, non-blocking
    #[test]
    fn s4_invalid_key_shows_transient_not_error_panel() {
        let mut state = drive_to_model_select("minimax");
        state.update(Event::LoginFlowValidationFailed {
            provider: "minimax".into(),
            key: "sk-test".into(),
            error: "API validation failed: 401 Unauthorized".into(),
        });
        let flow = state.login_flow.as_ref().unwrap();
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert!(state.transient_message.is_some());
    }

    // S5: fetch result after user toggle (covered in s2; here we assert
    // that the new models from the fetch are auto-selected while the
    // user's deselect on an existing one sticks).
    #[test]
    fn s5_fetch_superset_preserves_toggle_and_selects_new() {
        let mut state = drive_to_model_select("minimax");
        let first = state.login_flow.as_ref().unwrap().available_models[0].clone();
        state.update(Event::LoginFlowToggleModel {
            model: first.clone(),
        });
        // Fetch returns the existing list plus a brand-new model.
        let mut new_list = state.login_flow.as_ref().unwrap().available_models.clone();
        new_list.push("brand-new-model".into());
        state.update(Event::LoginFlowModelsFetched {
            provider: "minimax".into(),
            key: "sk-test".into(),
            models: new_list,
        });
        let flow = state.login_flow.as_ref().unwrap();
        assert!(!flow.selected_models.contains(&first));
        assert!(flow.selected_models.contains("brand-new-model"));
    }

    // S6: user saves before fetch returns — fetch result is ignored
    #[test]
    fn s6_save_before_fetch_then_fetch_is_ignored() {
        let mut state = drive_to_model_select("minimax");
        state.update(Event::LoginFlowSave);
        // Login flow is cleared.
        assert!(state.login_flow.is_none());
        // Providers dialog is shown (so user can choose active model).
        assert!(state.open_dialog.is_some());
        // Late fetch event arrives; nothing to update since login_flow is None.
        state.update(Event::LoginFlowModelsFetched {
            provider: "minimax".into(),
            key: "sk-test".into(),
            models: vec!["late".into()],
        });
        // Still no flow.
        assert!(state.login_flow.is_none());
        // No transient warning from the late fetch.
        assert!(state.transient_message.is_none());
    }

    // S7: user cancels before fetch returns — fetch result is ignored
    #[test]
    fn s7_cancel_before_fetch_then_fetch_is_ignored() {
        let mut state = drive_to_model_select("minimax");
        // Cancel from the model selector pops back to the provider
        // picker (root). The key input was "consumed" (replaced) when
        // the user submitted it, so it is no longer in the back stack.
        // The dialog remains open at the provider step.
        state.update(Event::LoginFlowCancel);
        assert!(state.login_flow.is_some(), "cancel should pop, not close");
        let flow = state.login_flow.as_ref().unwrap();
        assert_eq!(flow.step, LoginStep::ProviderPicker);
        // A late fetch event for a step that is no longer ModelSelect
        // must be ignored: no transient warning, no state change.
        state.update(Event::LoginFlowValidationFailed {
            provider: "minimax".into(),
            key: "sk-test".into(),
            error: "late".into(),
        });
        assert!(
            state.transient_message.is_none(),
            "late failure must not surface"
        );
    }

    // S8: fetch returns empty list — model selector is empty
    #[test]
    fn s8_empty_fetch_replaces_with_empty_list() {
        let mut state = drive_to_model_select("minimax");
        state.update(Event::LoginFlowModelsFetched {
            provider: "minimax".into(),
            key: "sk-test".into(),
            models: vec![],
        });
        let flow = state.login_flow.as_ref().unwrap();
        assert!(flow.available_models.is_empty());
        assert!(flow.selected_models.is_empty());
    }

    // S9: unknown provider — no defaults
    #[test]
    fn s9_unknown_provider_no_defaults() {
        let mut state = AppState::default();
        state.update(Event::LoginFlowStart);
        state.update(Event::LoginFlowSelectProvider {
            provider: "ghost".into(),
        });
        state.update(Event::LoginFlowSubmitKey {
            provider: "ghost".into(),
            key: "k".into(),
        });
        let flow = state.login_flow.as_ref().unwrap();
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert!(flow.available_models.is_empty());
    }

    // S10: fetch returns subset of defaults — model is dropped
    #[test]
    fn s10_fetch_subset_drops_missing_model() {
        let mut state = drive_to_model_select("minimax");
        let original = state.login_flow.as_ref().unwrap().available_models.clone();
        assert!(original.len() >= 2);
        let subset: Vec<String> = original.iter().take(1).cloned().collect();
        state.update(Event::LoginFlowModelsFetched {
            provider: "minimax".into(),
            key: "sk-test".into(),
            models: subset.clone(),
        });
        let flow = state.login_flow.as_ref().unwrap();
        assert_eq!(flow.available_models, subset);
        assert!(flow.selected_models.contains(&subset[0]));
    }

    // S13: empty key submitted — fetch is not spawned by the main loop,
    // but the state still transitions to ModelSelect with defaults.
    #[test]
    fn s13_empty_key_still_shows_defaults() {
        let mut state = AppState::default();
        state.update(Event::LoginFlowStart);
        state.update(Event::LoginFlowSelectProvider {
            provider: "minimax".into(),
        });
        state.update(Event::LoginFlowSubmitKey {
            provider: "minimax".into(),
            key: "".into(),
        });
        let flow = state.login_flow.as_ref().unwrap();
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert!(!flow.available_models.is_empty());
    }

    // -----------------------------------------------------------------------
    // Layer 2: open / cancel
    // -----------------------------------------------------------------------

    #[test]
    fn login_command_opens_provider_picker() {
        let mut state = AppState::default();
        state.update(Event::LoginFlowStart);
        assert!(state.open_dialog.is_some());
        assert!(state.login_flow.is_some());
        assert_eq!(
            state.login_flow.as_ref().unwrap().step,
            LoginStep::ProviderPicker
        );
    }

    #[test]
    fn login_select_provider_pushes_key_input() {
        let mut state = AppState::default();
        state.update(Event::LoginFlowStart);
        state.update(Event::LoginFlowSelectProvider {
            provider: "minimax".into(),
        });
        assert_eq!(state.login_flow.as_ref().unwrap().step, LoginStep::KeyInput);
        assert_eq!(state.login_flow.as_ref().unwrap().provider, "minimax");
    }

    #[test]
    fn login_submit_key_preserves_provider_when_empty() {
        let mut state = AppState::default();
        state.update(Event::LoginFlowStart);
        state.update(Event::LoginFlowSelectProvider {
            provider: "minimax".into(),
        });
        state.update(Event::LoginFlowSubmitKey {
            provider: "".into(),
            key: "sk-test".into(),
        });
        let flow = state.login_flow.as_ref().unwrap();
        assert_eq!(flow.step, LoginStep::ModelSelect);
        assert_eq!(flow.provider, "minimax");
        assert_eq!(flow.key, "sk-test");
    }

    #[test]
    fn login_toggle_model_updates_selection() {
        let mut state = drive_to_model_select("minimax");
        let first = state.login_flow.as_ref().unwrap().available_models[0].clone();
        assert!(state
            .login_flow
            .as_ref()
            .unwrap()
            .selected_models
            .contains(&first));
        state.update(Event::LoginFlowToggleModel {
            model: first.clone(),
        });
        assert!(!state
            .login_flow
            .as_ref()
            .unwrap()
            .selected_models
            .contains(&first));
    }

    #[test]
    fn login_cancel_closes_dialog() {
        let mut state = AppState::default();
        state.update(Event::LoginFlowStart);
        state.update(Event::LoginFlowCancel);
        assert!(state.open_dialog.is_none());
        assert!(state.login_flow.is_none());
    }
}
=======
>>>>>>> review
