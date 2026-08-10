//! Pure projections from Pi/Runie events into renderer-independent TUI
//! intents. No terminal or widget types belong in this module.

use runie_core::types::{AgentEvent, AgentMessage, AssistantMessageEvent};

use crate::{
    default_tool_display_mode, format_clock_timestamp, format_elapsed, format_error,
    ui_messages_for_event, Line, LineKind, ScrollbackMsg, Status, StatusMsg, UiMsg,
    PROMPT_TIMESTAMP_LIVE_THRESHOLD,
};

/// All renderer-independent projections for one core event.
///
/// Keeping this as one value makes the event fan-out inspectable and gives
/// actors a shared projection boundary. Each actor still consumes only its
/// own field and remains the sole owner of its state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventProjection {
    pub scope: EventProjectionScope,
    pub feed: Vec<ScrollbackMsg>,
    pub status: Vec<StatusMsg>,
    pub ui: Vec<UiMsg>,
}

pub fn project_event(event: &AgentEvent) -> EventProjection {
    EventProjection {
        scope: event_projection_scope(event),
        feed: scrollback_messages_for_event(event),
        status: crate::status_messages_for_event(event),
        ui: ui_messages_for_event(event),
    }
}

/// Declarative ownership scopes for one core event.
///
/// An event may be delivered to more than one owning actor (for example,
/// `ThemeChanged` updates both feed and status). The classifier only describes
/// delivery ownership; each actor still reduces its own mailbox messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventProjectionScope(u8);

impl EventProjectionScope {
    pub const NONE: Self = Self(0);
    pub const FEED: Self = Self(1 << 0);
    pub const STATUS: Self = Self(1 << 1);
    pub const TRANSCRIPT: Self = Self(1 << 2);
    pub const SESSION: Self = Self(1 << 3);

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn contains(self, scope: Self) -> bool {
        self.0 & scope.0 == scope.0
    }
}

/// Classify every event by the actors whose projections may consume it.
pub fn event_projection_scope(event: &AgentEvent) -> EventProjectionScope {
    use EventProjectionScope as Scope;
    match event {
        AgentEvent::AgentStart
        | AgentEvent::Error { .. }
        | AgentEvent::TurnStart
        | AgentEvent::Waiting { .. }
        | AgentEvent::TurnEnd { .. }
        | AgentEvent::AgentEnd { .. } => Scope::STATUS,
        AgentEvent::ThemeChanged { .. } | AgentEvent::Reset => Scope::FEED.union(Scope::STATUS),
        AgentEvent::ModelChanged { .. } => Scope::STATUS,
        AgentEvent::MessageStart { .. } => Scope::TRANSCRIPT,
        AgentEvent::MessageUpdate { .. } | AgentEvent::MessageEnd { .. } => {
            Scope::TRANSCRIPT.union(Scope::STATUS)
        }
        AgentEvent::ToolDisplayModeChanged { .. }
        | AgentEvent::ToolExecutionStart { .. }
        | AgentEvent::ToolExecutionUpdate { .. }
        | AgentEvent::ToolExecutionEnd { .. }
        | AgentEvent::BackgroundWorkStarted { .. }
        | AgentEvent::BackgroundWorkProgress { .. }
        | AgentEvent::BackgroundWorkFinished { .. }
        | AgentEvent::BackgroundWorkCancelled { .. }
        | AgentEvent::WorkflowStarted { .. }
        | AgentEvent::WorkflowProgress { .. }
        | AgentEvent::WorkflowFinished { .. } => Scope::FEED,
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
        | AgentEvent::TypedOperationRecordCreated { .. } => Scope::SESSION,
    }
}

pub fn scrollback_messages_for_event(event: &AgentEvent) -> Vec<ScrollbackMsg> {
    if let Some(messages) = simple_scrollback_event(event) {
        return messages;
    }
    tool_scrollback_event(event)
        .or_else(|| background_scrollback_event(event))
        .or_else(|| workflow_scrollback_event(event))
        .unwrap_or_default()
}

fn tool_scrollback_event(event: &AgentEvent) -> Option<Vec<ScrollbackMsg>> {
    Some(match event {
        AgentEvent::ToolExecutionStart {
            tool_call_id,
            tool_name,
            args,
            ..
        } => tool_start(tool_call_id, tool_name, args),
        _ => return None,
    })
}

