use crate::AgentEvent;
use runie_core::Event;

fn assert_converts(evt: AgentEvent, matcher: fn(&Event) -> bool) {
    let core = evt.to_core_event();
    assert!(matcher(&core), "Event conversion mismatch");
}

#[test]
fn test_thinking_to_core() {
    assert_converts(
        AgentEvent::Thinking { id: "req.0".to_string() },
        |e| matches!(e, Event::AgentThinking { id } if id == "req.0"),
    );
}

#[test]
fn test_thought_done_to_core() {
    assert_converts(
        AgentEvent::ThoughtDone { id: "req.0".to_string() },
        |e| matches!(e, Event::AgentThoughtDone { id } if id == "req.0"),
    );
}

#[test]
fn test_tool_start_to_core() {
    assert_converts(
        AgentEvent::ToolStart { id: "req.0".to_string(), name: "ls".to_string() },
        |e| matches!(e, Event::AgentToolStart { id, name } if id == "req.0" && name == "ls"),
    );
}

#[test]
fn test_tool_end_to_core() {
    assert_converts(
        AgentEvent::ToolEnd { duration_secs: 1.0, output: "out".to_string() },
        |e| matches!(e, Event::AgentToolEnd { duration_secs, output } if *duration_secs == 1.0 && output == "out"),
    );
}

#[test]
fn test_response_to_core() {
    assert_converts(
        AgentEvent::Response { id: "req.0".to_string(), content: "hi".to_string() },
        |e| matches!(e, Event::AgentResponse { id, content } if id == "req.0" && content == "hi"),
    );
}

#[test]
fn test_turn_complete_to_core() {
    assert_converts(
        AgentEvent::TurnComplete { id: "req.0".to_string(), duration_secs: 2.0 },
        |e| matches!(e, Event::AgentTurnComplete { id, duration_secs } if id == "req.0" && *duration_secs == 2.0),
    );
}

#[test]
fn test_done_to_core() {
    assert_converts(
        AgentEvent::Done { id: "req.0".to_string() },
        |e| matches!(e, Event::AgentDone { id } if id == "req.0"),
    );
}

#[test]
fn test_error_to_core() {
    assert_converts(
        AgentEvent::Error { id: "req.0".to_string(), message: "oops".to_string() },
        |e| matches!(e, Event::AgentError { id, message } if id == "req.0" && message == "oops"),
    );
}
