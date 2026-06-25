use crate::event::DurableCoreEvent;
use crate::Event;

#[test]
fn durable_conversion_message_sent() {
    let evt = Event::Response {
        id: "r1".into(),
        content: "hello".into(),
    };
    let durable = evt.to_durable();
    assert!(matches!(
        durable,
        Some(DurableCoreEvent::MessageSent { .. })
    ));
}

#[test]
fn durable_conversion_tool_call() {
    let input = serde_json::json!({"command": "ls" });
    let evt = Event::ToolStart {
        id: "t1".into(),
        name: "bash".into(),
        input: input.clone(),
    };
    let durable = evt.to_durable();
    assert!(
        matches!(durable, Some(DurableCoreEvent::ToolCalled { id, name, input: persisted }) if id == "t1" && name == "bash" && persisted == input)
    );
}

#[test]
fn durable_conversion_tool_result_preserves_id() {
    let evt = Event::ToolEnd {
        id: "t1".into(),
        duration_secs: 1.0,
        output: "done".into(),
    };
    let durable = evt.to_durable();
    assert!(
        matches!(durable, Some(DurableCoreEvent::ToolResult { id, output, success }) if id == "t1" && output == "done" && success)
    );
}

#[test]
fn durable_conversion_non_durable_returns_none() {
    let evt = Event::Quit;
    assert!(evt.to_durable().is_none());
}
