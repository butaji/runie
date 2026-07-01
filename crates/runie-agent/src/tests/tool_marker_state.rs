//! Regression: captured agent events applied to AppState must not leave raw
//! legacy TOOL: markers in assistant messages.

use crate::tests::ensure_mock_provider;
use crate::{agent_command_builder::agent_cmd, run_agent_turn_with_skills};
use runie_core::message::Role;
use runie_testing::{allow_all_gate, capture_events, mock_provider, mock_tool_skill};

#[tokio::test]
async fn agent_turn_state_no_raw_tool_markers() {
    let _mock_guard = ensure_mock_provider().await;
    let provider = mock_provider();
        let cmd = agent_cmd("list files").build();
    let (events, emit) = capture_events();
    run_agent_turn_with_skills(
        &provider,
        &cmd,
        emit,
        5,
        Some(&mock_tool_skill()),
        allow_all_gate(),
    )
    .await
    .unwrap();

    let mut state = runie_core::AppState::default();
    let config = runie_core::config::Config::default();
    state.apply_config(&config);
    for evt in events.lock().drain(..) {
        state.update(evt);
    }

    let bad: Vec<String> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::Assistant)
        .map(|m| m.content())
        .filter(|c| c.contains("TOOL:"))
        .collect();
    assert!(
        bad.is_empty(),
        "assistant messages should not contain raw TOOL: markers: {:?}",
        bad
    );

    let assistants: Vec<String> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::Assistant)
        .map(|m| m.content())
        .collect();
    assert!(
        assistants.iter().any(|c| c.contains("Done.")),
        "final assistant response should be present, got {:?}",
        assistants
    );
}