fn background_scrollback_event(event: &AgentEvent) -> Option<Vec<ScrollbackMsg>> {
    Some(match event {
        AgentEvent::BackgroundWorkStarted {
            work_id,
            description,
            background,
        } => background_start(work_id, description, *background),
        AgentEvent::BackgroundWorkProgress {
            work_id,
            description,
            activity,
        } => background_progress(work_id, description, activity),
        AgentEvent::BackgroundWorkFinished {
            work_id,
            description,
            is_error,
            elapsed_ms,
            error,
        } => background_finished(
            work_id,
            description,
            *is_error,
            *elapsed_ms,
            error.as_deref(),
        ),
        AgentEvent::BackgroundWorkCancelled {
            work_id,
            description,
            elapsed_ms,
        } => background_cancelled(work_id, description, *elapsed_ms),
        _ => return None,
    })
}

fn workflow_scrollback_event(event: &AgentEvent) -> Option<Vec<ScrollbackMsg>> {
    Some(match event {
        AgentEvent::WorkflowStarted {
            run_id,
            name,
            objective,
        } => workflow_started(run_id, name, objective),
        AgentEvent::WorkflowProgress {
            run_id,
            phase,
            state,
            active_agents,
        } => workflow_progress(run_id, phase, state, *active_agents),
        AgentEvent::WorkflowFinished {
            run_id,
            status,
            elapsed_ms,
        } => workflow_finished(run_id, status, elapsed_ms),
        AgentEvent::ToolExecutionEnd {
            tool_call_id,
            is_error,
            ..
        } => vec![
            ScrollbackMsg::ActivityToolEnd {
                is_error: *is_error,
            },
            ScrollbackMsg::RemoveToolArgs(tool_call_id.clone()),
        ],
        _ => return None,
    })
}

fn simple_scrollback_event(event: &AgentEvent) -> Option<Vec<ScrollbackMsg>> {
    Some(match event {
        AgentEvent::MessageStart {
            message: AgentMessage::User(user),
        } => user_start(user),
        AgentEvent::MessageStart {
            message: AgentMessage::Assistant(_),
        } => assistant_start(),
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
            message: AgentMessage::Assistant(_),
        } => vec![ScrollbackMsg::AssistantStreamEnd],
        AgentEvent::ThemeChanged { theme } => vec![ScrollbackMsg::SetTheme(*theme)],
        AgentEvent::ToolDisplayModeChanged { tool_call_id, mode } => {
            vec![ScrollbackMsg::SetToolMode(tool_call_id.clone(), *mode)]
        }
        _ => return None,
    })
}

