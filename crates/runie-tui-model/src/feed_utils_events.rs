use super::super::*;
use crate::events::is_actor_feed_event;

/// Project a `ToolExecutionUpdate` event into the structured-output
/// `ScrollbackMsg::ToolUpdate` rows. The helper is pure: it only reads
/// the active-tool set and the event, returning zero or one tool-update
/// message for the actor-owned scrollback to reduce.
pub fn structured_update_messages(
    active_tools: &std::collections::HashSet<String>,
    event: &runie_core::types::AgentEvent,
) -> Vec<ScrollbackMsg> {
    let runie_core::types::AgentEvent::ToolExecutionUpdate {
        tool_call_id,
        partial_result,
        ..
    } = event
    else {
        return Vec::new();
    };
    if !active_tools.contains(tool_call_id) {
        return Vec::new();
    }
    let Some(output) = structured_update_text(partial_result) else {
        return Vec::new();
    };
    let output = output
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if output.is_empty() {
        Vec::new()
    } else {
        vec![ScrollbackMsg::ToolUpdate {
            tool_call_id: tool_call_id.clone(),
            header: None,
            output,
        }]
    }
}

/// Project a `BackgroundWork*` event into the canonical scrollback
/// messages. The helper is pure so the actor-owned background work
/// projection and the renderer agree on the subagent lifecycle
/// messages.
#[allow(
    clippy::too_many_lines,
    reason = "background lifecycle formatting keeps Grok card variants explicit"
)]
pub fn background_messages_for_event(event: &runie_core::types::AgentEvent) -> Vec<ScrollbackMsg> {
    match event {
        runie_core::types::AgentEvent::BackgroundWorkStarted { .. } => background_started(event),
        runie_core::types::AgentEvent::BackgroundWorkProgress { .. } => background_progress(event),
        runie_core::types::AgentEvent::BackgroundWorkFinished { .. } => background_finished(event),
        runie_core::types::AgentEvent::BackgroundWorkCancelled { .. } => {
            background_cancelled(event)
        }
        runie_core::types::AgentEvent::WorkflowStarted { .. } => workflow_started(event),
        runie_core::types::AgentEvent::WorkflowProgress { .. } => workflow_progress(event),
        runie_core::types::AgentEvent::WorkflowFinished { .. } => workflow_finished(event),
        _ => Vec::new(),
    }
}

fn background_started(event: &runie_core::types::AgentEvent) -> Vec<ScrollbackMsg> {
    let runie_core::types::AgentEvent::BackgroundWorkStarted {
        work_id,
        description,
        background,
    } = event
    else {
        return Vec::new();
    };
    vec![
        ScrollbackMsg::SetToolName(work_id.clone(), "subagent".into()),
        ScrollbackMsg::SetToolMode(
            work_id.clone(),
            runie_core::types::ToolDisplayMode::Collapsed,
        ),
        ScrollbackMsg::ToolStart {
            tool_call_id: work_id.clone(),
            header: format!(
                "Subagent {}: {description:?}",
                if *background { "started" } else { "running" }
            ),
            activity: None,
        },
    ]
}

fn background_progress(event: &runie_core::types::AgentEvent) -> Vec<ScrollbackMsg> {
    let runie_core::types::AgentEvent::BackgroundWorkProgress {
        work_id,
        description,
        activity,
    } = event
    else {
        return Vec::new();
    };
    vec![ScrollbackMsg::ToolUpdate {
        tool_call_id: work_id.clone(),
        header: Some(format!("Subagent running: {description:?} — {activity}")),
        output: Vec::new(),
    }]
}

