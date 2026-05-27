// ============================================================================
// View Model Builder Tests - Agent List
// ============================================================================

use crate::components::status_bar::{BackgroundJob, JobStatus};
use crate::components::message_list::PlanStatus;
use crate::components::MessageItem;
use crate::tui::state::AppState;
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
fn test_agent_list_vm_empty_plan_steps() {
    let state = make_state();
    let vms = build_vms(&state);
    assert!(vms.agent_list.plan_steps.is_empty());
}

#[test]
fn test_agent_list_vm_with_plan_steps() {
    let mut state = make_state();
    state.messages = std::sync::Arc::new([
        MessageItem::PlanStep {
            step: 1,
            text: "Step 1: Read file".to_string(),
            status: PlanStatus::Pending,
        },
        MessageItem::PlanStep {
            step: 2,
            text: "Step 2: Edit file".to_string(),
            status: PlanStatus::Active,
        },
        MessageItem::PlanStep {
            step: 3,
            text: "Step 3: Write file".to_string(),
            status: PlanStatus::Complete,
        },
    ]);
    let vms = build_vms(&state);
    assert_eq!(vms.agent_list.plan_steps.len(), 3);
    assert_eq!(vms.agent_list.plan_steps[0].0, 1);
    assert_eq!(vms.agent_list.plan_steps[0].2, PlanStatus::Pending);
    assert_eq!(vms.agent_list.plan_steps[1].0, 2);
    assert_eq!(vms.agent_list.plan_steps[1].2, PlanStatus::Active);
    assert_eq!(vms.agent_list.plan_steps[2].0, 3);
    assert_eq!(vms.agent_list.plan_steps[2].2, PlanStatus::Complete);
}

#[test]
fn test_agent_list_vm_running_jobs() {
    let mut state = make_state();
    state.background_jobs = vec![
        BackgroundJob { name: "Running Job".to_string(), status: JobStatus::Running },
        BackgroundJob { name: "Complete Job".to_string(), status: JobStatus::Complete },
        BackgroundJob { name: "Failed Job".to_string(), status: JobStatus::Failed },
    ];
    let vms = build_vms(&state);
    assert_eq!(vms.agent_list.running_jobs.len(), 1);
    assert_eq!(vms.agent_list.running_jobs[0].name, "Running Job");
    assert_eq!(vms.agent_list.active_count, 1);
}

#[test]
fn test_agent_list_vm_no_running_jobs() {
    let mut state = make_state();
    state.background_jobs = vec![
        BackgroundJob { name: "Complete Job".to_string(), status: JobStatus::Complete },
        BackgroundJob { name: "Failed Job".to_string(), status: JobStatus::Failed },
    ];
    let vms = build_vms(&state);
    assert!(vms.agent_list.running_jobs.is_empty());
    assert_eq!(vms.agent_list.active_count, 0);
}

#[test]
fn test_agent_list_vm_tokens() {
    let mut state = make_state();
    state.session_token_usage = TokenUsage {
        prompt_tokens: 100,
        completion_tokens: 200,
        total_tokens: 300,
        estimated_cost: 0.05,
    };
    let vms = build_vms(&state);
    assert_eq!(vms.agent_list.tokens, 300);
    assert_eq!(vms.agent_list.cost, 0.05);
}

#[test]
fn test_agent_list_vm_agent_running() {
    let mut state = make_state();
    state.agent_running = true;
    let vms = build_vms(&state);
    assert!(vms.agent_list.agent_running);
}

#[test]
fn test_agent_list_vm_non_plan_messages_not_included() {
    let mut state = make_state();
    state.messages = std::sync::Arc::new([
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
        MessageItem::PlanStep {
            step: 1,
            text: "Plan step".to_string(),
            status: PlanStatus::Pending,
        },
    ]);
    let vms = build_vms(&state);
    assert_eq!(vms.agent_list.plan_steps.len(), 1);
    assert_eq!(vms.agent_list.plan_steps[0].0, 1);
}

#[test]
fn test_agent_list_vm_multiple_running_jobs() {
    let mut state = make_state();
    state.background_jobs = vec![
        BackgroundJob { name: "Running 1".to_string(), status: JobStatus::Running },
        BackgroundJob { name: "Running 2".to_string(), status: JobStatus::Running },
        BackgroundJob { name: "Complete".to_string(), status: JobStatus::Complete },
    ];
    let vms = build_vms(&state);
    assert_eq!(vms.agent_list.running_jobs.len(), 2);
    assert_eq!(vms.agent_list.active_count, 2);
}

#[test]
fn test_agent_list_vm_plan_step_text_preserved() {
    let mut state = make_state();
    state.messages = std::sync::Arc::new([
        MessageItem::PlanStep {
            step: 1,
            text: "Run tests".to_string(),
            status: PlanStatus::Active,
        },
    ]);
    let vms = build_vms(&state);
    assert_eq!(vms.agent_list.plan_steps[0].1, "Run tests");
}
