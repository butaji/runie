//! Durable event conversion for JSONL persistence.

use super::Event;

impl Event {
    /// Convert this event to a durable core event for JSONL persistence.
    /// Returns `None` for transient-only events (keystrokes, scroll, streaming deltas).
    pub fn to_durable(&self) -> Option<crate::event::DurableCoreEvent> {
        use crate::event::DurableCoreEvent;
        match self {
            Event::ResponseDelta { .. } => None,
            Event::Response { id, content } => Some(DurableCoreEvent::MessageSent {
                id: id.clone(),
                role: "assistant".into(),
                content: content.clone(),
                timestamp: crate::model::now(),
                provider: String::new(),
            }),
            Event::ToolStart { id, name, input } => Some(DurableCoreEvent::ToolCalled {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            }),
            Event::ToolEnd { id, output, .. } => Some(DurableCoreEvent::ToolResult {
                id: id.clone(),
                output: output.clone(),
                success: true,
            }),
            Event::SwitchModel { provider, model, .. } => Some(DurableCoreEvent::ModelSwitched {
                provider: provider.clone(),
                model: model.clone(),
            }),
            Event::RunNameCommand { name } => {
                Some(DurableCoreEvent::SessionRenamed { name: name.clone() })
            }
            _ => None,
        }
    }
}
