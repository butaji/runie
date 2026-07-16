//! Per-element line rendering — pure functions from Element + width to lines.

use ratatui::text::Line;
use runie_core::Element;

use crate::message as msg;

/// Render an element and return the number of terminal rows Ratatui will use.
pub fn element_line_count(elem: &Element, content_width: u16) -> usize {
    to_lines_and_count(elem, 0, content_width).1
}

/// Render an element to terminal lines.
///
/// Kept as a public entrypoint for future use (e.g. direct buffer rendering
/// without going through the message rendering pipeline). Currently unused but
/// exercised by integration tests.
#[allow(dead_code, reason = "kept for future direct rendering use")]
pub fn to_lines_internal(elem: &Element, content_width: u16) -> Vec<Line<'static>> {
    to_lines_and_count(elem, 0, content_width).0
}

/// Render an element and return both the lines and the number of terminal
/// rows those lines occupy. The lines are already pre-wrapped during rendering,
/// so the count is simply the number of lines returned.
pub fn to_lines_and_count(
    elem: &Element,
    animation_frame: u32,
    content_width: u16,
) -> (Vec<Line<'static>>, usize) {
    let lines = render_element(elem, animation_frame, content_width);
    let count = lines.len();
    (lines, count)
}

fn render_element(elem: &Element, animation_frame: u32, content_width: u16) -> Vec<Line<'static>> {
    use runie_core::Element::*;
    match elem {
        Spacer { .. } => vec![Line::from("")],
        UserMessage { content, timestamp } => {
            msg::render_user_message(content, *timestamp, content_width)
        }
        AgentMessage {
            content, timestamp, ..
        } => msg::render_agent_message(content, *timestamp, content_width),
        Thinking { started, .. } => msg::render_thinking(*started),
        ThoughtSummary {
            content,
            duration_secs,
            ..
        } => msg::render_thought_summary(content, *duration_secs),
        ThoughtMarker { content, .. } => msg::render_thought_marker(content, content_width),
        ContextGroup {
            tools, collapsed, ..
        } => msg::render_context_group(tools, *collapsed),
        SubagentRow { .. } => msg::render_subagent_row(elem, animation_frame),
        _ => render_tool_element(elem, animation_frame, content_width),
    }
}

fn render_tool_element(elem: &Element, animation_frame: u32, _content_width: u16) -> Vec<Line<'static>> {
    use runie_core::Element::*;
    match elem {
        ToolRunning {
            name,
            args,
            started,
            ..
        } => msg::render_tool_running(name, args, started.elapsed().as_secs_f64(), animation_frame),
        ToolDone {
            name,
            args,
            duration_secs,
            output,
            bytes_transferred,
            error,
            ..
        } => msg::render_tool_done(
            name,
            args,
            *duration_secs,
            output,
            *bytes_transferred,
            *error,
        ),
        ToolSummary {
            name,
            duration_secs,
            ..
        } => msg::render_tool_summary(name, "", *duration_secs),
        TurnComplete { duration_secs, .. } => msg::render_turn_complete(*duration_secs),
        _ => vec![Line::from("")],
    }
}
