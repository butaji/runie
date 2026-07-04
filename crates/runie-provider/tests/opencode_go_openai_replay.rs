//! Replay OpenCode Go OpenAI-compatible SSE fixtures.

use runie_core::provider_event::{ProviderEvent, StopReason};
use runie_provider::openai::stream::replay_sse;
use runie_testing::fixtures::openai::fixture;

#[test]
fn deepseek_v4_flash_simple_emits_text() {
    let events = replay_sse(&fixture("opencode_go_deepseek_v4_flash_simple.sse"));
    assert!(events.iter().any(|e| matches!(e, ProviderEvent::TextDelta(_))));
    assert!(events.iter().any(|e| matches!(
        e,
        ProviderEvent::Finish {
            reason: StopReason::Stop
        }
    )));
}

#[test]
fn deepseek_v4_flash_tool_emits_tool_call() {
    let events = replay_sse(&fixture("opencode_go_deepseek_v4_flash_tool.sse"));
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
    let events = replay_sse(&fixture("opencode_go_deepseek_v4_flash_multi_tool.sse"));
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
    let events = replay_sse(&fixture("opencode_go_deepseek_v4_flash_reasoning.sse"));
    assert!(events.iter().any(|e| matches!(e, ProviderEvent::ThinkingDelta(_))));
    assert!(events.iter().any(|e| matches!(e, ProviderEvent::TextDelta(_))));
}

#[test]
fn kimi_k2_6_simple_emits_content() {
    let events = replay_sse(&fixture("opencode_go_kimi_k2_6_simple.sse"));
    assert!(events.iter().any(|e| {
        matches!(e, ProviderEvent::TextDelta(_))
            || matches!(e, ProviderEvent::ThinkingDelta(_))
    }));
}

#[test]
fn glm_5_2_simple_emits_content() {
    let events = replay_sse(&fixture("opencode_go_glm_5_2_simple.sse"));
    assert!(events.iter().any(|e| {
        matches!(e, ProviderEvent::TextDelta(_))
            || matches!(e, ProviderEvent::ThinkingDelta(_))
    }));
}
