//! Pure projections from Pi/Runie events into renderer-independent TUI
//! intents. No terminal or widget types belong in this module.

use runie_core::types::{AgentEvent, AgentMessage, AssistantMessageEvent};

use crate::{Status, StatusMsg};

/// Events that the live feed actor is allowed to project from the shared bus.
///
/// Transcript message events are intentionally excluded: the compatibility
/// renderer owns their delivery in the current live adapter. Keeping this
/// policy in the model makes that boundary declarative and prevents a future
/// mapper from accidentally appending the same user/assistant row twice.
pub fn is_actor_feed_event(event: &AgentEvent) -> bool {
    matches!(
        event,
        AgentEvent::Reset
            | AgentEvent::ThemeChanged { .. }
            | AgentEvent::ToolDisplayModeChanged { .. }
            | AgentEvent::ToolExecutionStart { .. }
            | AgentEvent::ToolExecutionUpdate { .. }
            | AgentEvent::ToolExecutionEnd { .. }
            | AgentEvent::BackgroundWorkStarted { .. }
            | AgentEvent::BackgroundWorkProgress { .. }
            | AgentEvent::BackgroundWorkFinished { .. }
            | AgentEvent::BackgroundWorkCancelled { .. }
            | AgentEvent::WorkflowStarted { .. }
            | AgentEvent::WorkflowProgress { .. }
            | AgentEvent::WorkflowFinished { .. }
    )
}

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
        AgentEvent::ModelChanged { model } => vec![StatusMsg::SetContextWindow(
            (model.context_window > 0).then_some(model.context_window),
        )],
        AgentEvent::Reset => vec![StatusMsg::Set(Status::Ready)],
        AgentEvent::TurnEnd { .. } | AgentEvent::AgentEnd { .. } => {
            vec![StatusMsg::Set(Status::Ready)]
        }
        AgentEvent::MessageUpdate { event, .. } => match event {
            AssistantMessageEvent::TextDelta { .. } => vec![StatusMsg::Set(Status::Streaming)],
            AssistantMessageEvent::ThinkingDelta { .. } => vec![StatusMsg::Set(Status::Thinking)],
            AssistantMessageEvent::ThinkingEnd { elapsed_ms, .. } => {
                vec![StatusMsg::SetThinkingElapsed(*elapsed_ms)]
            }
            AssistantMessageEvent::Done {
                stop_reason, usage, ..
            } => vec![
                StatusMsg::FinishTurn(usage.clone(), *stop_reason),
                StatusMsg::Set(Status::Ready),
            ],
            AssistantMessageEvent::Error { error, .. } => {
                vec![StatusMsg::Set(Status::Error(error.error_text()))]
            }
            AssistantMessageEvent::Start { .. }
            | AssistantMessageEvent::TextStart { .. }
            | AssistantMessageEvent::TextEnd { .. }
            | AssistantMessageEvent::ThinkingStart { .. }
            | AssistantMessageEvent::ToolCallStart { .. }
            | AssistantMessageEvent::ToolCallDelta { .. }
            | AssistantMessageEvent::ToolCallEnd { .. } => Vec::new(),
        },
        AgentEvent::MessageEnd {
            message: AgentMessage::Assistant(assistant),
        } => {
            let mut messages = vec![StatusMsg::SetThinkingElapsed(assistant.thinking_elapsed_ms)];
            if let Some(error) = assistant.error_message.as_ref() {
                messages.push(StatusMsg::Set(Status::Error(error.clone())));
            } else if let Some(stop_reason) = assistant.stop_reason {
                messages.extend([
                    StatusMsg::FinishTurn(assistant.usage.clone(), stop_reason),
                    StatusMsg::Set(Status::Ready),
                ]);
            }
            messages
        }
        AgentEvent::ThinkingLevelChanged { .. }
        | AgentEvent::ToolDisplayModeChanged { .. }
        | AgentEvent::MessageStart { .. }
        | AgentEvent::MessageEnd { .. }
        | AgentEvent::ToolExecutionStart { .. }
        | AgentEvent::ToolExecutionUpdate { .. }
        | AgentEvent::ToolExecutionEnd { .. }
        | AgentEvent::BackgroundWorkStarted { .. }
        | AgentEvent::BackgroundWorkProgress { .. }
        | AgentEvent::BackgroundWorkFinished { .. }
        | AgentEvent::BackgroundWorkCancelled { .. }
        | AgentEvent::WorkflowStarted { .. }
        | AgentEvent::WorkflowProgress { .. }
        | AgentEvent::WorkflowFinished { .. } => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{is_actor_feed_event, status_messages_for_event};
    use runie_core::types::{AgentEvent, AgentMessage, AssistantMessage, AssistantMessageEvent};

    #[test]
    fn actor_feed_scope_excludes_transcript_messages() {
        assert!(!is_actor_feed_event(&AgentEvent::TurnStart));
        assert!(!is_actor_feed_event(&AgentEvent::AgentEnd {
            messages: vec![]
        }));
        assert!(is_actor_feed_event(&AgentEvent::Reset));
        assert!(is_actor_feed_event(&AgentEvent::ThemeChanged {
            theme: runie_core::types::ThemeKind::GrokNight,
        }));
    }

    #[test]
    fn thinking_duration_is_delivered_to_the_status_actor() {
        let messages = status_messages_for_event(&AgentEvent::MessageUpdate {
            message: AgentMessage::Assistant(AssistantMessage::default()),
            event: AssistantMessageEvent::ThinkingEnd {
                index: 0,
                content: "reasoning".into(),
                partial: AssistantMessage::default(),
                elapsed_ms: Some(900),
            },
        });
        assert_eq!(
            messages,
            vec![super::StatusMsg::SetThinkingElapsed(Some(900))]
        );
    }
}
