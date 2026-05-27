// ============================================================================
// View Model Builder Tests - Input Bar
// ============================================================================

use crate::tui::state::AppState;
use crate::tui::view_models::ViewModels;
use crate::components::CommandPalette;

fn make_state() -> AppState {
    AppState::default()
}

fn build_vms(state: &AppState) -> ViewModels {
    let render = crate::tui::state::RenderState::from(state);
    ViewModels::from_render_state(&render, &CommandPalette::default())
}

#[test]
fn test_input_bar_vm_default() {
    let state = make_state();
    let vms = build_vms(&state);
    assert_eq!(vms.input_bar.prompt, "❯ ");
    assert!(vms.input_bar.right_info.is_empty());
}

#[test]
fn test_input_bar_vm_with_right_info() {
    let mut state = make_state();
    state.input_right_info = "100 tokens".to_string();
    let vms = build_vms(&state);
    assert_eq!(vms.input_bar.right_info, "100 tokens");
}

#[test]
fn test_input_bar_vm_with_model() {
    let mut state = make_state();
    state.current_model = Some("openai/gpt-4o".to_string());
    state.input_right_info = "openai/gpt-4o".to_string();
    let vms = build_vms(&state);
    assert!(vms.input_bar.right_info.contains("openai"));
}

#[test]
fn test_input_bar_vm_prompt_always_same() {
    let mut state = make_state();
    state.input_right_info = "different info".to_string();
    let vms = build_vms(&state);
    assert_eq!(vms.input_bar.prompt, "❯ ");
}
