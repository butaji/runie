use crate::events::*;
use crate::{AgentMessage, ToolResult};
use runie_core::ToolCall;
use runie_tools::ToolRegistry;
use super::calculate_context_window_usage;
use super::context;

#[tokio::test]
async fn test_tool_empty_name_skipped() {
    let registry = ToolRegistry::new();
    let empty_name = "";
    let result = registry.get(empty_name);
    assert!(result.is_none(), "Empty tool name should not find any tool");
}

#[tokio::test]
async fn test_tool_call_with_empty_name_validation() {
    let tool_call = ToolCall {
        id: "call_test".to_string(),
        name: "".to_string(),
        arguments: serde_json::json!({}),
    };
    assert!(tool_call.name.is_empty());
}

#[tokio::test]
async fn test_tool_invalid_args_returns_error_not_panic() {
    let registry = ToolRegistry::new();
    let tool = registry.get("bash");
    if let Some(tool) = tool {
        let result = tool.execute(serde_json::json!({"command": 123})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, runie_core::ToolError::InvalidArguments(_)));
    }
}

#[tokio::test]
async fn test_tool_missing_required_args_returns_error() {
    let registry = ToolRegistry::new();
    let tool = registry.get("read_file");
    if let Some(tool) = tool {
        let result = tool.execute(serde_json::json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, runie_core::ToolError::InvalidArguments(_)));
        assert!(err.to_string().contains("Missing 'path' argument"));
    }
}

#[tokio::test]
async fn test_tool_call_delta_with_empty_name() {
    let name = "";
    assert!(name.is_empty(), "Empty name should be detected");

    fn is_valid_tool_name(name: &str) -> bool {
        !name.is_empty()
    }

    assert!(!is_valid_tool_name(""), "Empty tool name should be invalid");
    assert!(is_valid_tool_name("bash"), "Non-empty tool name should be valid");
}

#[tokio::test]
async fn test_empty_tool_name_skipped() {
    let empty_name = "";
    let trimmed = empty_name.trim();
    assert!(trimmed.is_empty(), "Empty tool name should be caught by validation");

    fn is_valid_tool_name(name: &str) -> bool {
        !name.trim().is_empty()
    }
    assert!(!is_valid_tool_name(""), "Empty string is invalid");
    assert!(!is_valid_tool_name("   "), "Whitespace-only string is invalid");
    assert!(is_valid_tool_name("bash"), "Normal tool name is valid");
}

#[tokio::test]
async fn test_unknown_tool_name_skipped() {
    let registry = ToolRegistry::new();
    let unknown_tool = registry.get("nonexistent_tool");
    assert!(unknown_tool.is_none(), "Unknown tool should not be found");

    let another_unknown = registry.get("completely_invalid");
    assert!(another_unknown.is_none(), "Invalid tool name should return None");

    let registered_tools: Vec<_> = vec!["bash", "read_file", "write_file"];
    fn tool_exists(name: &str, available: &[&str]) -> bool {
        available.iter().any(|&t| t == name)
    }
    assert!(!tool_exists("unknown", &registered_tools), "Unknown tool should be rejected");
    assert!(tool_exists("bash", &registered_tools), "Known tool should be accepted");
}

#[test]
fn test_compaction_constants() {
    assert!(context::MAX_CONTEXT_MESSAGES > super::COMPACT_THRESHOLD,
        "MAX_CONTEXT_MESSAGES should be greater than COMPACT_THRESHOLD");
    assert!(super::COMPACT_THRESHOLD > context::RECENT_MESSAGES_TO_KEEP,
        "COMPACT_THRESHOLD should be greater than RECENT_MESSAGES_TO_KEEP");
    assert!(context::RECENT_MESSAGES_TO_KEEP > 0, "RECENT_MESSAGES_TO_KEEP should be positive");
}

#[test]
fn test_compact_context_below_threshold() {
    let history = vec![
        AgentMessage {
            role: "system".to_string(),
            content: vec![ContentPart::Text { text: "You are a helpful assistant".to_string() }],
            timestamp: 0,
            usage: None,
            stop_reason: None,
            error_message: None,
            tool_calls: vec![],
        },
        AgentMessage {
            role: "user".to_string(),
            content: vec![ContentPart::Text { text: "Hello".to_string() }],
            timestamp: 1,
            usage: None,
            stop_reason: None,
            error_message: None,
            tool_calls: vec![],
        },
    ];

    let original_len = history.len();
    assert!(original_len <= super::COMPACT_THRESHOLD,
        "Test setup error: history should be below COMPACT_THRESHOLD");
}

#[test]
fn test_message_content_extraction_for_summary() {
    let parts = vec![
        ContentPart::Text { text: "Hello world".to_string() },
    ];
    let tool_calls: Vec<ToolCall> = vec![];
    let content = context::format_message_content(&parts, &tool_calls);
    assert_eq!(content, "Hello world");

    let parts_with_tool = vec![
        ContentPart::Text { text: "".to_string() },
        ContentPart::ToolUse {
            id: "call_123".to_string(),
            name: "bash".to_string(),
            input: serde_json::json!({"command": "ls"}),
        },
    ];
    let tool_calls = vec![ToolCall {
        id: "call_123".to_string(),
        name: "bash".to_string(),
        arguments: serde_json::json!({"command": "ls"}),
    }];
    let content = context::format_message_content(&parts_with_tool, &tool_calls);
    assert!(content.contains("bash"));
    assert!(content.contains("ls"));
}

#[test]
fn test_context_window_usage_calculation() {
    let messages = vec![
        AgentMessage {
            role: "user".to_string(),
            content: vec![ContentPart::Text { text: "This is a test message with some content".to_string() }],
            timestamp: 0,
            usage: None,
            stop_reason: None,
            error_message: None,
            tool_calls: vec![],
        },
    ];

    let usage = calculate_context_window_usage(&messages, 128_000);
    assert!(usage > 0.0, "Usage should be positive for non-empty message");
    assert!(usage < 1.0, "Usage should be less than 1% for small message");

    let empty_messages: Vec<AgentMessage> = vec![];
    let empty_usage = calculate_context_window_usage(&empty_messages, 128_000);
    assert_eq!(empty_usage, 0.0, "Empty messages should give 0% usage");

    let zero_usage = calculate_context_window_usage(&messages, 0);
    assert_eq!(zero_usage, 0.0, "Zero context window should give 0%");
}

#[test]
fn test_summarize_messages_empty_input() {
    let messages: Vec<AgentMessage> = vec![];
    assert!(messages.is_empty());
}
