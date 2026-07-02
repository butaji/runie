//! Conversion from the canonical provider streaming vocabulary to `Event`.
//!
//! `ProviderEvent` is the canonical type for the provider stream. Convert to
//! `Event` at the TUI/headless boundary so internal event handling can use a
//! single type.

use super::Event;
use crate::provider_event::ProviderEvent;

/// Convert a provider streaming event to the internal `Event` type.
///
/// Not all `ProviderEvent` variants have a direct `Event` equivalent; some
/// represent internal state machine transitions (`Thinking`, `ThoughtDone`)
/// that are emitted by the agent turn code rather than the provider stream.
impl From<ProviderEvent> for Event {
    fn from(event: ProviderEvent) -> Self {
        use ProviderEvent as PE;
        match event {
            PE::TextStart { id } => Event::TextStart { id },
            PE::TextDelta(content) => text_delta(content),
            PE::TextEnd { id } => Event::TextEnd { id },
            PE::ThinkingStart { id } => Event::ThinkingStart { id },
            PE::ThinkingDelta(content) => thinking_delta(content),
            PE::ThinkingEnd { id } => Event::ThinkingEnd { id },
            PE::ToolCallStart { id, name } => tool_start(id, name),
            PE::ToolCallInputDelta { id, delta } => tool_input_delta(id, delta),
            PE::ToolCallEnd { id } => tool_end(id),
            PE::Finish { reason: _ } => Event::Done { id: String::new() },
            PE::Error(e) => Event::Error {
                id: String::new(),
                message: e.to_string(),
            },
            PE::Usage { .. } => usage_event(),
        }
    }
}

fn text_delta(content: String) -> Event {
    Event::ResponseDelta {
        id: String::new(),
        content,
    }
}

fn thinking_delta(content: String) -> Event {
    Event::ThinkingDelta {
        id: String::new(),
        content,
    }
}

fn tool_start(id: String, name: String) -> Event {
    Event::ToolStart {
        id,
        name,
        input: Default::default(),
    }
}

fn tool_input_delta(id: String, delta: String) -> Event {
    Event::ToolInputDelta { id, content: delta }
}

fn tool_end(id: String) -> Event {
    Event::tool_end(id, 0.0, String::new())
}

fn usage_event() -> Event {
    Event::ResponseDelta {
        id: String::new(),
        content: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify every ProviderEvent variant maps to an Event without panicking.
    #[test]
    fn provider_event_maps_to_event() {
        let cases: Vec<ProviderEvent> = vec![
            ProviderEvent::TextStart { id: "t1".into() },
            ProviderEvent::TextDelta("hello".into()),
            ProviderEvent::TextEnd { id: "t1".into() },
            ProviderEvent::ThinkingStart { id: "th1".into() },
            ProviderEvent::ThinkingDelta("thinking...".into()),
            ProviderEvent::ThinkingEnd { id: "th1".into() },
            ProviderEvent::ToolCallStart {
                id: "c1".into(),
                name: "bash".into(),
            },
            ProviderEvent::ToolCallInputDelta {
                id: "c1".into(),
                delta: "-la".into(),
            },
            ProviderEvent::ToolCallEnd { id: "c1".into() },
            ProviderEvent::Error(crate::provider_event::ModelError::Other("oops".into())),
            ProviderEvent::Usage {
                input_tokens: 100,
                output_tokens: 50,
            },
            ProviderEvent::Finish {
                reason: crate::provider_event::StopReason::Stop,
            },
        ];

        for event in cases {
            let _: Event = event.clone().into();
        }
    }

    /// Verify that text and tool lifecycle events map 1:1 (preserving id).
    #[test]
    fn text_and_tool_lifecycle_preserves_id() {
        let text_start = ProviderEvent::TextStart { id: "id123".into() };
        let ev: Event = text_start.into();
        if let Event::TextStart { id } = ev {
            assert_eq!(id, "id123");
        } else {
            panic!("Expected TextStart");
        }

        let tool_start = ProviderEvent::ToolCallStart {
            id: "c99".into(),
            name: "read".into(),
        };
        let ev: Event = tool_start.into();
        if let Event::ToolStart { id, name, .. } = ev {
            assert_eq!(id, "c99");
            assert_eq!(name, "read");
        } else {
            panic!("Expected ToolStart");
        }
    }

    /// Verify ToolInputDelta maps to the new ToolInputDelta event (not ResponseDelta).
    #[test]
    fn tool_input_delta_maps_to_tool_input_delta_event() {
        let tool_input = ProviderEvent::ToolCallInputDelta {
            id: "c42".into(),
            delta: "{\"command\": ".into(),
        };
        let ev: Event = tool_input.into();
        if let Event::ToolInputDelta { id, content } = ev {
            assert_eq!(id, "c42");
            assert_eq!(content, "{\"command\": ");
        } else {
            panic!("Expected ToolInputDelta, got {:?}", ev);
        }
    }
}
