//! Message feed rendering and vim-nav selection highlight.

use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Paragraph, Wrap},
    Frame,
};
use runie_core::ui::elements::PostKind;
use runie_core::{Element, Snapshot};

use crate::ui::render_lines::to_lines_and_count;

pub(crate) fn render_messages(f: &mut Frame, snap: &Snapshot, area: Rect) {
    if snap.elements.is_empty() {
        render_empty_state(f, area);
    } else {
        render_message_content(f, snap, area);
    }
}

fn render_empty_state(f: &mut Frame, area: Rect) {
    let hint = Line::from("Type a message to start...").style(crate::theme::style_empty_state());
    f.render_widget(Paragraph::new(hint), area);
}

fn render_message_content(f: &mut Frame, snap: &Snapshot, area: Rect) {
    let height = area.height as usize;
    if height == 0 || snap.total_lines == 0 {
        return;
    }

    let content_width = area.width;
    let (lines, row_to_element) = build_lines_with_mapping(snap, content_width);
    let offset = compute_scroll_offset(snap, &row_to_element, area.height as usize);

    render_paragraph(f, area, lines, offset);

    if snap.vim_nav_mode {
        highlight_selected_post(f, snap, area, &row_to_element, offset);
    }

    render_scrollbar_if_needed(f, area, row_to_element.len(), offset, height);
}

fn compute_scroll_offset(snap: &Snapshot, row_to_element: &[usize], visible_height: usize) -> u16 {
    let mut offset = snap.scroll_offset(visible_height);
    if snap.vim_nav_mode {
        if let Some(selected_post) = snap.selected_post {
            if let Some(post_offset) =
                post_actual_offset(snap, row_to_element, visible_height, selected_post)
            {
                offset = post_offset;
            }
        }
    }
    offset
}

fn render_paragraph(f: &mut Frame, area: Rect, lines: Vec<Line<'_>>, offset: u16) {
    f.render_widget(
        Paragraph::new(lines)
            .scroll((offset, 0))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn highlight_selected_post(
    f: &mut Frame,
    snap: &Snapshot,
    area: Rect,
    row_to_element: &[usize],
    offset: u16,
) {
    if let Some(selected_post) = snap.selected_post {
        draw_post_background(f, snap, area, row_to_element, offset, selected_post);
        draw_post_left_line(f, snap, area, row_to_element, offset, selected_post);
    }
}

fn render_scrollbar_if_needed(f: &mut Frame, area: Rect, total: usize, offset: u16, height: usize) {
    if total > height {
        let full_w = f.area().width;
        let scrollbar_area = Rect {
            x: (area.x + area.width).min(full_w.saturating_sub(1)),
            y: area.y,
            width: 1,
            height: area.height,
        };
        super::render_scrollbar(f, scrollbar_area, total, offset, height);
    }
}

/// Flatten the feed into wrapped terminal lines and record which element
/// each visible row belongs to. This mapping is what the vim-nav selection
/// highlight uses, so the highlight height matches the *rendered* height of
/// a post even when message text wraps.
pub(crate) fn build_lines_with_mapping(
    snap: &Snapshot,
    content_width: u16,
) -> (Vec<Line<'_>>, Vec<usize>) {
    let mut lines = Vec::with_capacity(snap.total_lines);
    let mut mapping = Vec::with_capacity(snap.total_lines);
    for (idx, elem) in snap.elements.iter().enumerate() {
        let (elem_lines, wrapped_rows) = to_lines_and_count(elem, content_width);
        mapping.extend(std::iter::repeat_n(idx, wrapped_rows));
        lines.extend(elem_lines);
    }

    // Append streaming tail when a turn is active
    if snap.turn_active && !snap.streaming_tail.is_empty() {
        let streaming_lines = wrap_text_to_lines(&snap.streaming_tail, content_width);
        let last_idx = snap.elements.len().saturating_sub(1);
        for line in streaming_lines {
            mapping.push(last_idx);
            lines.push(line);
        }
    }

    (lines, mapping)
}

/// Wrap text to lines respecting content width.
fn wrap_text_to_lines(text: &str, width: u16) -> Vec<Line<'_>> {
    use ratatui::text::Line;
    use std::borrow::Cow;

    let mut result = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            result.push(Line::from(""));
            continue;
        }

        // Simple word wrap at width boundary
        let chars_per_line = width as usize;
        let chars: Vec<char> = line.chars().collect();

        if chars.len() <= chars_per_line {
            result.push(Line::from(Cow::Borrowed(line)));
            continue;
        }

        // Break into chunks of chars_per_line
        for chunk in chars.chunks(chars_per_line) {
            let wrapped: String = chunk.iter().collect();
            result.push(Line::from(Cow::Owned(wrapped)));
        }
    }

    result
}