fn user_start(user: &runie_core::types::UserMessage) -> Vec<ScrollbackMsg> {
    let text = user
        .content
        .iter()
        .map(|content| match content {
            runie_core::types::UserContent::Text { text } => text.as_str(),
            runie_core::types::UserContent::Image { .. } => "[image]",
            runie_core::types::UserContent::Video { .. } => "[video]",
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

fn assistant_start() -> Vec<ScrollbackMsg> {
    vec![
        ScrollbackMsg::AssistantStreamStart,
        ScrollbackMsg::Append(Line::new(LineKind::Separator, "")),
        ScrollbackMsg::Append(Line::new(LineKind::ThinkingStatus, "◆ Thinking…")),
        ScrollbackMsg::Append(Line::new(LineKind::Separator, "")),
        ScrollbackMsg::Append(Line::new(LineKind::Assistant, "")),
    ]
}

fn tool_start(id: &str, name: &str, args: &serde_json::Value) -> Vec<ScrollbackMsg> {
    vec![
        ScrollbackMsg::SetToolName(id.to_owned(), name.to_owned()),
        ScrollbackMsg::SetToolArgs(id.to_owned(), args.clone()),
        ScrollbackMsg::ActivityToolStart(name.to_owned()),
        ScrollbackMsg::SetToolMode(id.to_owned(), default_tool_display_mode(name)),
    ]
}

fn background_start(id: &str, description: &str, background: bool) -> Vec<ScrollbackMsg> {
    vec![ScrollbackMsg::ToolStart {
        tool_call_id: id.to_owned(),
        header: format!(
            "Subagent {}: {description:?}",
            if background { "started" } else { "running" }
        ),
        activity: None,
    }]
}

fn background_progress(id: &str, description: &str, activity: &str) -> Vec<ScrollbackMsg> {
    vec![ScrollbackMsg::ToolUpdate {
        tool_call_id: id.to_owned(),
        header: Some(format!("Subagent running: {description:?} — {activity}")),
        output: Vec::new(),
    }]
}

fn background_cancelled(
    id: &str,
    description: &str,
    elapsed_ms: Option<u64>,
) -> Vec<ScrollbackMsg> {
    vec![
        ScrollbackMsg::ToolEnd {
            tool_call_id: id.to_owned(),
            header: format!(
                "Subagent cancelled{}: {description:?}",
                format_elapsed(elapsed_ms)
            ),
            activity: None,
            output: Vec::new(),
        },
        ScrollbackMsg::MarkToolError(id.to_owned()),
    ]
}

fn workflow_started(id: &str, name: &str, objective: &str) -> Vec<ScrollbackMsg> {
    vec![
        ScrollbackMsg::SetToolName(id.to_owned(), "workflow".into()),
        ScrollbackMsg::WorkflowStart {
            run_id: id.to_owned(),
            name: name.to_owned(),
            objective: objective.to_owned(),
        },
    ]
}

fn workflow_progress(id: &str, phase: &str, state: &str, active_agents: u32) -> Vec<ScrollbackMsg> {
    vec![ScrollbackMsg::WorkflowProgress {
        run_id: id.to_owned(),
        phase: phase.to_owned(),
        state: state.to_owned(),
        active_agents,
    }]
}

fn workflow_finished(id: &str, status: &str, elapsed_ms: &Option<u64>) -> Vec<ScrollbackMsg> {
    vec![ScrollbackMsg::WorkflowEnd {
        run_id: id.to_owned(),
        status: status.to_owned(),
        elapsed_ms: *elapsed_ms,
    }]
}

fn background_finished(
    id: &str,
    description: &str,
    is_error: bool,
    elapsed_ms: Option<u64>,
    error: Option<&str>,
) -> Vec<ScrollbackMsg> {
    let mut messages = vec![ScrollbackMsg::ToolEnd {
        tool_call_id: id.to_owned(),
        header: format!(
            "Subagent {}{}{}: {description:?}",
            if is_error { "failed" } else { "completed" },
            format_elapsed(elapsed_ms),
            format_error(is_error, error)
        ),
        activity: None,
        output: Vec::new(),
    }];
    if is_error {
        messages.push(ScrollbackMsg::MarkToolError(id.to_owned()));
    }
    messages
}

/// Events that the live feed actor is allowed to project from the shared bus.
///
/// Transcript message events are intentionally excluded: the compatibility
/// renderer owns their delivery in the current live adapter. Keeping this
/// policy in the model makes that boundary declarative and prevents a future
/// mapper from accidentally appending the same user/assistant row twice.
pub fn is_actor_feed_event(event: &AgentEvent) -> bool {
    event_projection_scope(event).contains(EventProjectionScope::FEED)
}

/// Map status-owned event transitions without reading or mutating actor state.
pub fn status_messages_for_event(event: &AgentEvent) -> Vec<StatusMsg> {
    match event {
        AgentEvent::AgentStart => vec![StatusMsg::Set(Status::Thinking)],
        AgentEvent::Error { message } => vec![StatusMsg::Set(Status::Error(message.clone()))],
        AgentEvent::TurnStart => vec![StatusMsg::BeginTurn, StatusMsg::Set(Status::Thinking)],
        AgentEvent::Waiting { reason } => vec![StatusMsg::Set(Status::Waiting(reason.clone()))],
        AgentEvent::ThemeChanged { theme } => vec![StatusMsg::SetTheme(*theme)],
        AgentEvent::ModelChanged { model } => model_status(model.context_window),
        AgentEvent::Reset => vec![StatusMsg::Reset],
        AgentEvent::TurnEnd { .. } | AgentEvent::AgentEnd { .. } => {
            vec![StatusMsg::Set(Status::Ready)]
        }
        AgentEvent::MessageUpdate { event, .. } => status_update(event),
        AgentEvent::MessageEnd {
            message: AgentMessage::Assistant(assistant),
        } => status_message_end(assistant),
        _ => Vec::new(),
    }
}

fn model_status(context_window: u64) -> Vec<StatusMsg> {
    vec![StatusMsg::SetContextWindow(
        (context_window > 0).then_some(context_window),
    )]
}

fn status_update(event: &AssistantMessageEvent) -> Vec<StatusMsg> {
    match event {
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
        _ => Vec::new(),
    }
}

fn status_message_end(assistant: &runie_core::types::AssistantMessage) -> Vec<StatusMsg> {
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

#[cfg(test)]
#[path = "events_tests.rs"]
mod tests;
