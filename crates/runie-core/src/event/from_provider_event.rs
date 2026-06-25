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
