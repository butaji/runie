use super::*;
use crate::tui::tests::test_harness::AgentTestHarness;
use runie_agent::{AgentEvent, TokenUsage};

fn make_token_usage(input: usize, output: usize) -> TokenUsage {
    TokenUsage { input, output, cache_read: 0, cache_write: 0, total_tokens: input + output }
}

fn complete_turn(harness: AgentTestHarness, turn: usize, response: &str) -> AgentTestHarness {
    harness
        .handle_agent_event(AgentEvent::MessageStart { message: super::helpers::agent_message("assistant", ""), turn })
        .handle_agent_event(AgentEvent::MessageEnd { message: super::helpers::agent_message("assistant", response), turn })
        .handle_agent_event(AgentEvent::TurnEnd { turn, message_count: 2, tool_results_count: 0, token_usage: make_token_usage(0, 0) })
}

#[test]
fn test_first_turn_token_usage() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");
    harness = complete_turn(harness, 1, "Hi");
    harness.handle_agent_event(AgentEvent::TokenUsage { prompt_tokens: 10, completion_tokens: 5, total_tokens: 15, context_window: 128_000 });
    harness.handle_agent_event(AgentEvent::TurnEnd { turn: 1, message_count: 2, tool_results_count: 0, token_usage: make_token_usage(10, 5) });

    assert_eq!(harness.state.session_token_usage.total_tokens, 15, "first turn total");
    assert_eq!(harness.state.session_token_usage.prompt_tokens, 10, "first turn prompt");
    assert_eq!(harness.state.session_token_usage.completion_tokens, 5, "first turn completion");
}

#[test]
fn test_second_turn_token_usage() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");
    harness = complete_turn(harness, 1, "Hi");
    harness.handle_agent_event(AgentEvent::TokenUsage { prompt_tokens: 10, completion_tokens: 5, total_tokens: 15, context_window: 128_000 });
    harness.handle_agent_event(AgentEvent::TurnEnd { turn: 1, message_count: 2, tool_results_count: 0, token_usage: make_token_usage(10, 5) });

    harness.submit_user_message("How are you?");
    harness = complete_turn(harness, 2, "I'm good");
    harness.handle_agent_event(AgentEvent::TokenUsage { prompt_tokens: 15, completion_tokens: 10, total_tokens: 25, context_window: 128_000 });
    harness.handle_agent_event(AgentEvent::TurnEnd { turn: 2, message_count: 2, tool_results_count: 0, token_usage: make_token_usage(15, 10) });

    assert_eq!(harness.state.session_token_usage.total_tokens, 40, "total tokens: 15 + 25");
    assert_eq!(harness.state.session_token_usage.prompt_tokens, 25, "prompt tokens: 10 + 15");
    assert_eq!(harness.state.session_token_usage.completion_tokens, 15, "completion tokens: 5 + 10");
}
