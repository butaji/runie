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
            // Text lifecycle
            PE::TextStart { id } => Event::TextStart { id },
            PE::TextDelta(content) => Event::ResponseDelta {
                id: String::new(), // id not carried in TextDelta
                content,
            },
            PE::TextEnd { id } => Event::TextEnd { id },
            // Thinking lifecycle
            PE::ThinkingStart { id } => Event::ThinkingStart { id },
            PE::ThinkingDelta(content) => Event::ThinkingDelta { id: String::new(), content },
            PE::ThinkingEnd { id } => Event::ThinkingEnd { id },
            // Tool lifecycle
            PE::ToolCallStart { id, name } => Event::ToolStart { id, name, input: Default::default() },
            PE::ToolCallInputDelta { id, delta } => {
                // Accumulated in StreamState; no direct Event equivalent
                Event::ResponseDelta { id, content: delta }
            }
            PE::ToolCallEnd { id } => Event::ToolEnd {
                id,
                duration_secs: 0.0,
                output: String::new(),
            },
            // LLM lifecycle
            PE::Finish { reason: _ } => {
                // Turn completion is signaled by Done event from agent turn code
                Event::Done { id: String::new() }
            }
            PE::Error(e) => Event::Error {
                id: String::new(),
                message: e.to_string(),
            },
            PE::Usage { .. } => {
                // Usage info is tracked internally; no UI event needed
                Event::ResponseDelta { id: String::new(), content: String::new() }
            }
        }
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
            ProviderEvent::ToolCallStart { id: "c1".into(), name: "bash".into() },
            ProviderEvent::ToolCallInputDelta { id: "c1".into(), delta: "-la".into() },
            ProviderEvent::ToolCallEnd { id: "c1".into() },
            ProviderEvent::Error(crate::provider_event::ModelError::Other("oops".into())),
            ProviderEvent::Usage { input_tokens: 100, output_tokens: 50 },
            ProviderEvent::Finish { reason: crate::provider_event::StopReason::Stop },
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

        let tool_start = ProviderEvent::ToolCallStart { id: "c99".into(), name: "read".into() };
        let ev: Event = tool_start.into();
        if let Event::ToolStart { id, name, .. } = ev {
            assert_eq!(id, "c99");
            assert_eq!(name, "read");
        } else {
            panic!("Expected ToolStart");
        }
    }
}
