//! Tests for event types - now uses runie_core::Event directly
use runie_core::Event;

#[test]
fn test_agent_thinking_event() {
    let evt = Event::AgentThinking {
        id: "req.0".to_string(),
    };
    match evt {
        Event::AgentThinking { id } => assert_eq!(id, "req.0"),
        _ => panic!("Expected AgentThinking"),
    }
}

#[test]
fn test_agent_response_event() {
    let evt = Event::AgentResponse {
        id: "req.0".to_string(),
        content: "hello".to_string(),
    };
    match evt {
        Event::AgentResponse { id, content } => {
            assert_eq!(id, "req.0");
            assert_eq!(content, "hello");
        }
        _ => panic!("Expected AgentResponse"),
    }
}

#[test]
fn test_agent_tool_start_event() {
    let evt = Event::AgentToolStart {
        id: "req.0".to_string(),
        name: "bash".to_string(),
    };
    match evt {
        Event::AgentToolStart { id, name } => {
            assert_eq!(id, "req.0");
            assert_eq!(name, "bash");
        }
        _ => panic!("Expected AgentToolStart"),
    }
}

#[test]
fn test_agent_tool_end_event() {
    let evt = Event::AgentToolEnd {
        duration_secs: 1.5,
        output: "result".to_string(),
    };
    match evt {
        Event::AgentToolEnd {
            duration_secs,
            output,
        } => {
            assert!((duration_secs - 1.5).abs() < 0.001);
            assert_eq!(output, "result");
        }
        _ => panic!("Expected AgentToolEnd"),
    }
}

#[test]
fn test_agent_done_event() {
    let evt = Event::AgentDone {
        id: "req.0".to_string(),
    };
    match evt {
        Event::AgentDone { id } => assert_eq!(id, "req.0"),
        _ => panic!("Expected AgentDone"),
    }
}

#[test]
fn test_agent_turn_complete_event() {
    let evt = Event::AgentTurnComplete {
        id: "req.0".to_string(),
        duration_secs: 2.5,
    };
    match evt {
        Event::AgentTurnComplete { id, duration_secs } => {
            assert_eq!(id, "req.0");
            assert!((duration_secs - 2.5).abs() < 0.001);
        }
        _ => panic!("Expected AgentTurnComplete"),
    }
}
