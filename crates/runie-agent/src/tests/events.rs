//! Tests for event types - now uses runie_core::Event directly
use runie_core::event::AgentEvent;

#[test]
fn test_agent_thinking_event() {
    let evt = AgentEvent::Thinking {
        id: "req.0".to_string(),
    };
    match evt {
        AgentEvent::Thinking { id } => assert_eq!(id, "req.0"),
        _ => panic!("Expected Agent(AgentEvent::Thinking)"),
    }
}

#[test]
fn test_agent_response_event() {
    let evt = AgentEvent::Response {
        id: "req.0".to_string(),
        content: "hello".to_string(),
    };
    match evt {
        AgentEvent::Response { id, content } => {
            assert_eq!(id, "req.0");
            assert_eq!(content, "hello");
        }
        _ => panic!("Expected Agent(AgentEvent::Response)"),
    }
}

#[test]
fn test_agent_tool_start_event() {
    let evt = AgentEvent::ToolStart {
        id: "req.0".to_string(),
        name: "bash".to_string(),
        input: serde_json::Value::Null,
    };
    match evt {
        AgentEvent::ToolStart {
            id,
            name,
            input: serde_json::Value::Null,
        } => {
            assert_eq!(id, "req.0");
            assert_eq!(name, "bash");
        }
        _ => panic!("Expected Agent(AgentEvent::ToolStart)"),
    }
}

#[test]
fn test_agent_tool_end_event() {
    let evt = AgentEvent::ToolEnd {
        id: "".to_string(),
        duration_secs: 1.5,
        output: "result".to_string(),
    };
    match evt {
        AgentEvent::ToolEnd {
            id: _,
            duration_secs,
            output,
        } => {
            assert!((duration_secs - 1.5).abs() < 0.001);
            assert_eq!(output, "result");
        }
        _ => panic!("Expected Agent(AgentEvent::ToolEnd)"),
    }
}

#[test]
fn test_agent_done_event() {
    let evt = AgentEvent::Done {
        id: "req.0".to_string(),
    };
    match evt {
        AgentEvent::Done { id } => assert_eq!(id, "req.0"),
        _ => panic!("Expected Agent(AgentEvent::Done)"),
    }
}

#[test]
fn test_agent_turn_complete_event() {
    let evt = AgentEvent::TurnComplete {
        id: "req.0".to_string(),
        duration_secs: 2.5,
    };
    match evt {
        AgentEvent::TurnComplete { id, duration_secs } => {
            assert_eq!(id, "req.0");
            assert!((duration_secs - 2.5).abs() < 0.001);
        }
        _ => panic!("Expected Agent(AgentEvent::TurnComplete)"),
    }
}
