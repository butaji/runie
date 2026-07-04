//! Replay OpenCode Go Anthropic-compatible SSE fixtures.
//!
//! Fixtures live in the parent `runie-tests` repo under `fixtures/anthropic/`.
//! When `runie` is checked out standalone these tests skip if the fixtures are
//! not available.

use std::path::PathBuf;

use runie_core::provider_event::{ProviderEvent, StopReason};
use runie_provider::anthropic::replay_anthropic_sse;

fn fixture(name: &str) -> Option<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("fixtures")
        .join("anthropic")
        .join(name);
    std::fs::read_to_string(&path).ok()
}

#[test]
fn minimax_m3_simple_emits_text() {
    let Some(text) = fixture("opencode_go_minimax_m3_simple.sse") else {
        return;
    };
    let events = replay_anthropic_sse(&text);
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
    let Some(text) = fixture("opencode_go_minimax_m3_tool.sse") else {
        return;
    };
    let events = replay_anthropic_sse(&text);
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
    let Some(text) = fixture("opencode_go_minimax_m3_multi_tool.sse") else {
        return;
    };
    let events = replay_anthropic_sse(&text);
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
    let Some(text) = fixture("opencode_go_qwen3_7_max_simple.sse") else {
        return;
    };
    let events = replay_anthropic_sse(&text);
    assert!(events.iter().any(|e| matches!(e, ProviderEvent::TextDelta(_))));
}

#[test]
fn qwen3_7_max_reasoning_emits_text() {
    let Some(text) = fixture("opencode_go_qwen3_7_max_reasoning.sse") else {
        return;
    };
    let events = replay_anthropic_sse(&text);
    assert!(events.iter().any(|e| matches!(e, ProviderEvent::TextDelta(_))));
}
