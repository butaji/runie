//! /workflow slash command tests.

use super::{exec, fresh_state};
use crate::event::CommandEvent;
use crate::orchestrator::ExecutionMode;
use crate::orchestrator_actor::OrchestratorState;

#[test]
fn workflow_parses_and_emits_plan_generated() {
    let mut state = fresh_state();
    exec(&mut state, "/workflow \"analyze src\" as analyzer");

    assert_eq!(state.config.execution_mode, ExecutionMode::Team);
    assert!(
        matches!(state.orchestrator_state, OrchestratorState::Executing),
        "orchestrator should be executing"
    );
    let sys: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == crate::model::Role::System)
        .collect();
    let last = sys.last().expect("system message");
    assert!(
        last.content.contains("Started workflow"),
        "summary message: {}",
        last.content
    );
}

#[test]
fn workflow_invalid_definition_warns() {
    let mut state = fresh_state();
    exec(&mut state, "/workflow");

    assert_eq!(state.config.execution_mode, ExecutionMode::Solo);
    assert!(
        state
            .transient_message
            .as_ref()
            .map(|m| m.contains("Invalid workflow"))
            .unwrap_or(false),
        "expected transient warning: {:?}",
        state.transient_message
    );
}

#[test]
fn workflow_event_contains_tasks() {
    let mut state = fresh_state();
    let cmd = state.registry.get("workflow").expect("registered");
    let result = cmd
        .flow
        .clone()
        .exec(&mut state, "workflow", "\"analyze src\" as analyzer");
    match result {
        crate::commands::CommandResult::Event(CommandEvent::PlanGenerated { plan }) => {
            assert_eq!(plan.tasks.len(), 1);
            assert!(plan.tasks[0].id.starts_with("analyzer-"));
            assert_eq!(plan.tasks[0].task_description, "analyze src");
        }
        other => panic!("expected PlanGenerated event, got: {:?}", other),
    }
}
