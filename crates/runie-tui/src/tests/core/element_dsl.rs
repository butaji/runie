use super::*;
use runie_core::view::elements::Element;

#[test]
fn user_message_builder() {
    let e = Element::user("hello").at(1.0);
    assert!(
        matches!(e, Element::UserMessage { content, timestamp } if content == "hello" && timestamp == 1.0)
    );
}

#[test]
fn agent_message_builder() {
    let e = Element::agent("world").at(2.0);
    assert!(
        matches!(e, Element::AgentMessage { content, timestamp, .. } if content == "world" && timestamp == 2.0)
    );
}

#[test]
fn thought_marker_builder() {
    let e = Element::thought("thinking...").at(3.0);
    assert!(
        matches!(e, Element::ThoughtMarker { content, timestamp } if content == "thinking..." && timestamp == 3.0)
    );
}

#[test]
fn thought_summary_builder() {
    let e = Element::thought_summary("sum", 1.5).at(4.0);
    assert!(
        matches!(e, Element::ThoughtSummary { content, duration_secs, timestamp }
        if content == "sum" && duration_secs == 1.5 && timestamp == 4.0)
    );
}

#[test]
fn tool_running_builder() {
    let started = std::time::Instant::now();
    let e = Element::tool_running("ls", ".", started).at(5.0);
    assert!(
        matches!(e, Element::ToolRunning { name, args, timestamp, .. }
        if name == "ls" && args == "." && timestamp == 5.0)
    );
}

#[test]
fn tool_done_builder() {
    let e = Element::tool_done("ls", ".", 0.5, "file1", None, false).at(6.0);
    assert!(
        matches!(e, Element::ToolDone { name, args, duration_secs, output, bytes_transferred, error, timestamp }
        if name == "ls" && args == "." && duration_secs == 0.5 && output == "file1" && timestamp == 6.0 && bytes_transferred.is_none() && !error)
    );
}

#[test]
fn tool_summary_builder() {
    let e = Element::tool_summary("ls", 0.5).at(7.0);
    assert!(
        matches!(e, Element::ToolSummary { name, duration_secs, timestamp }
        if name == "ls" && duration_secs == 0.5 && timestamp == 7.0)
    );
}

#[test]
fn turn_complete_builder() {
    let e = Element::turn_complete(1.0).at(8.0);
    assert!(
        matches!(e, Element::TurnComplete { duration_secs, timestamp }
        if duration_secs == 1.0 && timestamp == 8.0)
    );
}

#[test]
fn spacer_builder() {
    let e = Element::spacer().at(9.0);
    assert!(matches!(e, Element::Spacer { timestamp } if timestamp == 9.0));
}

#[test]
fn thinking_builder() {
    let started = std::time::Instant::now();
    let e = Element::thinking(started).at(10.0);
    assert!(matches!(e, Element::Thinking { timestamp, .. } if timestamp == 10.0));
}
