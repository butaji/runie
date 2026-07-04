//! Replay OpenCode Go Anthropic-compatible SSE fixtures.

use runie_core::provider_event::{ProviderEvent, StopReason};
use runie_provider::anthropic::replay_anthropic_sse;
use runie_testing::fixtures::anthropic::fixture;

#[test]
fn minimax_m3_simple_emits_text() {
    let events = replay_anthropic_sse(&fixture("opencode_go_minimax_m3_simple.sse"));
    assert!(events.iter().any(|e| matches!(e, ProviderEvent::TextDelta(_))));
    assert!(events.iter().any(|e| matches!(
        e,
        ProviderEvent::Finish {
            reason: StopReason::Stop
        }
    )));
}

#[test]
fn minimax_m3_tool_emits_tool_call() {
    let events = replay_anthropic_sse(&fixture("opencode_go_minimax_m3_tool.sse"));
    assert!(events.iter().any(|e| matches!(
        e,
        ProviderEvent::ToolCallStart { name, .. } if name == "get_weather"
    )));
    assert!(events.iter().any(|e| matches!(
        e,
        ProviderEvent::Finish {
            reason: StopReason::ToolCalls
        }
    )));
}

#[test]
fn minimax_m3_multi_tool_emits_multiple_tools() {
    let events = replay_anthropic_sse(&fixture("opencode_go_minimax_m3_multi_tool.sse"));
    let tool_starts: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            ProviderEvent::ToolCallStart { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(tool_starts.len(), 2);
    assert!(tool_starts.iter().all(|n| *n == "get_weather"));
}

#[test]
fn qwen3_7_max_simple_emits_text() {
    let events = replay_anthropic_sse(&fixture("opencode_go_qwen3_7_max_simple.sse"));
    assert!(events.iter().any(|e| matches!(e, ProviderEvent::TextDelta(_))));
}

#[test]
fn qwen3_7_max_reasoning_emits_text() {
    let events = replay_anthropic_sse(&fixture("opencode_go_qwen3_7_max_reasoning.sse"));
    assert!(events.iter().any(|e| matches!(e, ProviderEvent::TextDelta(_))));
}
