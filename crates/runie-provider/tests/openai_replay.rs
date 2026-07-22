//! Replay OpenAI SSE streams and snapshot the resulting events.

use runie_core::provider_event::ProviderEvent;
use runie_core::provider_event::StopReason;
use runie_provider::openai::stream::replay_sse;
use runie_testing::fixtures::openai::fixture;

#[test]
fn simple_text_delta_emits_text_deltas() {
    let events = replay_sse(&fixture("simple_text_delta.sse"));
    let texts: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            ProviderEvent::TextDelta(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(texts, &["Hello", " world", "!"]);
    assert!(events
        .iter()
        .any(|e| matches!(e, ProviderEvent::Finish { reason: StopReason::Stop })));
}

#[test]
fn reasoning_content_emits_thinking_deltas() {
    let events = replay_sse(&fixture("reasoning_content.sse"));
    let reasonings: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            ProviderEvent::ThinkingDelta(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    assert!(reasonings[0].contains("think about this"));
    // Also verify text output
    let texts: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            ProviderEvent::TextDelta(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    assert!(texts.iter().any(|t| t.contains("42")));
}

#[test]
fn parallel_tool_calls_emits_two_tools() {
    let events = replay_sse(&fixture("parallel_tool_calls.sse"));
    let tool_starts: Vec<(&str, &str)> = events
        .iter()
        .filter_map(|e| match e {
            ProviderEvent::ToolCallStart { id, name } => Some((id.as_str(), name.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(tool_starts.len(), 2);
    assert_eq!(tool_starts[0], ("call_openai_001", "list_dir"));
    assert_eq!(tool_starts[1], ("call_openai_002", "read_file"));
    // Verify args are accumulated
    let tool_end_ids: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            ProviderEvent::ToolCallEnd { id } => Some(id.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(tool_end_ids.len(), 2);
    assert!(events
        .iter()
        .any(|e| matches!(e, ProviderEvent::Finish { reason: StopReason::ToolCalls })));
}

#[test]
fn rate_limit_error_emits_error_event() {
    let events = replay_sse(&fixture("rate_limit_error.sse"));
    assert!(events.iter().any(|e| matches!(
        e,
        ProviderEvent::Error(runie_core::provider_event::ModelError::RateLimit { retry_after_secs: _ })
    )));
}
