use crate::dsl::TestHarness;
use crate::{AgentEvent, ControlEvent, DialogEvent, EditEvent, Event, InputEvent, ModelConfigEvent, ScrollEvent, SystemEvent};

#[test]
fn response_delta_updates_tail() {
    let mut h = TestHarness::new();
    // Send a response delta — it should feed the buffer and update tail
    h.state.update(Event::Agent(AgentEvent::ResponseDelta {
        id: "req.1".to_string(),
        content: "Hello".to_string(),
    });
    // The tail is set, message may or may not be flushed (debounce)
    let tail = h.state.agent.streaming_buffer.tail();
    assert!(tail.contains("Hello"));
}

#[test]
fn agent_response_feeds_buffer() {
    let mut h = TestHarness::new();
    h.state.update(Event::Agent(AgentEvent::AgentResponse {
        id: "req.1".to_string(),
        content: "Hello, world!\n\n".to_string(),
    });
    // Stable paragraph should be committed to the message
    let flushed = h.state.agent.streaming_buffer.force_flush();
    assert!(!flushed.is_empty());
    let tail = h.state.agent.streaming_buffer.tail();
    assert!(tail.is_empty());
}

#[test]
fn turn_complete_flushes_buffer() {
    let mut h = TestHarness::new();
    h.state.update(Event::Agent(AgentEvent::AgentResponse {
        id: "req.1".to_string(),
        content: "Some text\n\n".to_string(),
    });
    h.state.update(Event::Agent(AgentEvent::AgentTurnComplete {
        id: "req.1".to_string(),
        duration_secs: 1.0,
    });
    // Buffer should be reset after turn completion
    assert!(h.state.agent.streaming_buffer.is_stable());
}

#[test]
fn streaming_buffer_in_state() {
    let h = TestHarness::new();
    // Streaming buffer exists and is initialized
    assert!(h.state.agent.streaming_buffer.tail().is_empty());
    assert!(h.state.agent.streaming_buffer.is_stable());
}
