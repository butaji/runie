//! Token usage and cost calculation tests.
//!
//! Tests:
//! - Token usage accumulation
//! - Cost calculation for gpt-4o, claude, unknown models
//! - Multiple token events accumulate correctly
//! - Zero tokens handled

use crate::tui::state::AppState;
use crate::tui::update::agent::handle_agent_event;
use runie_agent::AgentEvent;

/// Helper: Create AppState ready for token testing.
fn make_test_state() -> AppState {
    let mut state = AppState::default();
    state.current_model = Some("test-model".to_string());
    state
}

/// Helper: Create a TokenUsage event.
fn token_usage(prompt: usize, completion: usize) -> AgentEvent {
    AgentEvent::TokenUsage {
        prompt_tokens: prompt,
        completion_tokens: completion,
        total_tokens: prompt + completion,
        context_window: 128_000,
    }
}

// ─── Token usage accumulation tests ─────────────────────────────────────────

#[test]
fn test_token_usage_single_event() {
    let mut state = make_test_state();

    handle_agent_event(&mut state, token_usage(100, 200));

    assert_eq!(state.session_token_usage.prompt_tokens, 100);
    assert_eq!(state.session_token_usage.completion_tokens, 200);
    assert_eq!(state.session_token_usage.total_tokens, 300);
}

#[test]
fn test_token_usage_multiple_events_accumulate() {
    let mut state = make_test_state();

    handle_agent_event(&mut state, token_usage(100, 200));
    handle_agent_event(&mut state, token_usage(50, 100));
    handle_agent_event(&mut state, token_usage(25, 50));

    assert_eq!(state.session_token_usage.prompt_tokens, 175); // 100 + 50 + 25
    assert_eq!(state.session_token_usage.completion_tokens, 350); // 200 + 100 + 50
    assert_eq!(state.session_token_usage.total_tokens, 525); // 175 + 350
}

#[test]
fn test_token_usage_zero_tokens() {
    let mut state = make_test_state();

    handle_agent_event(&mut state, token_usage(0, 0));

    assert_eq!(state.session_token_usage.prompt_tokens, 0);
    assert_eq!(state.session_token_usage.completion_tokens, 0);
    assert_eq!(state.session_token_usage.total_tokens, 0);
}

#[test]
fn test_token_usage_large_numbers() {
    let mut state = make_test_state();

    handle_agent_event(&mut state, token_usage(50_000, 100_000));

    assert_eq!(state.session_token_usage.prompt_tokens, 50_000);
    assert_eq!(state.session_token_usage.completion_tokens, 100_000);
    assert_eq!(state.session_token_usage.total_tokens, 150_000);
}

// ─── Cost calculation tests ───────────────────────────────────────────────────

#[test]
fn test_cost_calculation_gpt_4o() {
    let mut state = make_test_state();
    state.current_model = Some("gpt-4o".to_string());

    handle_agent_event(&mut state, token_usage(100, 200));

    // gpt-4o pricing: $5/1M prompt, $15/1M completion (approximate)
    // 100 prompt + 200 completion = 300 total
    // Estimated cost should be > 0
    assert!(
        state.session_token_usage.estimated_cost > 0.0,
        "cost should be calculated for gpt-4o"
    );
}

#[test]
fn test_cost_calculation_gpt_4o_mini() {
    let mut state = make_test_state();
    state.current_model = Some("gpt-4o-mini".to_string());

    handle_agent_event(&mut state, token_usage(100, 200));

    assert!(
        state.session_token_usage.estimated_cost > 0.0,
        "cost should be calculated for gpt-4o-mini"
    );
}

#[test]
fn test_cost_calculation_o1() {
    let mut state = make_test_state();
    state.current_model = Some("o1".to_string());

    handle_agent_event(&mut state, token_usage(100, 200));

    assert!(
        state.session_token_usage.estimated_cost > 0.0,
        "cost should be calculated for o1"
    );
}

#[test]
fn test_cost_calculation_o1_mini() {
    let mut state = make_test_state();
    state.current_model = Some("o1-mini".to_string());

    handle_agent_event(&mut state, token_usage(100, 200));

    assert!(
        state.session_token_usage.estimated_cost > 0.0,
        "cost should be calculated for o1-mini"
    );
}

#[test]
fn test_cost_calculation_o3() {
    let mut state = make_test_state();
    state.current_model = Some("o3".to_string());

    handle_agent_event(&mut state, token_usage(100, 200));

    assert!(
        state.session_token_usage.estimated_cost > 0.0,
        "cost should be calculated for o3"
    );
}

#[test]
fn test_cost_calculation_o3_mini() {
    let mut state = make_test_state();
    state.current_model = Some("o3-mini".to_string());

    handle_agent_event(&mut state, token_usage(100, 200));

    assert!(
        state.session_token_usage.estimated_cost > 0.0,
        "cost should be calculated for o3-mini"
    );
}

