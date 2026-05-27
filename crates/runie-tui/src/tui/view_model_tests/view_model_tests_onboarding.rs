// ============================================================================
// View Model Builder Tests - Onboarding
// ============================================================================

use crate::components::onboarding::{Onboarding, OnboardingStep, ProviderOption, ModelOption};
use crate::tui::state::{AppState, TuiMode};
use crate::tui::view_models::{ViewModels, OnboardingStep as VmStep};
use crate::components::CommandPalette;

fn make_state() -> AppState {
    AppState::default()
}

fn build_vms(state: &AppState) -> ViewModels {
    let render = crate::tui::state::RenderState::from(state);
    ViewModels::from_render_state(&render, &CommandPalette::default())
}

#[test]
fn test_onboarding_vm_none_when_not_onboarding() {
    let mut state = make_state();
    state.mode = TuiMode::Chat;
    state.onboarding = None;
    let vms = build_vms(&state);
    assert!(vms.onboarding.is_none());
}

#[test]
fn test_onboarding_vm_welcome_step() {
    let mut state = make_state();
    state.mode = TuiMode::Onboarding;
    state.onboarding = Some(Onboarding::new());
    let vms = build_vms(&state);
    assert!(vms.onboarding.is_some());
    assert!(matches!(vms.onboarding.as_ref().unwrap().step, VmStep::Welcome));
}

#[test]
fn test_onboarding_vm_provider_select_step() {
    let mut state = make_state();
    state.mode = TuiMode::Onboarding;
    let mut onboarding = Onboarding::new();
    onboarding.step = OnboardingStep::ProviderSelect;
    state.onboarding = Some(onboarding);
    let vms = build_vms(&state);
    assert!(matches!(vms.onboarding.unwrap().step, VmStep::ProviderSelect));
}

#[test]
fn test_onboarding_vm_key_input_step() {
    let mut state = make_state();
    state.mode = TuiMode::Onboarding;
    let mut onboarding = Onboarding::new();
    onboarding.step = OnboardingStep::KeyInput;
    state.onboarding = Some(onboarding);
    let vms = build_vms(&state);
    assert!(matches!(vms.onboarding.unwrap().step, VmStep::KeyInput));
}

#[test]
fn test_onboarding_vm_model_select_step() {
    let mut state = make_state();
    state.mode = TuiMode::Onboarding;
    let mut onboarding = Onboarding::new();
    onboarding.step = OnboardingStep::ModelSelect;
    state.onboarding = Some(onboarding);
    let vms = build_vms(&state);
    assert!(matches!(vms.onboarding.unwrap().step, VmStep::ModelSelect));
}

#[test]
fn test_onboarding_vm_complete_step() {
    let mut state = make_state();
    state.mode = TuiMode::Onboarding;
    let mut onboarding = Onboarding::new();
    onboarding.step = OnboardingStep::Complete;
    state.onboarding = Some(onboarding);
    let vms = build_vms(&state);
    assert!(matches!(vms.onboarding.unwrap().step, VmStep::Complete));
}

#[test]
fn test_onboarding_vm_selected_item() {
    let mut state = make_state();
    state.mode = TuiMode::Onboarding;
    let mut onboarding = Onboarding::new();
    onboarding.selected_item = 5;
    state.onboarding = Some(onboarding);
    let vms = build_vms(&state);
    assert_eq!(vms.onboarding.unwrap().selected_item, 5);
}

#[test]
fn test_onboarding_vm_selected_provider() {
    let mut state = make_state();
    state.mode = TuiMode::Onboarding;
    let mut onboarding = Onboarding::new();
    onboarding.selected_provider = Some(2);
    state.onboarding = Some(onboarding);
    let vms = build_vms(&state);
    assert_eq!(vms.onboarding.unwrap().selected_provider, Some(2));
}

#[test]
fn test_onboarding_vm_api_key_input() {
    let mut state = make_state();
    state.mode = TuiMode::Onboarding;
    let mut onboarding = Onboarding::new();
    onboarding.api_key_input = "sk-test-12345".to_string();
    state.onboarding = Some(onboarding);
    let vms = build_vms(&state);
    assert_eq!(vms.onboarding.unwrap().api_key_input, "sk-test-12345");
}

#[test]
fn test_onboarding_vm_selected_model() {
    let mut state = make_state();
    state.mode = TuiMode::Onboarding;
    let mut onboarding = Onboarding::new();
    onboarding.selected_model = Some(3);
    state.onboarding = Some(onboarding);
    let vms = build_vms(&state);
    assert_eq!(vms.onboarding.unwrap().selected_model, Some(3));
}

#[test]
fn test_onboarding_vm_providers_extracted() {
    let mut state = make_state();
    state.mode = TuiMode::Onboarding;
    let mut onboarding = Onboarding::new();
    onboarding.providers = vec![
        ProviderOption {
            name: "OpenAI".to_string(),
            id: "openai".to_string(),
            description: "GPT models".to_string(),
            key_prefix: "sk-".to_string(),
        },
    ];
    state.onboarding = Some(onboarding);
    let vms = build_vms(&state);
    assert!(vms.onboarding.unwrap().providers.contains(&"OpenAI".to_string()));
}

#[test]
fn test_onboarding_vm_models_extracted() {
    let mut state = make_state();
    state.mode = TuiMode::Onboarding;
    let mut onboarding = Onboarding::new();
    onboarding.models = vec![
        ModelOption {
            name: "GPT-4o".to_string(),
            id: "gpt-4o".to_string(),
            description: "Fast".to_string(),
        },
    ];
    state.onboarding = Some(onboarding);
    let vms = build_vms(&state);
    assert!(vms.onboarding.unwrap().models.contains(&"GPT-4o".to_string()));
}

#[test]
fn test_onboarding_vm_error_message() {
    let mut state = make_state();
    state.mode = TuiMode::Onboarding;
    let mut onboarding = Onboarding::new();
    onboarding.error_message = Some("Invalid API key".to_string());
    state.onboarding = Some(onboarding);
    let vms = build_vms(&state);
    assert_eq!(vms.onboarding.unwrap().error_message, Some("Invalid API key".to_string()));
}

#[test]
fn test_onboarding_vm_no_error_message() {
    let mut state = make_state();
    state.mode = TuiMode::Onboarding;
    state.onboarding = Some(Onboarding::new());
    let vms = build_vms(&state);
    assert!(vms.onboarding.unwrap().error_message.is_none());
}