/// Fill the selected post's area with a subtle accent-colored background
/// at 10% opacity. The highlight spans exactly the same rows as the left
/// selection line (content + adjacent spacers/margins) so the selection
/// is readable and visually consistent.
fn draw_post_background(
    f: &mut Frame,
    snap: &Snapshot,
    area: Rect,
    row_to_element: &[usize],
    offset: u16,
    selected_post: usize,
) {
    let Some((visible_start, visible_end)) = visible_row_range(row_to_element, offset, area.height)
    else {
        return;
    };
    let Some((start, end)) = selected_post_row_range(snap, row_to_element, selected_post) else {
        return;
    };

    let bg = crate::theme::color_accent_bg();
    let first_visible = start.max(visible_start);
    let last_visible = end.min(visible_end);

    for row in first_visible..last_visible {
        let y = area.y + (row - visible_start) as u16;
        if y >= area.y + area.height {
            break;
        }
        // Fill the whole available terminal line, including outer margins,
        // so the selection highlight spans the full width with no gaps.
        let full_width = f.area().width;
        for x in 0..full_width {
            let cell = &mut f.buffer_mut()[(x, y)];
            let _ = cell.set_bg(bg);
        }
    }
}

/// Draw a thin accent vertical line in the leftmost terminal column for
/// every visible row of the selected post. The line spans exactly the same
/// rows as the accent background, giving the selection a clean left edge.
fn draw_post_left_line(
    f: &mut Frame,
    snap: &Snapshot,
    area: Rect,
    row_to_element: &[usize],
    offset: u16,
    selected_post: usize,
) {
    let Some((visible_start, visible_end)) = visible_row_range(row_to_element, offset, area.height)
    else {
        return;
    };
    let Some((start, end)) = selected_post_row_range(snap, row_to_element, selected_post) else {
        return;
    };

    let first_visible = start.max(visible_start);
    let last_visible = end.min(visible_end);
    if first_visible >= last_visible {
        return;
    }

    let accent = crate::theme::color_accent();

    for row in first_visible..last_visible {
        let y = area.y + (row - visible_start) as u16;
        if y >= area.y + area.height {
            break;
        }
        // Draw the thin selection line one column left of the feed area so
        // it hugs the terminal edge and does not steal horizontal space
        // from message content.
        let x = area.x.saturating_sub(1);
        let cell = &mut f.buffer_mut()[(x, y)];
        // Use a thin left-side block so the visual line sits on the left
        // edge of the cell without looking heavy.
        let _ = cell.set_char('▎');
        let _ = cell.set_fg(accent);
    }
}

fn visible_row_range(
    row_to_element: &[usize],
    offset: u16,
    area_height: u16,
) -> Option<(usize, usize)> {
    let visible_start = offset as usize;
    let visible_end = (offset as usize + area_height as usize).min(row_to_element.len());
    if visible_start >= visible_end {
        return None;
    }
    Some((visible_start, visible_end))
}

/// Compute the inclusive start and exclusive end rows of the selected
/// post's highlight area. This includes the post's content rows plus the
/// adjacent spacer/margin rows that the highlight extends into, so the
/// returned range is exactly the height of the selection.
fn selected_post_row_range(
    snap: &Snapshot,
    row_to_element: &[usize],
    selected_post: usize,
) -> Option<(usize, usize)> {
    let post = snap.posts.get(selected_post)?;
    let (elem_start_rows, elem_line_counts) = element_row_map(row_to_element);
    let (start, end) = post_content_range(snap, post, &elem_start_rows, &elem_line_counts)?;
    Some(extend_with_spacers(
        snap,
        row_to_element,
        start,
        end,
        post.kind,
    ))
}