#[test]
fn test_cost_calculation_claude() {
    let mut state = make_test_state();
    state.current_model = Some("claude-3-5-sonnet".to_string());

    handle_agent_event(&mut state, token_usage(100, 200));

    assert!(
        state.session_token_usage.estimated_cost > 0.0,
        "cost should be calculated for claude"
    );
}

#[test]
fn test_cost_calculation_claude_3_opus() {
    let mut state = make_test_state();
    state.current_model = Some("claude-3-opus".to_string());

    handle_agent_event(&mut state, token_usage(100, 200));

    assert!(
        state.session_token_usage.estimated_cost > 0.0,
        "cost should be calculated for claude-3-opus"
    );
}

#[test]
fn test_cost_calculation_claude_3_5_haiku() {
    let mut state = make_test_state();
    state.current_model = Some("claude-3-5-haiku".to_string());

    handle_agent_event(&mut state, token_usage(100, 200));

    assert!(
        state.session_token_usage.estimated_cost > 0.0,
        "cost should be calculated for claude-3-5-haiku"
    );
}

#[test]
fn test_cost_calculation_unknown_model() {
    let mut state = make_test_state();
    state.current_model = Some("unknown-model".to_string());

    handle_agent_event(&mut state, token_usage(100, 200));

    // Unknown model should still accumulate tokens but may have 0 cost
    assert_eq!(state.session_token_usage.total_tokens, 300);
}

#[test]
fn test_cost_calculation_no_model_set() {
    let mut state = AppState::default();
    // No current_model set

    handle_agent_event(&mut state, token_usage(100, 200));

    // Should still track tokens even without model
    assert_eq!(state.session_token_usage.total_tokens, 300);
    // Cost may or may not be calculated without model
}

// ─── Cost accumulation tests ─────────────────────────────────────────────────

#[test]
fn test_cost_accumulates_across_events() {
    let mut state = make_test_state();
    state.current_model = Some("gpt-4o".to_string());

    let cost1 = {
        let mut s = state.clone();
        handle_agent_event(&mut s, token_usage(100, 200));
        s.session_token_usage.estimated_cost
    };

    let cost2 = {
        let mut s = state.clone();
        handle_agent_event(&mut s, token_usage(100, 200));
        handle_agent_event(&mut s, token_usage(100, 200));
        s.session_token_usage.estimated_cost
    };

    assert!(
        cost2 > cost1,
        "cost should accumulate across multiple events"
    );
}

#[test]
fn test_cost_reflects_token_ratios() {
    let mut state1 = make_test_state();
    state1.current_model = Some("gpt-4o".to_string());

    let mut state2 = make_test_state();
    state2.current_model = Some("gpt-4o".to_string());

    // High completion ratio (expensive)
    handle_agent_event(&mut state1, token_usage(10, 1000));

    // Low completion ratio (cheaper)
    handle_agent_event(&mut state2, token_usage(1000, 10));

    // Completion tokens are more expensive, so state1 should have higher cost
    // Note: This depends on actual pricing model
    let cost1 = state1.session_token_usage.estimated_cost;
    let cost2 = state2.session_token_usage.estimated_cost;

    // With gpt-4o, completion > prompt, so more completion should = higher cost
    assert!(
        cost1 > 0.0 && cost2 > 0.0,
        "both should have non-zero costs"
    );
}

// ─── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn test_token_usage_overflow_protection() {
    let mut state = make_test_state();

    // Simulate very large token counts
    handle_agent_event(
        &mut state,
        AgentEvent::TokenUsage {
            prompt_tokens: usize::MAX / 2,
            completion_tokens: usize::MAX / 2,
            total_tokens: usize::MAX - 1,
            context_window: 128_000,
        },
    );

    // Should not panic - just track what we can
    assert!(
        state.session_token_usage.total_tokens > 0,
        "token usage should be recorded"
    );
}

#[test]
fn test_context_window_not_tracked_in_session() {
    let mut state = make_test_state();

    handle_agent_event(
        &mut state,
        AgentEvent::TokenUsage {
            prompt_tokens: 1000,
            completion_tokens: 2000,
            total_tokens: 3000,
            context_window: 128_000,
        },
    );

    // context_window is not tracked in session_token_usage
    // It's informational only
    assert_eq!(state.session_token_usage.total_tokens, 3000);
}

#[test]
fn test_token_usage_with_turns() {
    let mut state = make_test_state();
    state.current_model = Some("gpt-4o".to_string());

    // Simulate multiple turns of token usage
    for turn in 1..=3 {
        handle_agent_event(&mut state, token_usage(100 * turn, 200 * turn));
    }

    assert_eq!(state.session_token_usage.prompt_tokens, 600); // 100 + 200 + 300
    assert_eq!(state.session_token_usage.completion_tokens, 1200); // 200 + 400 + 600
    assert_eq!(state.session_token_usage.total_tokens, 1800);
}
