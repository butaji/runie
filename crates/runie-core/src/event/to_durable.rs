//! Durable event conversion — delegates to `DurableCoreEvent::try_from_event`.
//!
//! `Event::to_durable()` is the canonical public API. The conversion logic lives
//! in `DurableCoreEvent::try_from_event`, making it reusable from `From<Event>`
//! and eliminating the duplicate match in `to_durable.rs`.

use super::Event;

/// Convert this event to a durable core event for JSONL persistence.
/// Returns `None` for transient-only events (keystrokes, scroll, streaming deltas).
impl Event {
    pub fn to_durable(&self) -> Option<crate::event::DurableCoreEvent> {
        crate::event::DurableCoreEvent::try_from_event(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that tool and message events convert to durable events.
    #[test]
    fn tool_and_message_events_become_durable() {
        // Tool call → durable
        let event = Event::ToolStart {
            id: "c1".into(),
            name: "bash".into(),
            input: serde_json::json!({"cmd": "ls"}),
        };
        let durable = event.to_durable();
        assert!(durable.is_some());
        let durable = durable.unwrap();
        assert!(matches!(
            durable,
            crate::event::DurableCoreEvent::ToolCalled { id, name, .. }
            if id == "c1" && name == "bash"
        ));

        // Tool result → durable
        let event = Event::ToolEnd {
            id: "c1".into(),
            duration_secs: 1.5,
            output: "result".into(),
        };
        let durable = event.to_durable();
        assert!(durable.is_some());

        // Model switch → durable
        let event = Event::SwitchModel {
            provider: "anthropic".into(),
            model: "claude-3".into(),
            explicit: true,
        };
        let durable = event.to_durable();
        assert!(durable.is_some());
    }

    /// Verify that transient streaming events do NOT become durable.
    #[test]
    fn transient_events_skip_durable() {
        let transient = vec![
            Event::ResponseDelta {
                id: "r1".into(),
                content: "hello".into(),
            },
            Event::TextStart { id: "t1".into() },
            Event::TextEnd { id: "t1".into() },
            Event::ThinkingDelta {
                id: "th1".into(),
                content: "thinking".into(),
            },
        ];

        for event in transient {
            assert!(
                event.to_durable().is_none(),
                "{:?} should not become durable",
                event
            );
        }
    }

    /// Verify Response converts to MessageSent with correct role.
    #[test]
    fn response_converts_to_message_sent() {
        let event = Event::Response {
            id: "r1".into(),
            content: "Hello world".into(),
        };
        let durable = event.to_durable();
        assert!(matches!(
            durable,
            Some(crate::event::DurableCoreEvent::MessageSent {
                role,
                content,
                ..
            }) if role == "assistant" && content == "Hello world"
        ));
    }
}