fn background_finished(event: &runie_core::types::AgentEvent) -> Vec<ScrollbackMsg> {
    let runie_core::types::AgentEvent::BackgroundWorkFinished {
        work_id,
        description,
        is_error,
        elapsed_ms,
        error,
    } = event
    else {
        return Vec::new();
    };
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

fn background_cancelled(event: &runie_core::types::AgentEvent) -> Vec<ScrollbackMsg> {
    let runie_core::types::AgentEvent::BackgroundWorkCancelled {
        work_id,
        description,
        elapsed_ms,
    } = event
    else {
        return Vec::new();
    };
    vec![
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
    ]
}

fn workflow_started(event: &runie_core::types::AgentEvent) -> Vec<ScrollbackMsg> {
    let runie_core::types::AgentEvent::WorkflowStarted {
        run_id,
        name,
        objective,
    } = event
    else {
        return Vec::new();
    };
    vec![
        ScrollbackMsg::SetToolName(run_id.clone(), "workflow".into()),
        ScrollbackMsg::WorkflowStart {
            run_id: run_id.clone(),
            name: name.clone(),
            objective: objective.clone(),
        },
    ]
}

fn workflow_progress(event: &runie_core::types::AgentEvent) -> Vec<ScrollbackMsg> {
    let runie_core::types::AgentEvent::WorkflowProgress {
        run_id,
        phase,
        state,
        active_agents,
    } = event
    else {
        return Vec::new();
    };
    vec![ScrollbackMsg::WorkflowProgress {
        run_id: run_id.clone(),
        phase: phase.clone(),
        state: state.clone(),
        active_agents: *active_agents,
    }]
}

fn workflow_finished(event: &runie_core::types::AgentEvent) -> Vec<ScrollbackMsg> {
    let runie_core::types::AgentEvent::WorkflowFinished {
        run_id,
        status,
        elapsed_ms,
    } = event
    else {
        return Vec::new();
    };
    vec![ScrollbackMsg::WorkflowEnd {
        run_id: run_id.clone(),
        status: status.clone(),
        elapsed_ms: *elapsed_ms,
    }]
}

/// Project a subscribed `AgentEvent` into the actor-owned scrollback
/// messages that the bus deliverer hands to the actor. The helper
/// drops events that don't belong to the actor feed boundary and
/// delegates `BackgroundWork*` events to the background lifecycle
/// projection. Centralized here so the actor-owned bus projection and
/// the renderer share one canonical shape.
pub fn bus_messages_for_event(event: &runie_core::types::AgentEvent) -> Vec<ScrollbackMsg> {
    if !is_actor_feed_event(event) {
        return Vec::new();
    }
    bus_messages_for_actor_event(event)
}

fn bus_messages_for_actor_event(event: &runie_core::types::AgentEvent) -> Vec<ScrollbackMsg> {
    if let Some(messages) = simple_bus_messages(event) {
        return messages;
    }
    if matches!(
        event,
        runie_core::types::AgentEvent::BackgroundWorkStarted { .. }
            | runie_core::types::AgentEvent::BackgroundWorkProgress { .. }
            | runie_core::types::AgentEvent::BackgroundWorkFinished { .. }
            | runie_core::types::AgentEvent::BackgroundWorkCancelled { .. }
    ) {
        return background_messages_for_event(event);
    }
    Vec::new()
}

fn simple_bus_messages(event: &runie_core::types::AgentEvent) -> Option<Vec<ScrollbackMsg>> {
    if !is_simple_bus_event(event) {
        return None;
    }
    Some(match event {
        runie_core::types::AgentEvent::Reset => vec![ScrollbackMsg::Clear],
        runie_core::types::AgentEvent::ThemeChanged { theme } => {
            vec![ScrollbackMsg::SetTheme(*theme)]
        }
        runie_core::types::AgentEvent::ModelChanged { .. } => Vec::new(),
        runie_core::types::AgentEvent::ToolDisplayModeChanged { tool_call_id, mode } => {
            vec![ScrollbackMsg::SetToolMode(tool_call_id.clone(), *mode)]
        }
        runie_core::types::AgentEvent::ToolExecutionStart {
            tool_call_id,
            tool_name,
            ..
        } => vec![
            ScrollbackMsg::SetToolName(tool_call_id.clone(), tool_name.clone()),
            ScrollbackMsg::SetToolMode(tool_call_id.clone(), default_tool_display_mode(tool_name)),
        ],
        _ => unreachable!("checked by is_simple_bus_event"),
    })
}

fn is_simple_bus_event(event: &runie_core::types::AgentEvent) -> bool {
    matches!(
        event,
        runie_core::types::AgentEvent::Reset
            | runie_core::types::AgentEvent::ThemeChanged { .. }
            | runie_core::types::AgentEvent::ModelChanged { .. }
            | runie_core::types::AgentEvent::ToolDisplayModeChanged { .. }
            | runie_core::types::AgentEvent::ToolExecutionStart { .. }
    )
}
