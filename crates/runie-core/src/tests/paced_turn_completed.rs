//! Test that TurnCompleted → clear_turn_state produces a snapshot with
//! turn_active=true so the streamed response text is rendered.

use crate::dsl::AppStateDsl;
use crate::model::AppState;

#[test]
fn turn_completed_keeps_turn_active_for_snapshot() {
    let mut state = AppState::default();

    // Simulate the full turn lifecycle:
    // 1. Turn starts
    state.apply_turn_started();
    assert!(
        state.agent_state().turn_active,
        "turn_active should be true after TurnStarted"
    );

    // 2. Stream some response text
    state.update(crate::Event::ResponseDelta {
        id: "req.0".into(),
        content: "say hello\n".into(),
    });

    // 3. Turn completes — apply_turn_completed should NOT clear turn_active
    state.apply_turn_completed();
    assert!(
        state.agent_state().turn_active,
        "turn_active must stay true after TurnCompleted for final snapshot"
    );
    assert!(
        !state.agent_state().streaming,
        "streaming should be false"
    );
}
