//! Per-element line rendering — pure functions from Element + width to lines.

use crate::core_ui::Element;
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Wrap};

use crate::message as msg;

/// Render an element and return the number of terminal rows Ratatui will use.
pub fn element_line_count(elem: &Element, content_width: u16) -> usize {
    to_lines_and_count(elem, content_width).1
}

/// Render an element to terminal lines.
#[allow(dead_code)]
pub fn to_lines_internal(elem: &Element, content_width: u16) -> Vec<Line<'static>> {
    to_lines_and_count(elem, content_width).0
}

/// Render an element and return both the lines and the number of terminal
/// rows Ratatui will use after its wrap pass.
pub fn to_lines_and_count(elem: &Element, content_width: u16) -> (Vec<Line<'static>>, usize) {
    let lines = render_element(elem, content_width);
    let count = wrapped_row_count(&lines, content_width);
    (lines, count)
}

fn wrapped_row_count(lines: &[Line<'_>], width: u16) -> usize {
    Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .line_count(width)
}

fn render_element(elem: &Element, content_width: u16) -> Vec<Line<'static>> {
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
        ContextGroup { tools, collapsed, .. } => msg::render_context_group(tools, *collapsed),
        _ => render_tool_element(elem, content_width),
    }
}

fn render_tool_element(elem: &Element, _content_width: u16) -> Vec<Line<'static>> {
    use runie_core::Element::*;
    match elem {
        ToolRunning {
            name,
            args,
            started,
            ..
        } => msg::render_tool_running(name, args, started.elapsed().as_secs_f64()),
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
