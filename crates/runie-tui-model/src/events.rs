//! Pure projections from Pi/Runie events into renderer-independent TUI
//! intents. No terminal or widget types belong in this module.

use runie_core::types::{AgentEvent, AgentMessage, AssistantMessageEvent};

use crate::{Status, StatusMsg};

/// Map status-owned event transitions without reading or mutating actor state.
#[allow(
    clippy::too_many_lines,
    reason = "the model event table stays exhaustive and directly auditable"
)]
pub fn status_messages_for_event(event: &AgentEvent) -> Vec<StatusMsg> {
    match event {
        AgentEvent::AgentStart => vec![StatusMsg::Set(Status::Thinking)],
        AgentEvent::Error { message } => vec![StatusMsg::Set(Status::Error(message.clone()))],
        AgentEvent::TurnStart => vec![StatusMsg::BeginTurn, StatusMsg::Set(Status::Thinking)],
        AgentEvent::Waiting { reason } => {
            vec![StatusMsg::Set(Status::Waiting(reason.clone()))]
        }
        AgentEvent::ThemeChanged { theme } => vec![StatusMsg::SetTheme(*theme)],
        AgentEvent::Reset => vec![StatusMsg::Set(Status::Ready)],
        AgentEvent::TurnEnd { .. } | AgentEvent::AgentEnd { .. } => {
            vec![StatusMsg::Set(Status::Ready)]
        }
        AgentEvent::MessageUpdate { event, .. } => match event {
            AssistantMessageEvent::TextDelta { .. } => vec![StatusMsg::Set(Status::Streaming)],
            AssistantMessageEvent::ThinkingDelta { .. } => vec![StatusMsg::Set(Status::Thinking)],
            AssistantMessageEvent::Done {
                stop_reason, usage, ..
            } => vec![
                StatusMsg::FinishTurn(usage.clone(), *stop_reason),
                StatusMsg::Set(Status::Ready),
            ],
            AssistantMessageEvent::Error { error, .. } => {
                vec![StatusMsg::Set(Status::Error(error.error_text()))]
            }
            _ => Vec::new(),
        },
        AgentEvent::MessageEnd {
            message: AgentMessage::Assistant(assistant),
        } => {
            if let Some(error) = assistant.error_message.as_ref() {
                vec![StatusMsg::Set(Status::Error(error.clone()))]
            } else if let Some(stop_reason) = assistant.stop_reason {
                vec![
                    StatusMsg::FinishTurn(assistant.usage.clone(), stop_reason),
                    StatusMsg::Set(Status::Ready),
                ]
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    }
}
