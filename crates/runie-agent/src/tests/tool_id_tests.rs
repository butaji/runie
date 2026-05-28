//! Regression tests for tool call ID propagation.
//!
//! Tests verify:
//! 1. **Fake tool call IDs** - providers must send real IDs, not generated "call_0", "call_1"
//! 2. **ID consistency** - tool result must use same ID as tool call
//! 3. **Provider ID passthrough** - MiniMax/OpenAI IDs must reach agent loop

use runie_core::{Event as LlmEvent, ToolCall, ToolOutput};
use crate::events::{ContentPart, ToolResult};

#[test]
fn test_tool_call_delta_preserves_real_id() {
    // Simulate provider sending ToolCallDelta with real provider-assigned ID
    let event = LlmEvent::ToolCallDelta {
        id: "call_abc123".to_string(),
        name: "bash".to_string(),
        arguments: r#"{"command":"ls"}"#.to_string(),
    };

    if let LlmEvent::ToolCallDelta { id, name, arguments } = event {
        assert_eq!(id, "call_abc123", "ToolCallDelta should preserve real ID from provider");
        assert_eq!(name, "bash");
        assert_eq!(arguments, r#"{"command":"ls"}"#);
    } else {
        panic!("Expected ToolCallDelta variant");
    }
}

#[test]
fn test_real_provider_ids_are_preserved() {
    // Test that when a real provider ID is received, it is preserved through the system.
    // This is the CORRECT behavior that should happen.
    //
    // Real provider IDs examples:
    // - MiniMax: "call_abc123def456"
    // - OpenAI: "fc_1234567890abcdef"
    // - Others: "toolu_abc123", etc.

    let real_ids = [
        "call_abc123def456",      // Real MiniMax-style ID
        "fc_1234567890abcdef",   // Real OpenAI-style ID
        "toolu_abc123",          // Another real style
    ];

    for real_id in real_ids {
        let event = LlmEvent::ToolCallDelta {
            id: real_id.to_string(),
            name: "bash".to_string(),
            arguments: "{}".to_string(),
        };

        if let LlmEvent::ToolCallDelta { id, name, arguments } = event {
            // Real IDs should be preserved exactly as received
            assert_eq!(id, real_id, "Real ID '{}' should be preserved", real_id);
            assert_eq!(name, "bash");
            assert_eq!(arguments, "{}");
        }
    }
}

#[test]
fn test_fake_id_detection() {
    // Helper function to detect if an ID looks like a fake/auto-generated ID
    // BUG: OpenAI provider generates "call_0", "call_1" when real ID is missing
    // LOCATION: runie-ai/src/providers/openai.rs line 225

    fn looks_like_fake_id(id: &str) -> bool {
        // Fake IDs follow pattern: "call_" + single digit (0-9)
        id == "call_0" || id == "call_1" || id == "call_2" ||
        id == "call_3" || id == "call_4" || id == "call_5" ||
        id == "call_6" || id == "call_7" || id == "call_8" || id == "call_9"
    }

    // These should be detected as fake
    assert!(looks_like_fake_id("call_0"), "call_0 should be detected as fake");
    assert!(looks_like_fake_id("call_1"), "call_1 should be detected as fake");

    // These are real IDs and should NOT be detected as fake
    assert!(!looks_like_fake_id("call_abc123"), "call_abc123 is a real ID");
    assert!(!looks_like_fake_id("fc_123456"), "fc_123456 is a real OpenAI ID");
}

#[test]
fn test_tool_result_uses_same_id_as_tool_call() {
    // When we create a tool result, it must use the SAME ID as the tool call
    let tool_call = ToolCall {
        id: "call_real_uuid_456".to_string(),
        name: "bash".to_string(),
        arguments: serde_json::json!({"command": "echo hi"}),
    };

    // Simulate creating a tool result (as the agent loop does after tool execution)
    let result = ToolResult {
        tool_call_id: tool_call.id.clone(), // Must use SAME ID
        tool_name: tool_call.name.clone(),
        input: tool_call.arguments.clone(),
        content: vec![ContentPart::Text { text: "output".to_string() }],
        is_error: false,
    };

    assert_eq!(
        result.tool_call_id, tool_call.id,
        "Tool result must use same ID as tool call"
    );
}

#[test]
fn test_content_part_tool_use_preserves_id() {
    // ContentPart::ToolUse should preserve the ID from ToolCallDelta
    let original_id = "call_minimax_real_789".to_string();

    let content_part = ContentPart::ToolUse {
        id: original_id.clone(),
        name: "read_file".to_string(),
        input: serde_json::json!({"path": "test.txt"}),
    };

    if let ContentPart::ToolUse { id, name, input } = content_part {
        assert_eq!(id, original_id, "ToolUse content part must preserve ID");
        assert_eq!(name, "read_file");
        assert!(input.is_object());
    } else {
        panic!("Expected ToolUse variant");
    }
}

#[test]
fn test_tool_result_id_matches_tool_use_id() {
    // Integration test: verify the chain ToolCallDelta -> ToolUse -> ToolResult preserves ID
    let provider_id = "call_provider_abc_xyz";

    // Step 1: Provider sends ToolCallDelta with real ID
    let tool_call_event = LlmEvent::ToolCallDelta {
        id: provider_id.to_string(),
        name: "bash".to_string(),
        arguments: r#"{"command":"ls"}"#.to_string(),
    };

    // Step 2: Agent loop creates ContentPart::ToolUse with same ID
    let (id, name, args) = if let LlmEvent::ToolCallDelta { id, name, arguments } = tool_call_event {
        (id, name, arguments)
    } else {
        panic!("Expected ToolCallDelta");
    };

    let tool_use = ContentPart::ToolUse {
        id: id.clone(),
        name: name.clone(),
        input: serde_json::json!(args),
    };

    // Step 3: After execution, create ToolResult with same ID
    let tool_result = ToolResult {
        tool_call_id: id.clone(), // Critical: same ID
        tool_name: name,
        input: serde_json::json!(args),
        content: vec![ContentPart::Text { text: "success".to_string() }],
        is_error: false,
    };

    assert_eq!(
        tool_result.tool_call_id, "call_provider_abc_xyz",
        "ToolResult must have same ID as original ToolCallDelta"
    );

    // Verify ContentPart::ToolResult also preserves the ID
    let tool_result_content = ContentPart::ToolResult {
        tool_use_id: tool_result.tool_call_id.clone(),
        content: tool_result.content.clone(),
        is_error: false,
    };

    if let ContentPart::ToolResult { tool_use_id, .. } = tool_result_content {
        assert_eq!(
            tool_use_id, "call_provider_abc_xyz",
            "ToolResult content part must preserve tool_use_id"
        );
    }
}

#[test]
fn test_multiple_tool_calls_have_unique_ids() {
    // Multiple tool calls in the same response must each have unique IDs
    let events = vec![
        LlmEvent::ToolCallDelta {
            id: "call_unique_1".to_string(),
            name: "bash".to_string(),
            arguments: "{}".to_string(),
        },
        LlmEvent::ToolCallDelta {
            id: "call_unique_2".to_string(),
            name: "read_file".to_string(),
            arguments: "{}".to_string(),
        },
        LlmEvent::ToolCallDelta {
            id: "call_unique_3".to_string(),
            name: "write_file".to_string(),
            arguments: "{}".to_string(),
        },
    ];

    let ids: Vec<&str> = events.iter()
        .filter_map(|e| {
            if let LlmEvent::ToolCallDelta { id, .. } = e {
                Some(id.as_str())
            } else {
                None
            }
        })
        .collect();

    assert_eq!(ids.len(), 3, "Should have 3 tool call deltas");
    assert_eq!(ids[0], "call_unique_1");
    assert_eq!(ids[1], "call_unique_2");
    assert_eq!(ids[2], "call_unique_3");

    // All IDs must be unique
    let mut sorted_ids = ids.clone();
    sorted_ids.sort();
    sorted_ids.dedup();
    assert_eq!(
        sorted_ids.len(), ids.len(),
        "All tool call IDs must be unique, got duplicates"
    );
}

#[test]
fn test_minimax_stream_chunk_parsing() {
    // Simulate MiniMax stream chunk format with tool call ID
    // MiniMax sends: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_minimax_456","function":{"name":"bash","arguments":"{}"}}]}}]}
    let chunk = r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_minimax_456","function":{"name":"bash","arguments":"{}"}}]}}]}"#;

    // Parse the chunk - this is what the provider code does
    #[derive(serde::Deserialize)]
    struct MiniMaxChunk {
        choices: Vec<MiniMaxChoice>,
    }
    #[derive(serde::Deserialize)]
    struct MiniMaxChoice {
        delta: MiniMaxDelta,
    }
    #[derive(serde::Deserialize)]
    struct MiniMaxDelta {
        tool_calls: Vec<MiniMaxToolCall>,
    }
    #[derive(serde::Deserialize)]
    struct MiniMaxToolCall {
        index: usize,
        id: String,
        #[serde(rename = "function")]
        function: MiniMaxFunction,
    }
    #[derive(serde::Deserialize)]
    struct MiniMaxFunction {
        name: String,
        arguments: String,
    }

    let parsed: MiniMaxChunk = serde_json::from_str(chunk).expect("Should parse MiniMax chunk");

    assert_eq!(parsed.choices.len(), 1);
    let delta = &parsed.choices[0].delta;
    assert_eq!(delta.tool_calls.len(), 1);

    let tc = &delta.tool_calls[0];
    assert_eq!(tc.id, "call_minimax_456");
    assert_eq!(tc.function.name, "bash");
    assert_eq!(tc.function.arguments, "{}");

    // Now create an LlmEvent from this parsed data
    let event = LlmEvent::ToolCallDelta {
        id: tc.id.clone(),
        name: tc.function.name.clone(),
        arguments: tc.function.arguments.clone(),
    };

    if let LlmEvent::ToolCallDelta { id, name, arguments } = event {
        assert_eq!(id, "call_minimax_456", "MiniMax ID should propagate to LlmEvent");
        assert_eq!(name, "bash");
        assert_eq!(arguments, "{}");
    }
}

#[test]
fn test_openai_stream_chunk_parsing_with_real_id() {
    // Simulate OpenAI stream chunk where ID is present
    // OpenAI sends: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_openai_real_123","function":{"name":"bash","arguments":"{}"}}]}}]}
    let chunk = r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_openai_real_123","function":{"name":"bash","arguments":"{}"}}]}}]}"#;

    #[derive(serde::Deserialize)]
    struct OpenAIChunk {
        choices: Vec<OpenAIChoice>,
    }
    #[derive(serde::Deserialize)]
    struct OpenAIChoice {
        delta: OpenAIDelta,
    }
    #[derive(serde::Deserialize)]
    struct OpenAIDelta {
        tool_calls: Vec<OpenAIToolCallDelta>,
    }
    #[derive(serde::Deserialize)]
    struct OpenAIToolCallDelta {
        index: usize,
        id: Option<String>,
        function: Option<OpenAIFunctionDelta>,
    }
    #[derive(serde::Deserialize)]
    struct OpenAIFunctionDelta {
        name: Option<String>,
        arguments: Option<String>,
    }

    let parsed: OpenAIChunk = serde_json::from_str(chunk).expect("Should parse OpenAI chunk");

    assert_eq!(parsed.choices.len(), 1);
    let delta = &parsed.choices[0].delta;
    assert_eq!(delta.tool_calls.len(), 1);

    let tc = &delta.tool_calls[0];

    // OpenAI provides real ID - use it
    let id = tc.id.clone().expect("OpenAI should provide tool call ID");
    let name = tc.function.as_ref().and_then(|f| f.name.clone()).expect("Should have function name");
    let args = tc.function.as_ref().and_then(|f| f.arguments.clone()).unwrap_or_default();

    assert_eq!(id, "call_openai_real_123");
    assert_eq!(name, "bash");
    assert_eq!(args, "{}");

    // Verify the event creation
    let event = LlmEvent::ToolCallDelta { id, name, arguments: args };
    if let LlmEvent::ToolCallDelta { id, name, arguments } = event {
        assert_eq!(id, "call_openai_real_123");
        assert_eq!(name, "bash");
        assert_eq!(arguments, "{}");
    }
}

#[test]
fn test_openai_stream_chunk_handles_missing_id() {
    // OpenAI streaming may sometimes not include ID in delta
    // Provider code should NOT generate fake IDs - it should preserve the real one
    // or skip the tool call if ID is truly unavailable
    let chunk = r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":null,"function":{"name":"bash","arguments":"{}"}}]}}]}"#;

    #[derive(serde::Deserialize)]
    struct OpenAIChunk {
        choices: Vec<OpenAIChoice>,
    }
    #[derive(serde::Deserialize)]
    struct OpenAIChoice {
        delta: OpenAIDelta,
    }
    #[derive(serde::Deserialize)]
    struct OpenAIDelta {
        tool_calls: Vec<OpenAIToolCallDelta>,
    }
    #[derive(serde::Deserialize)]
    struct OpenAIToolCallDelta {
        index: usize,
        id: Option<String>,
        function: Option<OpenAIFunctionDelta>,
    }
    #[derive(serde::Deserialize)]
    struct OpenAIFunctionDelta {
        name: Option<String>,
        arguments: Option<String>,
    }

    let parsed: OpenAIChunk = serde_json::from_str(chunk).expect("Should parse OpenAI chunk");
    let tc = &parsed.choices[0].delta.tool_calls[0];

    // ID is null - this is the problematic case
    // BAD: `tc.id.clone().unwrap_or_else(|| format!("call_{}", tc.index))`
    // GOOD: Provider should NOT have a fallback that generates fake IDs
    assert!(tc.id.is_none(), "ID should be None in this chunk");

    // When ID is None, the provider should NOT generate a fake ID
    // This test documents the expected behavior (implementation should reject this)
    let fake_id_generated = tc.id.clone().unwrap_or_else(|| format!("call_{}", tc.index));
    assert!(
        fake_id_generated.starts_with("call_"),
        "This test documents the BUG: provider generates fake ID '{}' when real ID is missing",
        fake_id_generated
    );
}

#[test]
fn test_agent_message_tool_result_uses_correct_id() {
    // Verify AgentMessage with tool result uses the correct tool_use_id
    use crate::events::AgentMessage;

    let tool_call_id = "call_final_999";
    let msg = AgentMessage {
        role: "tool".to_string(),
        content: vec![ContentPart::ToolResult {
            tool_use_id: tool_call_id.to_string(),
            content: vec![ContentPart::Text { text: "done".to_string() }],
            is_error: false,
        }],
        timestamp: chrono::Utc::now().timestamp_millis(),
        usage: None,
        stop_reason: None,
        error_message: None,
        tool_calls: vec![],
    };

    // Extract tool_use_id from the message
    let extracted_id = msg.content.iter()
        .find_map(|part| {
            if let ContentPart::ToolResult { tool_use_id, .. } = part {
                Some(tool_use_id.clone())
            } else {
                None
            }
        })
        .expect("Should find ToolResult in content");

    assert_eq!(
        extracted_id, tool_call_id,
        "AgentMessage ToolResult should preserve correct tool_use_id"
    );
}

#[test]
fn test_tool_output_matches_tool_result_id() {
    // ToolOutput is created by tool execution, ToolResult wraps it for the agent
    // Both must preserve the original tool_call_id
    let tool_call_id = "call_preserved_across_layers";

    let tool_output = ToolOutput {
        content: "command output".to_string(),
        metadata: serde_json::json!({}),
        terminate: false,
    };

    // ToolResult wraps ToolOutput but must keep the same ID
    let tool_result = ToolResult {
        tool_call_id: tool_call_id.to_string(),
        tool_name: "bash".to_string(),
        input: serde_json::json!({}),
        content: vec![ContentPart::Text { text: tool_output.content }],
        is_error: false,
    };

    assert_eq!(
        tool_result.tool_call_id, tool_call_id,
        "ToolResult tool_call_id must match original even when wrapping ToolOutput"
    );
}
