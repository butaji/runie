//! Pure projections from Pi/Runie events into renderer-independent TUI
//! intents. No terminal or widget types belong in this module.

use runie_core::types::{AgentEvent, AgentMessage, AssistantMessageEvent};

use crate::{
    default_tool_display_mode, format_clock_timestamp, format_elapsed, format_error, Line,
    LineKind, ScrollbackMsg, Status, StatusMsg, PROMPT_TIMESTAMP_LIVE_THRESHOLD,
};

#[allow(
    clippy::too_many_lines,
    reason = "the event projection table keeps actor-owned mappings declarative"
)]
pub fn scrollback_messages_for_event(event: &AgentEvent) -> Vec<ScrollbackMsg> {
    match event {
        AgentEvent::MessageStart {
            message: runie_core::types::AgentMessage::User(user),
        } => {
            let text = user
                .content
                .iter()
                .map(|content| match content {
                    runie_core::types::UserContent::Text { text } => text.as_str(),
                    runie_core::types::UserContent::Image { .. } => "[image]",
                })
                .collect::<Vec<_>>()
                .join("");
            let mut messages = vec![
                ScrollbackMsg::ActivityReset,
                ScrollbackMsg::Append(Line::new(LineKind::User, text).with_vpad(true)),
            ];
            if user.timestamp >= PROMPT_TIMESTAMP_LIVE_THRESHOLD {
                messages.push(ScrollbackMsg::SetPromptTimestamp(Some(
                    format_clock_timestamp(user.timestamp),
                )));
            }
            messages
        }
        AgentEvent::MessageStart {
            message: runie_core::types::AgentMessage::Assistant(_),
        } => vec![
            ScrollbackMsg::AssistantStreamStart,
            ScrollbackMsg::Append(Line::new(LineKind::Separator, "")),
            ScrollbackMsg::Append(Line::new(LineKind::ThinkingStatus, "◆ Thinking…")),
            ScrollbackMsg::Append(Line::new(LineKind::Separator, "")),
            ScrollbackMsg::Append(Line::new(LineKind::Assistant, "")),
        ],
        AgentEvent::MessageUpdate {
            event: AssistantMessageEvent::TextDelta { delta, .. },
            ..
        } => vec![ScrollbackMsg::AppendToLastByKind(
            LineKind::Assistant,
            delta.clone(),
        )],
        AgentEvent::MessageUpdate {
            event: AssistantMessageEvent::ThinkingDelta { delta, .. },
            ..
        } => vec![ScrollbackMsg::AppendToLastByKind(
            LineKind::Reasoning,
            delta.clone(),
        )],
        AgentEvent::Reset => vec![ScrollbackMsg::Clear],
        AgentEvent::MessageEnd {
            message: runie_core::types::AgentMessage::Assistant(_),
        } => vec![ScrollbackMsg::AssistantStreamEnd],
        AgentEvent::ThemeChanged { theme } => vec![ScrollbackMsg::SetTheme(*theme)],
        AgentEvent::ModelChanged { .. } => Vec::new(),
        AgentEvent::ToolDisplayModeChanged { tool_call_id, mode } => {
            vec![ScrollbackMsg::SetToolMode(tool_call_id.clone(), *mode)]
        }
        AgentEvent::ToolExecutionStart {
            tool_call_id,
            tool_name,
            args,
            ..
        } => vec![
            ScrollbackMsg::SetToolName(tool_call_id.clone(), tool_name.clone()),
            ScrollbackMsg::SetToolArgs(tool_call_id.clone(), args.clone()),
            ScrollbackMsg::ActivityToolStart(tool_name.clone()),
            ScrollbackMsg::SetToolMode(tool_call_id.clone(), default_tool_display_mode(tool_name)),
        ],
        AgentEvent::BackgroundWorkStarted {
            work_id,
            description,
            background,
        } => vec![ScrollbackMsg::ToolStart {
            tool_call_id: work_id.clone(),
            header: format!(
                "Subagent {}: {description:?}",
                if *background { "started" } else { "running" }
            ),
            activity: None,
        }],
        AgentEvent::BackgroundWorkProgress {
            work_id,
            description,
            activity,
        } => vec![ScrollbackMsg::ToolUpdate {
            tool_call_id: work_id.clone(),
            header: Some(format!("Subagent running: {description:?} — {activity}")),
            output: Vec::new(),
        }],
        AgentEvent::BackgroundWorkFinished {
            work_id,
            description,
            is_error,
            elapsed_ms,
            error,
        } => {
            let mut messages = vec![ScrollbackMsg::ToolEnd {
                tool_call_id: work_id.clone(),
                header: format!(
                    "Subagent {}{}{}: {description:?}",
                    if *is_error { "failed" } else { "completed" },
                    format_elapsed(*elapsed_ms),
                    format_error(*is_error, error.as_deref())
                ),
                activity: None,
                output: Vec::new(),
            }];
            if *is_error {
                messages.push(ScrollbackMsg::MarkToolError(work_id.clone()));
            }
            messages
        }
        AgentEvent::BackgroundWorkCancelled {
            work_id,
            description,
            elapsed_ms,
        } => vec![
            ScrollbackMsg::ToolEnd {
                tool_call_id: work_id.clone(),
                header: format!(
                    "Subagent cancelled{}: {description:?}",
                    format_elapsed(*elapsed_ms)
                ),
                activity: None,
                output: Vec::new(),
            },
            ScrollbackMsg::MarkToolError(work_id.clone()),
        ],
        AgentEvent::WorkflowStarted {
            run_id,
            name,
            objective,
        } => vec![
            ScrollbackMsg::SetToolName(run_id.clone(), "workflow".into()),
            ScrollbackMsg::WorkflowStart {
                run_id: run_id.clone(),
                name: name.clone(),
                objective: objective.clone(),
            },
        ],
        AgentEvent::WorkflowProgress {
            run_id,
            phase,
            state,
            active_agents,
        } => vec![ScrollbackMsg::WorkflowProgress {
            run_id: run_id.clone(),
            phase: phase.clone(),
            state: state.clone(),
            active_agents: *active_agents,
        }],
        AgentEvent::WorkflowFinished {
            run_id,
            status,
            elapsed_ms,
        } => vec![ScrollbackMsg::WorkflowEnd {
            run_id: run_id.clone(),
            status: status.clone(),
            elapsed_ms: *elapsed_ms,
        }],
        AgentEvent::ToolExecutionEnd {
            tool_call_id,
            is_error,
            ..
        } => {
            vec![
                ScrollbackMsg::ActivityToolEnd {
                    is_error: *is_error,
                },
                ScrollbackMsg::RemoveToolArgs(tool_call_id.clone()),
            ]
        }
        AgentEvent::AgentStart
        | AgentEvent::AgentEnd { .. }
        | AgentEvent::Error { .. }
        | AgentEvent::ThinkingLevelChanged { .. }
        | AgentEvent::ActiveToolsChanged { .. }
        | AgentEvent::SessionLabelChanged { .. }
        | AgentEvent::SessionNameChanged { .. }
        | AgentEvent::SessionLaneChanged { .. }
        | AgentEvent::SessionEntryAppended { .. }
        | AgentEvent::BranchSummaryCreated { .. }
        | AgentEvent::CustomSessionEntryCreated { .. }
        | AgentEvent::CompactionCreated { .. }
        | AgentEvent::OperationRecordCreated { .. }
        | AgentEvent::TypedOperationRecordCreated { .. }
        | AgentEvent::TurnStart
        | AgentEvent::Waiting { .. }
        | AgentEvent::TurnEnd { .. }
        | AgentEvent::MessageStart { .. }
        | AgentEvent::MessageUpdate { .. }
        | AgentEvent::MessageEnd { .. }
        | AgentEvent::ToolExecutionUpdate { .. } => Vec::new(),
    }
}

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
        AgentEvent::Reset => vec![StatusMsg::Reset],
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
        | AgentEvent::ActiveToolsChanged { .. }
        | AgentEvent::SessionLabelChanged { .. }
        | AgentEvent::SessionNameChanged { .. }
        | AgentEvent::SessionLaneChanged { .. }
        | AgentEvent::SessionEntryAppended { .. }
        | AgentEvent::BranchSummaryCreated { .. }
        | AgentEvent::CustomSessionEntryCreated { .. }
        | AgentEvent::CompactionCreated { .. }
        | AgentEvent::OperationRecordCreated { .. }
        | AgentEvent::TypedOperationRecordCreated { .. }
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
    use super::{is_actor_feed_event, scrollback_messages_for_event, status_messages_for_event};
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
    fn scrollback_projection_is_model_owned_for_feed_events() {
        let messages = scrollback_messages_for_event(&AgentEvent::ToolExecutionStart {
            tool_call_id: "call-1".into(),
            tool_name: "bash".into(),
            args: serde_json::json!({"cmd": "pwd"}),
        });
        assert_eq!(messages.len(), 4);
        assert_eq!(
            messages[0],
            super::ScrollbackMsg::SetToolName("call-1".into(), "bash".into())
        );
        assert_eq!(
            messages[1],
            super::ScrollbackMsg::SetToolArgs("call-1".into(), serde_json::json!({"cmd": "pwd"}))
        );
        assert_eq!(
            messages[2],
            super::ScrollbackMsg::ActivityToolStart("bash".into())
        );
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