fn post_content_range(
    snap: &Snapshot,
    post: &runie_core::ui::elements::Post,
    elem_start_rows: &[usize],
    elem_line_counts: &[usize],
) -> Option<(usize, usize)> {
    let mut bracket_start: Option<usize> = None;
    let mut bracket_end: Option<usize> = None;
    for elem_idx in post.start..post.end {
        let elem = snap.elements.get(elem_idx)?;
        if matches!(elem, Element::Spacer { .. }) {
            continue;
        }
        let start = elem_start_rows[elem_idx];
        let end = start + elem_line_counts[elem_idx];
        bracket_start = Some(bracket_start.map_or(start, |s| s.min(start)));
        bracket_end = Some(bracket_end.map_or(end, |e| e.max(end)));
    }
    Some((bracket_start?, bracket_end?))
}

fn extend_with_spacers(
    snap: &Snapshot,
    row_to_element: &[usize],
    start: usize,
    end: usize,
    kind: PostKind,
) -> (usize, usize) {
    if kind == PostKind::UserInput {
        return (start, end);
    }
    let new_start = if start > 0 && is_spacer_at_row(snap, row_to_element, start - 1) {
        start - 1
    } else {
        start
    };
    let new_end = if end < row_to_element.len() && is_spacer_at_row(snap, row_to_element, end) {
        end + 1
    } else {
        end
    };
    (new_start, new_end)
}

fn is_spacer_at_row(snap: &Snapshot, row_to_element: &[usize], row: usize) -> bool {
    let elem_idx = row_to_element.get(row).copied().unwrap_or(usize::MAX);
    matches!(snap.elements.get(elem_idx), Some(Element::Spacer { .. }))
}

/// Compute the actual wrapped-row offset that places the start of the
/// selected post at the top of the viewport when possible, or keeps it
/// visible near the bottom when the post is lower in the feed.
fn post_actual_offset(
    snap: &Snapshot,
    row_to_element: &[usize],
    visible_height: usize,
    selected_post: usize,
) -> Option<u16> {
    let post = snap.posts.get(selected_post)?;
    let (starts, _) = element_row_map(row_to_element);
    let first_content = (post.start..post.end)
        .find(|&i| !matches!(snap.elements.get(i), Some(Element::Spacer { .. })))?;
    // Scroll so the full bracket is visible. Non-user posts extend into
    // the spacer above them (leading spacer for the first post, trailing
    // spacer of the previous post otherwise). User messages already have
    // internal margins, so their bracket starts at the content element.
    let target_top = if post.kind == PostKind::UserInput {
        starts[first_content]
    } else {
        starts[first_content].saturating_sub(1)
    };
    let max_offset = row_to_element.len().saturating_sub(visible_height);
    Some(target_top.min(max_offset).min(u16::MAX as usize) as u16)
}

/// From a flat `row -> element` mapping, derive the start row and line
/// count for each element index.
fn element_row_map(row_to_element: &[usize]) -> (Vec<usize>, Vec<usize>) {
    let elem_count = row_to_element.iter().copied().max().map_or(0, |m| m + 1);
    let mut starts = vec![0usize; elem_count];
    let mut counts = vec![0usize; elem_count];
    for (row, &elem_idx) in row_to_element.iter().enumerate() {
        counts[elem_idx] += 1;
        if row == 0 || elem_idx != row_to_element[row - 1] {
            starts[elem_idx] = row;
        }
    }
    (starts, counts)
}

