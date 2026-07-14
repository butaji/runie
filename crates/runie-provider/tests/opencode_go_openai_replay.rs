//! Replay OpenCode Go OpenAI-compatible SSE fixtures.
//!
//! Fixtures live in the parent `runie-tests` repo under `fixtures/openai/`.
//! When `runie` is checked out standalone these tests skip if the fixtures are
//! not available.

use std::path::PathBuf;

use runie_core::provider_event::{ProviderEvent, StopReason};
use runie_provider::openai::stream::replay_sse;

fn fixture(name: &str) -> Option<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("fixtures")
        .join("openai")
        .join(name);
    std::fs::read_to_string(&path).ok()
}

#[test]
fn deepseek_v4_flash_simple_emits_text() {
    let Some(text) = fixture("opencode_go_deepseek_v4_flash_simple.sse") else {
        return;
    };
    let events = replay_sse(&text);
    assert!(events
        .iter()
        .any(|e| matches!(e, ProviderEvent::TextDelta(_))));
    assert!(events.iter().any(|e| matches!(
        e,
        ProviderEvent::Finish {
            reason: StopReason::Stop
        }
    )));
}

#[test]
fn deepseek_v4_flash_tool_emits_tool_call() {
    let Some(text) = fixture("opencode_go_deepseek_v4_flash_tool.sse") else {
        return;
    };
    let events = replay_sse(&text);
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
fn deepseek_v4_flash_multi_tool_emits_multiple_tools() {
    let Some(text) = fixture("opencode_go_deepseek_v4_flash_multi_tool.sse") else {
        return;
    };
    let events = replay_sse(&text);
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
fn deepseek_v4_flash_reasoning_emits_thinking() {
    let Some(text) = fixture("opencode_go_deepseek_v4_flash_reasoning.sse") else {
        return;
    };
    let events = replay_sse(&text);
    assert!(events
        .iter()
        .any(|e| matches!(e, ProviderEvent::ThinkingDelta(_))));
    assert!(events
        .iter()
        .any(|e| matches!(e, ProviderEvent::TextDelta(_))));
}

#[test]
fn kimi_k2_6_simple_emits_content() {
    let Some(text) = fixture("opencode_go_kimi_k2_6_simple.sse") else {
        return;
    };
    let events = replay_sse(&text);
    assert!(events.iter().any(|e| {
        matches!(e, ProviderEvent::TextDelta(_)) || matches!(e, ProviderEvent::ThinkingDelta(_))
    }));
}

#[test]
fn glm_5_2_simple_emits_content() {
    let Some(text) = fixture("opencode_go_glm_5_2_simple.sse") else {
        return;
    };
    let events = replay_sse(&text);
    assert!(events.iter().any(|e| {
        matches!(e, ProviderEvent::TextDelta(_)) || matches!(e, ProviderEvent::ThinkingDelta(_))
    }));
}
