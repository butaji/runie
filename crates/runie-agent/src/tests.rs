//! Tests for runie-agent
//! Layer 1: Pure state/logic tests (no ratatui)
//! Layer 2: Event handling tests (agent loop with mock provider)

use runie_core::provider::{Message, Provider, ResponseChunk};
use runie_provider::MockProvider;
use crate::{
    parse_tool_calls, has_tool_calls, AgentCommand, AgentEvent,
    Tool, ToolResult, run_agent_turn,
};

// ============================================================================
// Layer 1: Tool Parsing (Pure Functions)
// ============================================================================

#[test]
fn test_parse_read_file_tool() {
    let text = "TOOL:read_file:Cargo.toml";
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0], Tool::ReadFile { path: "Cargo.toml".to_string() });
}

#[test]
fn test_parse_list_dir_tool() {
    let text = "TOOL:list_dir:src";
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0], Tool::ListDir { path: "src".to_string() });
}

#[test]
fn test_parse_write_file_tool() {
    let text = "TOOL:write_file:hello.txt:Hello World";
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0], Tool::WriteFile {
        path: "hello.txt".to_string(),
        content: "Hello World".to_string(),
    });
}

#[test]
fn test_parse_bash_tool() {
    let text = "TOOL:bash:echo hello";
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0], Tool::Bash { command: "echo hello".to_string() });
}

#[test]
fn test_parse_multiple_tools() {
    let text = "TOOL:read_file:a.txt\nTOOL:read_file:b.txt";
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 2);
}

#[test]
fn test_parse_no_tools() {
    let text = "Hello, how can I help you?";
    let tools = parse_tool_calls(text);
    assert!(tools.is_empty());
}

#[test]
fn test_parse_unknown_tool_ignored() {
    let text = "TOOL:unknown_tool:arg";
    let tools = parse_tool_calls(text);
    assert!(tools.is_empty());
}

#[test]
fn test_has_tool_calls_true() {
    assert!(has_tool_calls("TOOL:bash:ls"));
}

#[test]
fn test_has_tool_calls_false() {
    assert!(!has_tool_calls("Just a plain response"));
}

#[test]
fn test_parse_tool_with_extra_colons_in_content() {
    let text = "TOOL:write_file:test.txt:line1:line2";
    let tools = parse_tool_calls(text);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0], Tool::WriteFile {
        path: "test.txt".to_string(),
        content: "line1:line2".to_string(),
    });
}

// ============================================================================
// Layer 1: Tool Execution (Pure Functions / IO)
// ============================================================================

#[test]
fn test_tool_read_file_exists() {
    let tool = Tool::ReadFile { path: "Cargo.toml".to_string() };
    let result = tool.execute();
    assert!(result.success);
    assert!(result.output.contains("runie-agent"));
}

#[test]
fn test_tool_read_file_missing() {
    let tool = Tool::ReadFile { path: "nonexistent_file_12345.txt".to_string() };
    let result = tool.execute();
    assert!(!result.success);
    assert!(result.output.contains("Error"));
}

#[test]
fn test_tool_list_dir() {
    let tool = Tool::ListDir { path: ".".to_string() };
    let result = tool.execute();
    assert!(result.success);
    assert!(!result.output.is_empty());
}