pub(crate) fn estimate_element_tokens(elem: &Element) -> usize {
    use runie_core::Element::*;
    match elem {
        UserMessage { content, .. }
        | AgentMessage { content, .. }
        | ThoughtMarker { content, .. } => content.len() / 4,
        Thinking { .. } | ThoughtSummary { .. } | ToolSummary { .. } | TurnComplete { .. } => 10,
        ToolRunning { .. } => 10,
        ToolDone { output, .. } => output.len() / 4 + 10,
        Spacer { .. } => 0,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::ui::render_lines::{element_line_count, to_lines_internal};
    use ratatui::{backend::TestBackend, Terminal};
    use runie_core::ui::elements::Element;

    fn assert_count_matches(element: Element, width: u16) {
        let count = element_line_count(&element, width);
        let rendered = to_lines_internal(&element, width);
        let wrapped_rows = Paragraph::new(rendered.as_slice())
            .wrap(Wrap { trim: false })
            .line_count(width);
        let core_count = runie_core::layout::element_line_count(&element, width);
        assert_eq!(
            count,
            wrapped_rows,
            "element_line_count mismatch for {element:?} at width {width}: expected {wrapped_rows}, got {count}",
        );
        assert_eq!(
            core_count,
            wrapped_rows,
            "core layout count mismatch for {element:?} at width {width}: expected {wrapped_rows}, got {core_count}",
        );
    }

    #[test]
    fn element_line_count_matches_rendered_lines_user_message() {
        assert_count_matches(Element::user("hello world").at(0.0), 80);
        assert_count_matches(Element::user("a".repeat(200)).at(0.0), 40);
        assert_count_matches(Element::user("line1\nline2\nline3").at(0.0), 30);
    }

    #[test]
    fn element_line_count_matches_rendered_lines_agent_message() {
        assert_count_matches(Element::agent("hello world").at(0.0), 80);
        assert_count_matches(Element::agent("a".repeat(200)).at(0.0), 40);
        assert_count_matches(Element::agent("line1\nline2\nline3").at(0.0), 30);
    }

    #[test]
    fn agent_message_with_code_block_line_count_matches_rendered_rows() {
        assert_count_matches(
            Element::agent("intro\n```rust\nlet x = 1;\n```\noutro").at(0.0),
            80,
        );
        assert_count_matches(
            Element::agent("```python\nprint('hello')\nprint('world')\n```").at(0.0),
            40,
        );
    }

    #[test]
    fn agent_message_with_list_line_count_matches_rendered_rows() {
        assert_count_matches(Element::agent("items:\n- one\n- two").at(0.0), 80);
        assert_count_matches(Element::agent("1. first\n2. second").at(0.0), 40);
    }

    #[test]
    fn wide_text_does_not_overflow_viewport() {
        // Each CJK character is two display cells; ensure core and renderer agree.
        assert_count_matches(Element::agent("日本語テキスト").at(0.0), 20);
        assert_count_matches(Element::user("日本語テキスト").at(0.0), 20);
    }

    #[test]
    fn element_line_count_matches_rendered_lines_thought_marker() {
        assert_count_matches(Element::thought("hello world").at(0.0), 80);
        assert_count_matches(Element::thought("a".repeat(200)).at(0.0), 40);
        assert_count_matches(Element::thought("line1\nline2\nline3").at(0.0), 30);
    }

    #[test]
    fn element_line_count_matches_rendered_lines_simple_variants() {
        let started = std::time::Instant::now();
        assert_count_matches(Element::spacer().at(0.0), 80);
        assert_count_matches(Element::thinking(started).at(0.0), 80);
        assert_count_matches(Element::thought_summary("summary", 1.0).at(0.0), 80);
        assert_count_matches(Element::tool_running("ls", ".", started).at(0.0), 80);
        assert_count_matches(
            Element::tool_done("ls", ".", 0.5, "out1\nout2\nout3", None, false).at(0.0),
            80,
        );
        assert_count_matches(Element::tool_summary("ls", 0.5).at(0.0), 80);
        assert_count_matches(Element::turn_complete(1.0).at(0.0), 80);
    }

    #[test]
    fn scrollbar_thumb_matches_markdown_message_height() {
        let width = 40u16;
        let height = 4u16;
        let element =
            Element::agent("items:\n- one\n- two\n- three\n- four\n- five\n- six").at(0.0);
        let rendered = to_lines_internal(&element, width).len();
        assert!(
            rendered > height as usize,
            "message should be taller than viewport"
        );

        let snap = Snapshot {
            elements: Arc::new([element]),
            line_counts: Arc::new([rendered]),
            total_lines: rendered,
            last_visible_height: height,
            content_width: width,
            ..Default::default()
        };

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| render_messages(f, &snap, f.area()))
            .unwrap();

        let thumb = crate::theme::SCROLLBAR_THUMB.chars().next().unwrap();
        let buffer = terminal.backend().buffer();
        let has_thumb = buffer
            .content()
            .iter()
            .any(|cell| cell.symbol() == thumb.to_string());
        assert!(
            has_thumb,
            "scrollbar thumb should be visible for tall message"
        );
    }
}
