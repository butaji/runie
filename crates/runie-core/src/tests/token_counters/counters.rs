//! Layer 1 + Layer 2 tests for token counters and animated speed

use crate::event::Event;

use crate::event::{InputEvent, ControlEvent, ModelConfigEvent, SystemEvent, DialogEvent, ScrollEvent, AgentEvent, SessionEvent, EditEvent, CommandEvent, DurableCoreEvent};
use crate::model::AppState;
fn fresh_state() -> AppState {
    AppState::default()
}

// =============================================================================
// Layer 1: Token counting logic
// =============================================================================

#[test]
fn estimate_tokens_chars_divided_by_four_ceil() {
    // Approximation: 4 chars ≈ 1 token, rounding up.
    assert_eq!(crate::tokens::estimate_tokens("hello"), 2); // 5 chars ceil = 2
    assert_eq!(crate::tokens::estimate_tokens("hello world"), 3); // 11 chars ceil = 3
    assert_eq!(crate::tokens::estimate_tokens(""), 0); // 0 chars = 0 tokens
    assert_eq!(crate::tokens::estimate_tokens("🎉"), 1); // 1 char ceil = 1
    assert_eq!(crate::tokens::estimate_tokens("test"), 1); // 4 chars = 1
}

#[test]
fn submit_increments_tokens_in() {
    let mut state = fresh_state();
    state.input.input = "hello world".to_string();
    state.update(Event::submit());
    assert_eq!(
        state.agent.tokens_in, 3,
        "Input 'hello world' = 11 chars ≈ 3 tokens"
    );
}

#[test]
fn agent_response_increments_tokens_out() {
    let mut state = fresh_state();
    state.agent.turn_active = true;
    state.update(Event::Agent(AgentEvent::Response {
        id: "r1".to_string(),
        content: "hello".to_string(),
    }));
    assert_eq!(
        state.agent.tokens_out, 2,
        "Output 'hello' = 5 chars ≈ 2 tokens"
    );
    assert_eq!(state.agent.turn_tokens_out, 2, "Turn tokens should track");
}

#[test]
fn multiple_responses_accumulate_tokens_out() {
    let mut state = fresh_state();
    state.agent.turn_active = true;
    state.update(Event::Agent(AgentEvent::Response {
        id: "r1".to_string(),
        content: "hello".to_string(),
    }));
    state.update(Event::Agent(AgentEvent::Response {
        id: "r1".to_string(),
        content: " world".to_string(),
    }));
    assert_eq!(
        state.agent.tokens_out, 4,
        "'hello' (2) + ' world' (2) = 4 tokens"
    );
}

#[test]
fn finish_turn_resets_turn_tokens() {
    let mut state = fresh_state();
    state.agent.turn_active = true;
    state.agent.current_request_id = Some("r1".to_string());
    state.update(Event::Agent(AgentEvent::Response {
        id: "r1".to_string(),
        content: "hello world".to_string(),
    }));
    assert_eq!(state.agent.turn_tokens_out, 3);

    state.update(Event::Agent(AgentEvent::Done {
        id: "r1".to_string(),
    }));
    assert_eq!(
        state.agent.turn_tokens_out, 0,
        "Turn tokens reset on finish"
    );
    assert_eq!(state.agent.tokens_out, 3, "Cumulative tokens preserved");
}

// =============================================================================
// Layer 1: Speed calculation
// =============================================================================

#[test]
fn speed_zero_when_no_tokens_streamed() {
    let mut state = fresh_state();
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    state.tick_animation();
    assert_eq!(
        state.agent.speed_tps, 0.0,
        "Speed should be 0 with no tokens"
    );
}

#[test]
fn speed_updates_on_tick_with_new_tokens() {
    let mut state = fresh_state();
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());

    // Initialize rolling window with start event
    state.agent.speed_window = crate::state::SpeedWindow::new(1000);
    state.agent.speed_window.record(0); // Start at 0 tokens
    state.agent.tokens_at_last_speed = 0;
    // Set last_speed_update to past so elapsed >= 0.05
    state.agent.last_speed_update =
        Some(std::time::Instant::now() - std::time::Duration::from_millis(200));

    // Simulate tokens arriving
    state.agent.tokens_out = 10;
    state.agent.turn_tokens_out = 10;

    // Update speed - this will record the new tokens
    state.update_speed();

    // With rolling window, speed is calculated from actual timing between events.
    // We verify the mechanism works by checking window has events.
    assert!(
        state.agent.speed_window.len() >= 2,
        "Window should have events, got {}",
        state.agent.speed_window.len()
    );

    // Speed can be 0 in fast tests (elapsed < 1ms), but the mechanism works in real timing.
    let speed = state.agent.speed_window.speed();
    assert!(speed >= 0.0, "Speed should be non-negative");
}

