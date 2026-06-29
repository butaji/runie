//! Durable event conversion for JSONL persistence.

use super::Event;

impl Event {
    /// Convert this event to a durable core event for JSONL persistence.
    /// Returns `None` for transient-only events (keystrokes, scroll, streaming deltas).
    pub fn to_durable(&self) -> Option<crate::event::DurableCoreEvent> {
        use crate::event::DurableCoreEvent;
        match self {
            // Streaming (transient)
            Event::ResponseDelta { .. }
            | Event::TextStart { .. }
            | Event::TextEnd { .. }
            | Event::ThinkingStart { .. }
            | Event::ThinkingDelta { .. }
            | Event::ThinkingEnd { .. } => None,
            Event::ToolStart { id, name, input } => Some(tool_called(id, name, input)),
            Event::ToolEnd { id, output, .. } => Some(tool_result(id, output)),
            Event::Response { id, content } => Some(message_sent(id, content)),
            Event::SwitchModel {
                provider, model, ..
            } => Some(model_switched(provider, model)),
            Event::RunNameCommand { name } => {
                Some(DurableCoreEvent::SessionRenamed { name: name.clone() })
            }
            Event::SwitchTheme { name } => {
                Some(DurableCoreEvent::ThemeSwitched { name: name.clone() })
            }
            Event::SetThinkingLevel(level) => {
                Some(DurableCoreEvent::ThinkingLevelSet { level: *level })
            }
            _ => None,
        }
    }
}

fn tool_called(id: &str, name: &str, input: &serde_json::Value) -> crate::event::DurableCoreEvent {
    crate::event::DurableCoreEvent::ToolCalled {
        id: id.to_owned(),
        name: name.to_owned(),
        input: input.clone(),
    }
}

fn tool_result(id: &str, output: &str) -> crate::event::DurableCoreEvent {
    crate::event::DurableCoreEvent::ToolResult {
        id: id.to_owned(),
        output: output.to_owned(),
        success: true,
    }
}

fn message_sent(id: &str, content: &str) -> crate::event::DurableCoreEvent {
    crate::event::DurableCoreEvent::MessageSent {
        id: id.to_owned(),
        role: "assistant".into(),
        content: content.to_owned(),
        timestamp: crate::model::now(),
        provider: String::new(),
    }
}

fn model_switched(provider: &str, model: &str) -> crate::event::DurableCoreEvent {
    crate::event::DurableCoreEvent::ModelSwitched {
        provider: provider.to_owned(),
        model: model.to_owned(),
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
}