#[test]
fn test_tool_write_file_roundtrip() {
    let path = "/tmp/runie_agent_test_write.txt";
    let tool = Tool::WriteFile {
        path: path.to_string(),
        content: "test content 42".to_string(),
    };
    let result = tool.execute();
    assert!(result.success);

    let read = Tool::ReadFile { path: path.to_string() }.execute();
    assert!(read.success);
    assert_eq!(read.output, "test content 42");

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_tool_bash_echo() {
    let tool = Tool::Bash { command: "echo hello_agent".to_string() };
    let result = tool.execute();
    assert!(result.success);
    assert!(result.output.contains("hello_agent"));
}

#[test]
fn test_tool_bash_invalid_command() {
    let tool = Tool::Bash { command: "not_a_real_command_12345".to_string() };
    let result = tool.execute();
    assert!(!result.success);
}

#[test]
fn test_tool_result_structure() {
    let tool = Tool::Bash { command: "echo ok".to_string() };
    let result = tool.execute();
    assert_eq!(result.tool.name(), "bash");
    assert!(result.success);
}

// ============================================================================
// Layer 1: Agent Command Structure
// ============================================================================

#[test]
fn test_agent_command_structure() {
    let cmd = AgentCommand {
        content: "test".to_string(),
        id: "req.0".to_string(),
    };

    assert_eq!(cmd.content, "test");
    assert_eq!(cmd.id, "req.0");
}

// ============================================================================
// Layer 2: Agent Loop (Event Handling)
// ============================================================================

#[test]
fn test_agent_loop_simple_response() {
    let provider = MockProvider;
    let cmd = AgentCommand {
        content: "Hello World".to_string(),
        id: "req.0".to_string(),
    };

    let mut events = Vec::new();
    run_agent_turn(
        &provider,
        &cmd,
        |evt| events.push(evt),
        5,
    );

    // Should emit: thinking, thought_done, 2 response chunks, done
    let thinking_count = events.iter().filter(|e| matches!(e, AgentEvent::Thinking { .. })).count();
    let response_count = events.iter().filter(|e| matches!(e, AgentEvent::Response { .. })).count();
    let done_count = events.iter().filter(|e| matches!(e, AgentEvent::Done { .. })).count();

    assert_eq!(thinking_count, 1);
    assert_eq!(response_count, 2);
    assert_eq!(done_count, 1);
}

#[test]
fn test_agent_loop_with_tool_call() {
    let provider = MockProvider;
    let cmd = AgentCommand {
        content: "list files".to_string(),
        id: "req.0".to_string(),
    };

    let mut events = Vec::new();
    run_agent_turn(
        &provider,
        &cmd,
        |evt| events.push(evt),
        5,
    );

    let tool_start_count = events.iter().filter(|e| matches!(e, AgentEvent::ToolStart { .. })).count();
    let tool_end_count = events.iter().filter(|e| matches!(e, AgentEvent::ToolEnd { .. })).count();
    let turn_complete_count = events.iter().filter(|e| matches!(e, AgentEvent::TurnComplete { .. })).count();

    // Should call list_dir tool, then get final response
    assert!(tool_start_count >= 1, "Expected at least 1 tool start, got {}", tool_start_count);
    assert_eq!(tool_start_count, tool_end_count);
    assert_eq!(turn_complete_count, 1);
}

#[test]
fn test_agent_loop_respects_max_iterations() {
    // Create a provider that always returns a tool call (infinite loop if unchecked)
    struct InfiniteLoopProvider;
    impl Provider for InfiniteLoopProvider {
        fn generate(&self, _messages: Vec<Message>) -> Vec<ResponseChunk> {
            vec![ResponseChunk {
                content: "TOOL:bash:echo loop".to_string(),
            }]
        }
    }

    let provider = InfiniteLoopProvider;
    let cmd = AgentCommand {
        content: "loop".to_string(),
        id: "req.0".to_string(),
    };

    let mut events = Vec::new();
    run_agent_turn(
        &provider,
        &cmd,
        |evt| events.push(evt),
        3,
    );

    let tool_start_count = events.iter().filter(|e| matches!(e, AgentEvent::ToolStart { .. })).count();

    // Should stop after max_iterations (3) even though provider keeps requesting tools
    assert_eq!(tool_start_count, 3);
}

#[test]
fn test_agent_loop_events_have_correct_id() {
    let provider = MockProvider;
    let cmd = AgentCommand {
        content: "test".to_string(),
        id: "req.42".to_string(),
    };

    let mut events = Vec::new();
    run_agent_turn(
        &provider,
        &cmd,
        |evt| events.push(evt),
        5,
    );

    for evt in &events {
        let evt_id = match evt {
            AgentEvent::Thinking { id } => id,
            AgentEvent::ThoughtDone { id } => id,
            AgentEvent::ToolStart { id, .. } => id,
            AgentEvent::Response { id, .. } => id,
            AgentEvent::TurnComplete { id, .. } => id,
            AgentEvent::Done { id } => id,
            AgentEvent::Error { id, .. } => id,
            _ => continue,
        };
        assert_eq!(evt_id, "req.42");
    }
}

#[test]
fn test_agent_event_to_core_event_mapping() {
    use runie_core::Event;

    let events = vec![
        AgentEvent::Thinking { id: "req.0".to_string() },
        AgentEvent::ThoughtDone { id: "req.0".to_string() },
        AgentEvent::ToolStart { id: "req.0".to_string(), name: "test".to_string() },
        AgentEvent::ToolEnd { duration_secs: 1.0 },
        AgentEvent::Response { id: "req.0".to_string(), content: "hi".to_string() },
        AgentEvent::TurnComplete { id: "req.0".to_string(), duration_secs: 2.0 },
        AgentEvent::Done { id: "req.0".to_string() },
        AgentEvent::Error { id: "req.0".to_string(), message: "oops".to_string() },
    ];

    for evt in events {
        let core_evt = evt.to_core_event();
        match (&evt, core_evt) {
            (AgentEvent::Thinking { id }, Event::AgentThinking { id: core_id }) => {
                assert_eq!(id, &core_id);
            }
            (AgentEvent::ThoughtDone { id }, Event::AgentThoughtDone { id: core_id }) => {
                assert_eq!(id, &core_id);
            }
            (AgentEvent::ToolStart { id, name }, Event::AgentToolStart { id: core_id, name: core_name }) => {
                assert_eq!(id, &core_id);
                assert_eq!(name, &core_name);
            }
            (AgentEvent::ToolEnd { duration_secs }, Event::AgentToolEnd { duration_secs: core_dur }) => {
                assert_eq!(duration_secs, &core_dur);
            }
            (AgentEvent::Response { id, content }, Event::AgentResponse { id: core_id, content: core_content }) => {
                assert_eq!(id, &core_id);
                assert_eq!(content, &core_content);
            }
            (AgentEvent::TurnComplete { id, duration_secs }, Event::AgentTurnComplete { id: core_id, duration_secs: core_dur }) => {
                assert_eq!(id, &core_id);
                assert_eq!(duration_secs, &core_dur);
            }
            (AgentEvent::Done { id }, Event::AgentDone { id: core_id }) => {
                assert_eq!(id, &core_id);
            }
            (AgentEvent::Error { id, message }, Event::AgentError { id: core_id, message: core_msg }) => {
                assert_eq!(id, &core_id);
                assert_eq!(message, &core_msg);
            }
            _ => panic!("Mismatched event conversion"),
        }
    }
}
