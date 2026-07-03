//! Mock API event builders.

use runie_core::event::Event;
use runie_core::provider_event::{ProviderEvent, StopReason};

/// Event emitted when a response stream starts.
pub fn ev_response_created(id: impl Into<String>) -> Event {
    Event::response(id, String::new())
}

/// Text delta event for a response stream.
pub fn ev_output_text_delta(id: impl Into<String>, text: impl Into<String>) -> Event {
    Event::ResponseDelta {
        id: id.into(),
        content: text.into(),
    }
}

/// Event emitted when a turn completes successfully.
pub fn ev_completed(id: impl Into<String>) -> Event {
    Event::Done { id: id.into() }
}

/// Event emitted when a turn fails.
pub fn ev_error(id: impl Into<String>, message: impl Into<String>) -> Event {
    Event::Error {
        id: id.into(),
        message: message.into(),
    }
}

/// Build a `ProviderEvent::TextDelta`.
pub fn llm_text_delta(text: impl Into<String>) -> ProviderEvent {
    ProviderEvent::TextDelta(text.into())
}

/// Build a `ProviderEvent::Finish`.
pub fn llm_finish() -> ProviderEvent {
    ProviderEvent::Finish {
        reason: StopReason::Stop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_build_expected_variants() {
        assert!(matches!(ev_response_created("1"), Event::Response { .. }));
        assert!(matches!(
            ev_output_text_delta("1", "hi"),
            Event::ResponseDelta { .. }
        ));
        assert!(matches!(ev_completed("1"), Event::Done { .. }));
        assert!(matches!(ev_error("1", "oops"), Event::Error { .. }));
    }
}
