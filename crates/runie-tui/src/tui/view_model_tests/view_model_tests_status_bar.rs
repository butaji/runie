// ============================================================================
// View Model Builder Tests - Status Bar
// ============================================================================

use crate::components::status_bar::{BackgroundJob, JobStatus};
use crate::tui::state::{AppState, TuiMode};
use crate::tui::view_models::ViewModels;
use crate::components::CommandPalette;
use runie_ai::TokenUsage;

fn make_state() -> AppState {
    AppState::default()
}

fn build_vms(state: &AppState) -> ViewModels {
    let render = crate::tui::state::RenderState::from(state);
    ViewModels::from_render_state(&render, &CommandPalette::default())
}

#[test]
fn test_status_bar_vm_default_mode() {
    let state = make_state();
    let vms = build_vms(&state);
    assert_eq!(vms.status_bar.mode, TuiMode::Chat);
}

#[test]
fn test_status_bar_vm_chat_mode() {
    let mut state = make_state();
    state.mode = TuiMode::Chat;
    let vms = build_vms(&state);
    assert_eq!(vms.status_bar.mode, TuiMode::Chat);
}

#[test]
fn test_status_bar_vm_permission_mode() {
    let mut state = make_state();
    state.mode = TuiMode::Permission;
    let vms = build_vms(&state);
    assert_eq!(vms.status_bar.mode, TuiMode::Permission);
}

#[test]
fn test_status_bar_vm_command_palette_mode() {
    let mut state = make_state();
    state.mode = TuiMode::CommandPalette;
    let vms = build_vms(&state);
    assert_eq!(vms.status_bar.mode, TuiMode::CommandPalette);
}

#[test]
fn test_status_bar_vm_diff_viewer_mode() {
    let mut state = make_state();
    state.mode = TuiMode::DiffViewer;
    let vms = build_vms(&state);
    assert_eq!(vms.status_bar.mode, TuiMode::DiffViewer);
}

#[test]
fn test_status_bar_vm_session_tree_mode() {
    let mut state = make_state();
    state.mode = TuiMode::SessionTree;
    let vms = build_vms(&state);
    assert_eq!(vms.status_bar.mode, TuiMode::SessionTree);
}

#[test]
fn test_status_bar_vm_onboarding_mode() {
    let mut state = make_state();
    state.mode = TuiMode::Onboarding;
    let vms = build_vms(&state);
    assert_eq!(vms.status_bar.mode, TuiMode::Onboarding);
}

#[test]
fn test_status_bar_vm_overlay_mode() {
    let mut state = make_state();
    state.mode = TuiMode::Overlay;
    let vms = build_vms(&state);
    assert_eq!(vms.status_bar.mode, TuiMode::Overlay);
}

#[test]
fn test_status_bar_vm_select_mode() {
    let mut state = make_state();
    state.mode = TuiMode::Select;
    let vms = build_vms(&state);
    assert_eq!(vms.status_bar.mode, TuiMode::Select);
}

#[test]
fn test_status_bar_vm_with_model() {
    let mut state = make_state();
    state.current_model = Some("anthropic/claude-sonnet-4".to_string());
    let vms = build_vms(&state);
    assert_eq!(vms.status_bar.current_model, Some("anthropic/claude-sonnet-4".to_string()));
}

#[test]
fn test_status_bar_vm_token_usage() {
    let mut state = make_state();
    state.session_token_usage = TokenUsage {
        prompt_tokens: 100,
        completion_tokens: 200,
        total_tokens: 300,
        estimated_cost: 0.05,
    };
    let vms = build_vms(&state);
    assert_eq!(vms.status_bar.session_token_usage.total_tokens, 300);
    assert_eq!(vms.status_bar.session_token_usage.estimated_cost, 0.05);
}

#[test]
fn test_status_bar_vm_background_jobs() {
    let mut state = make_state();
    state.background_jobs = vec![
        BackgroundJob { name: "Job1".to_string(), status: JobStatus::Running },
        BackgroundJob { name: "Job2".to_string(), status: JobStatus::Complete },
    ];
    let vms = build_vms(&state);
    assert_eq!(vms.status_bar.background_jobs.len(), 2);
}

#[test]
fn test_status_bar_vm_agent_running() {
    let mut state = make_state();
    state.agent_running = true;
    let vms = build_vms(&state);
    assert!(vms.status_bar.agent_running);
}

#[test]
fn test_status_bar_vm_agent_not_running() {
    let state = make_state();
    let vms = build_vms(&state);
    assert!(!vms.status_bar.agent_running);
}

#[test]
fn test_status_bar_vm_braille_frame() {
    let mut state = make_state();
    state.animation.braille_frame = 5;
    let vms = build_vms(&state);
    assert_eq!(vms.status_bar.braille_frame, 5);
}

#[test]
fn test_status_bar_vm_braille_frame_zero_default() {
    let state = make_state();
    let vms = build_vms(&state);
    assert_eq!(vms.status_bar.braille_frame, 0);
}
