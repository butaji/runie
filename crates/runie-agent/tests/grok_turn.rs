//! Replay captured Grok Build SSE streams through the headless runner.
//!
//! Grok Build uses an OpenAI-compatible SSE format, so the existing `replay_sse`
//! parser handles Grok fixtures directly. This module provides a `GrokReplayProvider`
//! and tests for the Grok headless replay path.

use runie_agent::{run_headless_turn, HeadlessOptions};
use runie_core::message::ChatMessage;
use runie_core::provider::Provider;
use runie_core::provider_event::ProviderEvent;
use runie_testing::{allow_all_gate, grok_replay_from_fixtures, GrokReplayProvider};

// Layer 4 — E2E: GrokReplayProvider emits text deltas from SSE fixture.
#[tokio::test]
async fn grok_replay_provider_emits_text_deltas() {
    let provider = grok_replay_from_fixtures(&["sample.sse"]);
    let messages = vec![
        ChatMessage::system("You are helpful."),
        ChatMessage::user("say hello"),
    ];
    let options = HeadlessOptions {
        execute_tools: false,
        max_tool_rounds: 5,
        on_chunk: None,
        on_event: None,
        permission_gate: allow_all_gate(),
    };
    let result = run_headless_turn(messages, &provider, options)
        .await
        .unwrap();
    // The sample fixture produces "Hello world" (two content deleltas).
    assert!(result.content.contains("Hello"));
    assert!(result.content.contains("world"));
}

// Layer 4 — E2E: GrokReplayProvider cycles through multiple fixtures.
#[tokio::test]
async fn grok_replay_cycles_fixtures() {
    let provider = grok_replay_from_fixtures(&["sample.sse", "sample.sse"]);
    let messages = vec![
        ChatMessage::system("You are helpful."),
        ChatMessage::user("hello"),
    ];
    let options = HeadlessOptions {
        execute_tools: false,
        max_tool_rounds: 5,
        on_chunk: None,
        on_event: None,
        permission_gate: allow_all_gate(),
    };
    let result = run_headless_turn(messages, &provider, options)
        .await
        .unwrap();
    // Both fixtures produce "Hello world" text
    assert!(result.content.contains("Hello"));
}

// Layer 4 — E2E: GrokReplayProvider constructs without panicking.
#[tokio::test]
async fn grok_replay_provider_construction() {
    let provider = GrokReplayProvider::from_fixture_names(&["sample.sse"]);
    let messages = vec![
        ChatMessage::system("You are helpful."),
        ChatMessage::user("hello"),
    ];
    let options = HeadlessOptions {
        execute_tools: false,
        max_tool_rounds: 5,
        on_chunk: None,
        on_event: None,
        permission_gate: allow_all_gate(),
    };
    let result = run_headless_turn(messages, &provider, options)
        .await
        .unwrap();
    assert!(!result.content.is_empty());
}

// Layer 1 — State/Logic: GrokReplayProvider handles empty fixtures gracefully.
#[tokio::test]
async fn grok_replay_provider_empty_fixtures() {
    let provider = GrokReplayProvider::new(vec![]);
    let messages = vec![
        ChatMessage::system("You are helpful."),
        ChatMessage::user("hello"),
    ];
    let options = HeadlessOptions {
        execute_tools: false,
        max_tool_rounds: 5,
        on_chunk: None,
        on_event: None,
        permission_gate: allow_all_gate(),
    };
    let result = run_headless_turn(messages, &provider, options)
        .await
        .unwrap();
    // Empty fixtures produce no content but should not panic.
    assert!(result.content.is_empty());
}

// Layer 1 — State/Logic: GrokReplayProvider generates stream from fixture.
#[tokio::test]
async fn grok_replay_provider_stream_generates_events() {
    use futures::StreamExt;
    let provider = GrokReplayProvider::from_fixture_names(&["sample.sse"]);
    let messages = vec![ChatMessage::user("hello")];
    let stream = provider.generate(messages);
    let events: Vec<anyhow::Result<ProviderEvent>> = stream.collect().await;
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Ok(ProviderEvent::TextDelta(_)))),
        "expected TextDelta event, got: {events:?}"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Ok(ProviderEvent::Finish { .. }))),
        "expected Finish event, got: {events:?}"
    );
}

// Layer 4 — E2E: grok_replay_from_fixtures wraps in BuiltProvider with correct key/model.
#[tokio::test]
async fn grok_replay_built_provider_has_correct_metadata() {
    let provider = grok_replay_from_fixtures(&["sample.sse"]);
    assert_eq!(provider.key(), "grok");
    assert_eq!(provider.model(), "grok-3");
}

// Layer 4 — E2E: headless result messages include the assistant response.
#[tokio::test]
async fn grok_replay_result_messages_include_response() {
    let provider = grok_replay_from_fixtures(&["sample.sse"]);
    let messages = vec![
        ChatMessage::system("You are helpful."),
        ChatMessage::user("say hello"),
    ];
    let options = HeadlessOptions {
        execute_tools: false,
        max_tool_rounds: 5,
        on_chunk: None,
        on_event: None,
        permission_gate: allow_all_gate(),
    };
    let result = run_headless_turn(messages, &provider, options)
        .await
        .unwrap();
    assert!(result.content.contains("Hello"));
    assert!(result.content.contains("world"));
    // Result includes the system message and user message
    assert!(result.messages.len() >= 2);
    // Assistant message is added at the end
    let last = result.messages.last();
    assert!(
        last.map(|m| m.content().contains("Hello")).unwrap_or(false),
        "expected 'Hello' in last message"
    );
}
