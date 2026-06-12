//! Login flow pure-state tests.

use crate::dialog::{ItemAction, PanelItem};
use crate::login_flow::{
    build_done_panel, build_key_input, build_login_stack, build_model_selector,
    build_provider_picker, LoginFlowState, LoginStep,
};
use crate::Event;

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
        .any(|i| matches!(i, PanelItem::Action { label, .. } if label == "Cancel")));
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
fn build_login_stack_matches_step() {
    let state = LoginFlowState::new();
    let stack = build_login_stack(&state);
    assert_eq!(
        stack.current().map(|p| p.id.as_str()),
        Some("login-provider")
    );

    let state = state.with_provider("minimax".into());
    let stack = build_login_stack(&state);
    assert_eq!(stack.current().map(|p| p.id.as_str()), Some("login-key"));

    let state = state.with_key_and_defaults("M3".into(), vec!["M3".into()]);
    let stack = build_login_stack(&state);
    assert_eq!(stack.current().map(|p| p.id.as_str()), Some("login-models"));
}

// -----------------------------------------------------------------------
