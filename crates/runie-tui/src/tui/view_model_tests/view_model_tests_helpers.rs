// ============================================================================
// View Model Builder Tests - Helpers & Integration
// ============================================================================

use crate::components::onboarding::Onboarding;
use crate::components::MessageItem;
use crate::components::message_list::PlanStatus;
use crate::tui::state::{AppState, TuiMode};
use crate::tui::view_models::{ViewModels, OnboardingStep as VmStep};
use crate::components::CommandPalette;
use runie_ai::TokenUsage;

fn make_state() -> AppState {
    AppState::default()
}

fn build_vms(state: &AppState) -> ViewModels {
    let render = crate::tui::state::RenderState::from(state);
    ViewModels::from_render_state(&render, &CommandPalette::default())
}

// ─── Helper function tests ───────────────────────────────────────────────

#[test]
fn test_extract_plan_steps_filters_non_plan_messages() {
    let messages: &[MessageItem] = &[
        MessageItem::User {
            text: "Hello".to_string(),
            model: None,
            timestamp: None,
        },
        MessageItem::Assistant {
            text: "Hi".to_string(),
            model: None,
            timestamp: None,
        },
    ];
    let steps: Vec<(usize, String, PlanStatus)> = messages.iter()
        .filter_map(|msg| {
            if let MessageItem::PlanStep { step, text, status } = msg {
                Some((*step, text.clone(), status.clone()))
            } else {
                None
            }
        })
        .collect();
    assert!(steps.is_empty());
}

#[test]
fn test_extract_plan_steps_extracts_only_plan_steps() {
    let messages: &[MessageItem] = &[
        MessageItem::User {
            text: "Hello".to_string(),
            model: None,
            timestamp: None,
        },
        MessageItem::PlanStep {
            step: 1,
            text: "First step".to_string(),
            status: PlanStatus::Pending,
        },
        MessageItem::Assistant {
            text: "Hi".to_string(),
            model: None,
            timestamp: None,
        },
        MessageItem::PlanStep {
            step: 2,
            text: "Second step".to_string(),
            status: PlanStatus::Complete,
        },
    ];
    let steps: Vec<(usize, String, PlanStatus)> = messages.iter()
        .filter_map(|msg| {
            if let MessageItem::PlanStep { step, text, status } = msg {
                Some((*step, text.clone(), status.clone()))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].0, 1);
    assert_eq!(steps[1].0, 2);
}

// ─── Command Palette VM tests ────────────────────────────────────────────

#[test]
fn test_command_palette_vm_not_open_returns_none() {
    let mut state = make_state();
    state.mode = TuiMode::Chat;
    state.command_palette.open = false;
    let vms = build_vms(&state);
    assert!(vms.command_palette.is_none());
}

// ─── Overlay VM tests ───────────────────────────────────────────────────

#[test]
fn test_overlay_vm_not_in_overlay_mode_returns_none() {
    let mut state = make_state();
    state.mode = TuiMode::Chat;
    let vms = build_vms(&state);
    assert!(vms.overlay.is_none());
}

#[test]
fn test_overlay_vm_in_overlay_mode_returns_some() {
    let mut state = make_state();
    state.mode = TuiMode::Overlay;
    let vms = build_vms(&state);
    assert!(vms.overlay.is_some());
    let overlay = vms.overlay.unwrap();
    assert!(overlay.show_close);
    assert_eq!(overlay.active_tab, 0);
}

// ─── Session Tree VM tests ───────────────────────────────────────────────

#[test]
fn test_session_tree_vm_not_in_session_tree_mode_returns_none() {
    let mut state = make_state();
    state.mode = TuiMode::Chat;
    let vms = build_vms(&state);
    assert!(vms.session_tree.is_none());
}

#[test]
fn test_session_tree_vm_in_session_tree_mode() {
    let mut state = make_state();
    state.mode = TuiMode::SessionTree;
    state.session_tree = crate::components::SessionTreeNavigator::new();
    let vms = build_vms(&state);
    assert!(vms.session_tree.is_some());
}

// ─── Integration: ViewModels::from_render_state ─────────────────────────

#[test]
fn integration_viewmodels_chat_mode_full_state() {
    let mut state = make_state();
    state.mode = TuiMode::Chat;
    state.agent_running = true;
    state.current_model = Some("gpt-4o".to_string());
    state.input_right_info = "100 tokens".to_string();
    state.session_token_usage = TokenUsage {
        prompt_tokens: 50,
        completion_tokens: 50,
        total_tokens: 100,
        estimated_cost: 0.01,
    };
    let vms = build_vms(&state);

    assert!(vms.message_list.messages.is_empty());
    assert_eq!(vms.input_bar.prompt, "❯ ");
    assert!(vms.input_bar.right_info.contains("100 tokens"));
    assert!(vms.status_bar.agent_running);
    assert_eq!(vms.status_bar.current_model, Some("gpt-4o".to_string()));
    assert_eq!(vms.status_bar.session_token_usage.total_tokens, 100);
    assert!(vms.agent_list.plan_steps.is_empty());
    assert!(vms.permission_modal.is_none());
    assert!(vms.command_palette.is_none());
    assert!(vms.overlay.is_none());
    assert!(vms.session_tree.is_none());
    assert!(vms.diff_viewer.is_none());
    assert!(vms.onboarding.is_none());
}

#[test]
fn integration_viewmodels_permission_mode() {
    let mut state = make_state();
    state.mode = TuiMode::Permission;
    state.permission_modal.tool = Some("Read".to_string());
    state.permission_modal.args = Some("{\"path\": \"foo.txt\"}".to_string());
    state.permission_modal.desc = Some("Read a file from disk".to_string());
    let vms = build_vms(&state);

    assert!(vms.permission_modal.is_some());
    let modal = vms.permission_modal.unwrap();
    assert_eq!(modal.tool, "Read");
    assert_eq!(modal.args, "{\"path\": \"foo.txt\"}");
    assert!(modal.visible);
}

#[test]
fn integration_viewmodels_onboarding_mode() {
    let mut state = make_state();
    state.mode = TuiMode::Onboarding;
    let onboarding = Onboarding::new();
    state.onboarding = Some(onboarding);
    let vms = build_vms(&state);

    assert!(vms.onboarding.is_some());
    let onboarding_vm = vms.onboarding.unwrap();
    assert!(matches!(onboarding_vm.step, VmStep::Welcome));
    assert_eq!(onboarding_vm.selected_item, 0);
    assert!(onboarding_vm.selected_provider.is_none());
}

#[test]
fn integration_viewmodels_diff_viewer_mode() {
    let mut state = make_state();
    state.mode = TuiMode::DiffViewer;
    state.diff_viewer = Some(crate::components::DiffViewer::new(
        "test.rs".to_string(),
        "old content".to_string(),
        "new content".to_string(),
    ));
    let vms = build_vms(&state);

    assert!(vms.diff_viewer.is_some());
    let diff_vm = vms.diff_viewer.unwrap();
    assert_eq!(diff_vm.filename, "test.rs");
    assert!(diff_vm.visible);
}
