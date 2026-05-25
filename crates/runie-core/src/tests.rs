//! Tests for runie-core types

use crate::{Event, Message, MessageNode, Session, ToolSchema};

#[test]
fn test_message_user_creation() {
    let msg = Message::User {
        content: "Hello".to_string(),
        attachments: vec![],
    };
    assert!(matches!(msg, Message::User { content, .. } if content == "Hello"));
}

#[test]
fn test_message_system_creation() {
    let msg = Message::System {
        content: "You are helpful".to_string(),
    };
    assert!(matches!(msg, Message::System { .. }));
    if let Message::System { content } = msg {
        assert_eq!(content, "You are helpful");
    }
}

#[test]
fn test_message_assistant_with_tool_calls() {
    let msg = Message::Assistant {
        content: "I'll help".to_string(),
        tool_calls: vec![],
        thinking: Some("Let me think".to_string()),
    };
    assert!(matches!(msg, Message::Assistant { thinking: Some(_), .. }));
}

#[test]
fn test_message_tool_result() {
    let msg = Message::ToolResult {
        tool_call_id: "call_1".to_string(),
        content: "Result content".to_string(),
        is_error: false,
    };
    if let Message::ToolResult { tool_call_id, content, is_error } = msg {
        assert_eq!(tool_call_id, "call_1");
        assert_eq!(content, "Result content");
        assert!(!is_error);
    }
}

#[test]
fn test_event_agent_start() {
    let event = Event::AgentStart {
        session_id: "test-session".to_string(),
        timestamp: chrono::Utc::now(),
    };
    assert!(matches!(event, Event::AgentStart { .. }));
    if let Event::AgentStart { session_id, .. } = &event {
        assert_eq!(session_id, "test-session");
    }
}

#[test]
fn test_event_turn_start() {
    let event = Event::TurnStart {
        turn: 5,
        timestamp: chrono::Utc::now(),
    };
    assert!(matches!(event, Event::TurnStart { turn: 5, .. }));
}

#[test]
fn test_event_tool_execution() {
    let event = Event::ToolExecutionStart {
        tool_call_id: "call_1".to_string(),
        tool_name: "bash".to_string(),
        args: serde_json::json!({}),
        timestamp: chrono::Utc::now(),
    };
    assert!(matches!(event, Event::ToolExecutionStart { .. }));
    if let Event::ToolExecutionStart { tool_name, .. } = &event {
        assert_eq!(tool_name, "bash");
    }
}

#[test]
fn test_event_usage() {
    let event = Event::Usage {
        prompt_tokens: 100,
        completion_tokens: 50,
        total_tokens: 150,
    };
    if let Event::Usage { prompt_tokens, completion_tokens, total_tokens } = event {
        assert_eq!(prompt_tokens, 100);
        assert_eq!(completion_tokens, 50);
        assert_eq!(total_tokens, 150);
    }
}

#[test]
fn test_event_error() {
    let event = Event::Error {
        message: "Something went wrong".to_string(),
    };
    assert!(matches!(event, Event::Error { .. }));
}

#[test]
fn test_tool_schema_creation() {
    let schema = ToolSchema {
        name: "test_tool".to_string(),
        description: "A test tool".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {}
        }),
    };
    assert_eq!(schema.name, "test_tool");
    assert_eq!(schema.description, "A test tool");
}

#[test]
fn test_tool_schema_with_parameters() {
    let schema = ToolSchema {
        name: "bash".to_string(),
        description: "Run bash commands".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The command to run"
                }
            },
            "required": ["command"]
        }),
    };
    assert!(schema.parameters.is_object());
    let props = schema.parameters.get("properties").unwrap();
    assert!(props.get("command").is_some());
}

#[test]
fn test_session_new() {
    let session = Session::new("test-id".to_string());
    assert_eq!(session.id, "test-id");
    assert!(session.messages.is_empty());
    assert!(session.metadata.is_null());
}

#[test]
fn test_session_add_message() {
    let mut session = Session::new("test-id".to_string());
    let msg = Message::User {
        content: "Hello".to_string(),
        attachments: vec![],
    };
    let node_id = session.add_message(None, msg);
    assert!(!node_id.is_empty());
    assert_eq!(session.messages.len(), 1);
}

#[test]
fn test_session_message_node() {
    let node = MessageNode {
        id: "node-1".to_string(),
        parent_id: None,
        message: Message::User {
            content: "Hi".to_string(),
            attachments: vec![],
        },
        timestamp: chrono::Utc::now(),
        metadata: serde_json::Value::Null,
    };
    assert_eq!(node.id, "node-1");
    assert!(node.parent_id.is_none());
    assert!(matches!(node.message, Message::User { .. }));
}

#[test]
fn test_session_get_message() {
    let mut session = Session::new("test".to_string());
    let msg = Message::System { content: "Test".to_string() };
    let id = session.add_message(None, msg);
    let retrieved = session.get_message(&id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, id);
}

#[test]
fn test_session_get_thread() {
    let mut session = Session::new("test".to_string());
    let root_msg = Message::System { content: "Root".to_string() };
    let root_id = session.add_message(None, root_msg);
    let child_msg = Message::User { content: "Child".to_string(), attachments: vec![] };
    session.add_message(Some(root_id.clone()), child_msg);
    let thread = session.get_thread(&root_id);
    assert_eq!(thread.len(), 1);
    assert_eq!(thread[0].id, root_id);
}
