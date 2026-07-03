//! Tests for the headless runner.

use crate::headless::{run_headless_cli, run_headless_turn, HeadlessCliOptions, HeadlessOptions};
use crate::PermissionGate;
use runie_core::event::headless::HeadlessEvent;
use runie_core::message::{ChatMessage, Role};
use runie_core::permissions::{AutoAllowSink, PermissionManager};
use runie_core::provider::Provider;
use runie_core::provider_event::{ModelError, ProviderEvent};
use runie_provider::MockProvider;
use runie_testing::allow_all_gate;
use std::sync::Arc;

#[tokio::test]
async fn headless_runner_with_mock_returns_content() {
    let provider = MockProvider::default();
    let messages = vec![
        ChatMessage::system("You are helpful."),
        ChatMessage::user("hello world"),
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
    assert!(result.tool_outputs.is_empty());
}

#[tokio::test]
async fn headless_runner_executes_tool_and_returns_output() {
    let _mock_guard = crate::tests::ensure_mock_provider().await;
    let provider = MockProvider::default();
    let messages = vec![
        ChatMessage::system("You are helpful."),
        ChatMessage::user("list files"),
    ];
    let options = HeadlessOptions {
        execute_tools: true,
        max_tool_rounds: 5,
        on_chunk: None,
        on_event: None,
        permission_gate: allow_all_gate(),
    };
    let result = run_headless_turn(messages, &provider, options)
        .await
        .unwrap();
    assert!(!result.content.is_empty());
    assert_eq!(result.tool_outputs.len(), 1);
    assert_eq!(result.tool_outputs[0].tool_name, "list_dir");
    assert!(result.tool_outputs[0].tool_args.get("path").is_some());
    assert!(!result.tool_outputs[0].content.is_empty());
}

#[tokio::test]
async fn headless_runner_with_execute_tools_enabled() {
    let _mock_guard = crate::tests::ensure_mock_provider().await;
    let provider = MockProvider::default();
    let messages = vec![
        ChatMessage::system("You are helpful."),
        ChatMessage::user("list files"),
    ];
    let options = HeadlessOptions {
        execute_tools: true,
        max_tool_rounds: 5,
        on_chunk: None,
        on_event: None,
        permission_gate: allow_all_gate(),
    };
    let result = run_headless_turn(messages, &provider, options)
        .await
        .unwrap();
    assert!(!result.tool_outputs.is_empty());
}

#[tokio::test]
async fn headless_runner_feeds_parse_errors_back_to_model() {
    let _mock_guard = crate::tests::ensure_mock_provider().await;
    let provider = MockProvider::default();
    let messages = vec![
        ChatMessage::system("You are helpful."),
        ChatMessage::user("malformed tool call"),
    ];
    let options = HeadlessOptions {
        execute_tools: true,
        max_tool_rounds: 5,
        on_chunk: None,
        on_event: None,
        permission_gate: allow_all_gate(),
    };
    let result = run_headless_turn(messages, &provider, options)
        .await
        .unwrap();

    assert!(
        result.tool_outputs.is_empty(),
        "malformed tool should not be executed"
    );
    let has_parse_error = result
        .messages
        .iter()
        .any(|m| m.role == Role::Tool && m.content().contains("Could not parse tool call"));
    assert!(has_parse_error, "parse error should be added to messages");
}

#[tokio::test]
async fn headless_runner_executes_tool_call_markup() {
    let _mock_guard = crate::tests::ensure_mock_provider().await;
    let provider = MockProvider::default();
    let messages = vec![
        ChatMessage::system("You are helpful."),
        ChatMessage::user("use markup tool call"),
    ];
    let options = HeadlessOptions {
        execute_tools: true,
        max_tool_rounds: 5,
        on_chunk: None,
        on_event: None,
        permission_gate: allow_all_gate(),
    };
    let result = run_headless_turn(messages, &provider, options)
        .await
        .unwrap();

    assert_eq!(result.tool_outputs.len(), 1);
    assert_eq!(result.tool_outputs[0].tool_name, "list_dir");
    assert!(result.tool_outputs[0].tool_args.get("path").is_some());
    assert!(result.content.contains("[TOOL_CALL]"));
}

// Layer 1 — State/Logic: helper constructs a PermissionGate with the supplied sink.
#[tokio::test]
async fn headless_cli_helper_builds_gate() {
    let sink: Arc<dyn runie_core::permissions::ApprovalSink> = Arc::new(AutoAllowSink);
    let _opts = HeadlessCliOptions {
        execute_tools: true,
        max_tool_rounds: 5,
        on_chunk: None,
        on_event: None,
    };
    let gate = PermissionGate::new(PermissionManager::default(), sink.clone());
    assert!(Arc::ptr_eq(gate.sink_ref(), &sink));
}

// Layer 4 — Smoke: run_headless_cli still works with a mock provider.
#[tokio::test]
async fn headless_cli_smoke_with_mock() {
    let _mock_guard = crate::tests::ensure_mock_provider().await;
    let sink: Arc<dyn runie_core::permissions::ApprovalSink> = Arc::new(AutoAllowSink);
    let messages = vec![
        ChatMessage::system("You are helpful."),
        ChatMessage::user("hello"),
    ];
    let opts = HeadlessCliOptions {
        execute_tools: false,
        max_tool_rounds: 5,
        on_chunk: None,
        on_event: None,
    };
    let result = run_headless_cli(Some("mock"), Some("echo"), messages, sink, opts, None)
        .await
        .unwrap();
    assert!(!result.content.is_empty());
}

// Layer 4: provider error in a headless turn is still reported
#[tokio::test]
async fn headless_turn_error_propagates() {
    struct ErrorProvider;
    impl Provider for ErrorProvider {
        fn generate(
            &self,
            _messages: Vec<ChatMessage>,
        ) -> std::pin::Pin<Box<dyn futures::Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>>
        {
            let events = vec![
                Ok(ProviderEvent::TextDelta("Before error. ".into())),
                Ok(ProviderEvent::Error(ModelError::RateLimit {
                    retry_after_secs: Some(5),
                })),
            ];
            Box::pin(futures::stream::iter(events))
        }
    }

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

    // The headless runner should propagate the error without panicking
    let result = run_headless_turn(messages, &ErrorProvider, options).await;
    assert!(
        result.is_err(),
        "expected error to propagate, got: {result:?}"
    );
}

// Layer 1: HeadlessEvent emits valid JSON lines
#[test]
fn headless_event_serializes_to_jsonl() {
    let evt = HeadlessEvent::Text {
        data: "Hello".into(),
    };
    let line = evt.to_json_line();
    assert!(line.contains(r#""type":"text""#));
    assert!(line.contains(r#""data":"Hello""#));
}

// Layer 1: HeadlessEvent error round-trips
#[test]
fn headless_event_error_round_trips() {
    let evt = HeadlessEvent::Error {
        message: "oops".into(),
    };
    let line = evt.to_json_line();
    let parsed: HeadlessEvent = serde_json::from_str(&line).unwrap();
    assert!(matches!(parsed, HeadlessEvent::Error { message } if message == "oops"));
}

// Layer 1: HeadlessEvent tool call events serialize correctly
#[test]
fn headless_event_tool_call_round_trips() {
    let evt = HeadlessEvent::ToolCallStart {
        id: "c1".into(),
        name: "read_file".into(),
    };
    let line = evt.to_json_line();
    let parsed: HeadlessEvent = serde_json::from_str(&line).unwrap();
    assert!(matches!(
        parsed,
        HeadlessEvent::ToolCallStart { id, name }
        if id == "c1" && name == "read_file"
    ));
}

// Layer 1: Denied tool does not cause infinite loop
#[tokio::test]
async fn denied_tool_does_not_loop() {
    use runie_core::permissions::DenyAllSink;

    // Create a deny-all gate (simulates headless mode with no permissions)
    let gate = PermissionGate::new(
        PermissionManager::default(),
        Arc::new(DenyAllSink) as Arc<dyn runie_core::permissions::ApprovalSink>,
    );

    let options = HeadlessOptions {
        execute_tools: true,
        max_tool_rounds: 10, // Set high to verify loop would continue without the fix
        on_chunk: None,
        on_event: None,
        permission_gate: gate,
    };

    // Use MockProvider which emits a bash tool call for "native tool"
    let _mock_guard = crate::tests::ensure_mock_provider().await;
    let provider = MockProvider::default();
    let messages = vec![
        ChatMessage::system("You are helpful."),
        ChatMessage::user("native tool"),
    ];

    // The fix should make this return in a reasonable time instead of looping
    let result = run_headless_turn(messages, &provider, options)
        .await
        .unwrap();

    // Should have exactly one tool output (the denied bash tool)
    assert_eq!(
        result.tool_outputs.len(),
        1,
        "Expected exactly one tool output, got: {:?}",
        result.tool_outputs
    );
    assert_eq!(result.tool_outputs[0].tool_name, "bash");
    assert!(
        result.tool_outputs[0].content.contains("Permission denied"),
        "Expected 'Permission denied', got: {}",
        result.tool_outputs[0].content
    );
    // Verify the tool output is blocked
    assert!(result.tool_outputs[0].status == runie_core::tool::ToolStatus::Blocked);
}
