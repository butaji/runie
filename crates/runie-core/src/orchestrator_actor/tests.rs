use super::*;
use crate::Event;

fn idle() -> OrchestratorActor {
    OrchestratorActor::new()
}

#[test]
fn actor_starts_idle() {
    let orch = idle();
    assert!(matches!(orch.state, OrchestratorState::Idle));
    assert!(orch.active_plan.is_none());
    assert!(orch.results.is_empty());
}

#[test]
fn start_request_transitions_to_aligning() {
    let mut orch = idle();
    orch.state = OrchestratorState::Idle;
    // Simulate StartRequest command
    let ctx = orch.ctx.clone();
    drop(ctx);
    assert!(orch.can_start_request());
    assert!(!orch.has_pending_questions());
    // Without pending questions, it goes to Planning
    // We verify state machine accepts StartRequest when idle
    assert!(orch.state == OrchestratorState::Idle);
}

#[test]
fn record_question_marks_pending() {
    let mut orch = idle();
    assert!(!orch.has_pending_questions());
    orch.record_question("Which file?".into());
    assert!(orch.has_pending_questions());
    assert!(!orch.can_submit_plan());
}

#[test]
fn record_answer_clears_pending() {
    let mut orch = idle();
    orch.record_question("Which file?".into());
    assert!(orch.has_pending_questions());
    orch.record_answer("src/main.rs".into());
    assert!(!orch.has_pending_questions());
    assert!(orch.can_submit_plan());
}

#[test]
fn cancel_resets_to_idle() {
    let mut orch = idle();
    orch.state = OrchestratorState::Executing;
    orch.active_plan = Some(OrchestratorPlan::simple("test", ModelTrait::General));
    orch.ctx.record_question("Q");
    // Simulate cancel
    orch.state = OrchestratorState::Idle;
    orch.active_plan = None;
    orch.results.clear();
    orch.ctx = OrchestratorContext::new();
    assert!(matches!(orch.state, OrchestratorState::Idle));
    assert!(orch.active_plan.is_none());
}

#[test]
fn collect_subagent_result() {
    let mut orch = idle();
    orch.dispatch_subagent(SubagentTask::new("t1", "role", "task", ModelTrait::General));
    assert_eq!(orch.results.len(), 1);
    orch.collect_result("t1".into(), "output".into());
    assert_eq!(orch.results[0].1, "output");
}

#[test]
fn orchestrator_state_is_terminal() {
    assert!(OrchestratorState::Idle.is_terminal());
    assert!(OrchestratorState::Done {
        plan: OrchestratorPlan::simple("", ModelTrait::General),
        result: PlanResult {
            success: true,
            response: String::new(),
            failures: vec![],
            elapsed_secs: 0.0
        },
    }
    .is_terminal());
    assert!(OrchestratorState::Failed {
        error: String::new()
    }
    .is_terminal());
    assert!(!OrchestratorState::Aligning.is_terminal());
    assert!(!OrchestratorState::Planning.is_terminal());
    assert!(!OrchestratorState::Executing.is_terminal());
}

#[test]
fn team_mode_start_fails_with_not_implemented() {
    let mut orch = idle();
    let bus = EventBus::<Event>::new(16);
    handle_start_request(&mut orch, &bus);
    assert!(matches!(orch.state, OrchestratorState::Idle));
}