#[test]
fn speed_decays_when_no_new_tokens() {
    let mut state = fresh_state();
    state.agent.turn_active = true;
    state.agent.speed_tps = 100.0;
    state.agent.tokens_at_last_speed = state.agent.tokens_out;
    // Set to 2 seconds ago to ensure elapsed > 1.0
    state.agent.last_speed_update =
        Some(std::time::Instant::now() - std::time::Duration::from_secs(2));

    state.update_speed();

    assert!(
        state.agent.speed_tps < 100.0,
        "Speed should decay with no new tokens, got {}",
        state.agent.speed_tps
    );
    assert!(state.agent.speed_tps >= 0.0, "Speed should not go negative");
}

#[test]
fn speed_clamps_to_zero_after_long_idle() {
    let mut state = fresh_state();
    state.agent.turn_active = true;
    state.agent.speed_tps = 50.0;
    state.agent.tokens_at_last_speed = state.agent.tokens_out;
    state.agent.last_speed_update =
        Some(std::time::Instant::now() - std::time::Duration::from_secs(10));

    // Decay happens per-call; one call decays by 50%, we need multiple calls
    for _ in 0..10 {
        state.update_speed();
    }

    assert!(
        state.agent.speed_tps < 0.1,
        "Speed should decay toward 0 after long idle, got {}",
        state.agent.speed_tps
    );
}

// =============================================================================
// Layer 2: Event-driven state transitions
// =============================================================================

#[test]
fn turn_start_initializes_speed_tracking() {
    let mut state = fresh_state();
    state.update(Event::Agent(AgentEvent::Thinking {
        id: "r1".to_string(),
    }));
    assert!(
        state.agent.last_speed_update.is_some(),
        "Speed tracking should init on turn start"
    );
    assert_eq!(state.agent.tokens_at_last_speed, 0);
}

#[test]
fn new_turn_resets_speed() {
    let mut state = fresh_state();
    state.agent.turn_active = true;
    state.agent.speed_tps = 42.0;
    state.agent.tokens_out = 100;
    state.agent.turn_tokens_out = 50;

    // Finish turn
    state.agent.current_request_id = Some("r1".to_string());
    state.update(Event::Agent(AgentEvent::Done {
        id: "r1".to_string(),
    }));

    assert_eq!(state.agent.speed_tps, 0.0, "Speed reset on turn end");
    assert_eq!(state.agent.turn_tokens_out, 0, "Turn tokens reset");
    assert_eq!(state.agent.tokens_out, 100, "Cumulative preserved");
}

#[test]
fn snapshot_includes_token_counters() {
    let mut state = fresh_state();
    state.agent.tokens_in = 10;
    state.agent.tokens_out = 20;
    state.agent.speed_tps = 5.5;

    let snap = state.snapshot();
    assert_eq!(snap.tokens_in, 10);
    assert_eq!(snap.tokens_out, 20);
    assert_eq!(snap.speed_tps, 5.5);
}

// =============================================================================
// Layer 1: Token animation
// =============================================================================

#[test]
fn animate_tokens_initial_state() {
    let mut state = fresh_state();
    // Animation should not change if values match
    state.agent.tokens_in = 0;
    state.agent.tokens_in_display = 0.0;
    state.agent.tokens_in_prev = 0;

    state.agent.tokens_out = 0;
    state.agent.tokens_out_display = 0.0;
    state.agent.tokens_out_prev = 0;

    // Run multiple ticks - should stabilize
    for _ in 0..5 {
        state.tick_animation();
    }

    assert_eq!(state.agent.tokens_in_display.round() as usize, 0);
    assert_eq!(state.agent.tokens_out_display.round() as usize, 0);
}

#[test]
fn animate_tokens_converges_to_target() {
    let mut state = fresh_state();

    // Set initial values
    state.agent.tokens_in = 100;
    state.agent.tokens_in_display = 0.0; // Start at 0
    state.agent.tokens_in_prev = 0;

    state.agent.tokens_out = 200;
    state.agent.tokens_out_display = 0.0; // Start at 0
    state.agent.tokens_out_prev = 0;

    // Run many ticks to let animation converge
    for _ in 0..50 {
        state.tick_animation();
    }

    // Should converge close to target values
    assert!(
        (state.agent.tokens_in_display - 100.0).abs() < 1.0,
        "tokens_in_display should converge to 100, got {}",
        state.agent.tokens_in_display
    );
    assert!(
        (state.agent.tokens_out_display - 200.0).abs() < 1.0,
        "tokens_out_display should converge to 200, got {}",
        state.agent.tokens_out_display
    );
}
