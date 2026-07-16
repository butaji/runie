//! Per-element line rendering — pure functions from Element + width to lines.

use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Wrap};
use runie_core::Element;

use crate::message as msg;

/// Render an element and return the number of terminal rows Ratatui will use.
pub fn element_line_count(elem: &Element, content_width: u16) -> usize {
    to_lines_and_count(elem, content_width).1
}

/// Render an element to terminal lines.
///
/// Kept as a public entrypoint for future use (e.g. direct buffer rendering
/// without going through the message rendering pipeline). Currently unused but
/// exercised by integration tests.
#[allow(dead_code, reason = "kept for future direct rendering use")]
pub fn to_lines_internal(elem: &Element, content_width: u16) -> Vec<Line<'static>> {
    to_lines_and_count(elem, content_width).0
}

/// Render an element and return both the lines and the number of terminal
/// rows those lines occupy. The lines are already pre-wrapped during rendering,
/// so the count is simply the number of lines returned.
pub fn to_lines_and_count(elem: &Element, content_width: u16) -> (Vec<Line<'static>>, usize) {
    let lines = render_element(elem, content_width);
    let count = lines.len();
    (lines, count)
}

#[allow(dead_code)]
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
        ContextGroup {
            tools, collapsed, ..
        } => msg::render_context_group(tools, *collapsed),
        SubagentRow { .. } => msg::render_subagent_row(elem),
        AnthropicThinking {
            content,
            signature,
            redacted,
            timestamp,
        } => msg::render_anthropic_thinking(content, signature.clone(), *redacted, *timestamp),
        Image {
            data,
            mime_type,
            width_cells,
            height_cells,
            protocol,
            timestamp,
        } => msg::render_image(data, mime_type, *width_cells, *height_cells, *protocol, *timestamp),
        DataPart {
            data,
            format_string,
            timestamp,
        } => msg::render_data_part(data, format_string.as_deref(), *timestamp),
        MarkdownTable {
            headers,
            rows,
            alignments,
            timestamp,
        } => msg::render_markdown_table(headers, rows, alignments, *timestamp),
        DiffOutput {
            content,
            diff_type,
            timestamp,
        } => msg::render_diff_output(content, *diff_type, *timestamp),
        WebSearchCall {
            query,
            results,
            timestamp,
        } => msg::render_web_search_call(query, results, *timestamp),
        AnsiStyled {
            raw_content,
            plain_text,
            timestamp,
        } => msg::render_ansi_styled(raw_content, plain_text, *timestamp),
        ToolConfirmation {
            request_id,
            name,
            args,
            description,
            timestamp,
        } => msg::render_tool_confirmation(request_id, name, args, description, *timestamp),
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
