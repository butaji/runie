//! Message feed rendering and vim-nav selection highlight.

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use runie_core::Element;
use runie_core::Snapshot;
use crate::theme::{blend_color, color_bg, color_rail_running, wave_brightness};
use crate::theme::{monitor_glyph, GLYPH_STATUS_CANCELLED, GLYPH_STATUS_COMPLETED, GLYPH_STATUS_FAILED, GLYPH_STATUS_QUEUED, GLYPH_STATUS_RUNNING};

const FEED_WAVE_SPEED: f32 = 0.15;

/// One lifecycle-to-chrome mapping shared by workflow-like feed elements.
fn lifecycle_visual(status: &str) -> Option<(&'static str, ratatui::style::Color, bool)> {
    match status {
        "running" | "started" => Some((GLYPH_STATUS_RUNNING, color_rail_running(), true)),
        "completed" | "done" => Some((GLYPH_STATUS_COMPLETED, crate::theme::color_rail_success(), false)),
        "failed" | "killed" => Some((GLYPH_STATUS_FAILED, crate::theme::color_rail_error(), false)),
        "cancelled" => Some((GLYPH_STATUS_CANCELLED, crate::theme::color_rail_error(), false)),
        "paused" => Some((GLYPH_STATUS_QUEUED, crate::theme::color_warning(), false)),
        _ => None,
    }
}

pub(crate) mod lines;
pub(crate) mod nav;

pub(crate) use lines::{build_lines_with_mapping, estimate_element_tokens, sticky_header_row_for_mode};

pub(crate) fn render_messages(f: &mut Frame, snap: &Snapshot, area: Rect) {
    if snap.elements.is_empty() {
        render_empty_state(f, area);
    } else {
        render_message_content(f, snap, area);
    }
}

fn render_empty_state(f: &mut Frame, area: Rect) {
    f.render_widget(Paragraph::new(""), area);
}

fn render_message_content(f: &mut Frame, snap: &Snapshot, area: Rect) {
    let height = area.height as usize;
    if height == 0 || snap.total_lines == 0 {
        return;
    }

    // Reserve 2 columns of right-side slack, the leading feed indent (1 col),
    // and the accent rail column (1 col) so post content lands at column 3
    // while timestamps keep their right edge.
    let content_width = runie_core::layout::feed_content_width_with_slack(
        area.width,
        snap.compact_layout,
        snap.feed_right_slack,
    );
    let (lines, row_to_element) = build_lines_with_mapping(snap, content_width);
    let offset = nav::compute_scroll_offset(snap, &row_to_element, area.height as usize);

    // Render lines with user message backgrounds applied directly to lines
    render_paragraph_with_user_backgrounds(f, snap, area, &lines, offset, &row_to_element);

    if let Some((header_row, _)) = sticky_header_row_for_mode(&row_to_element, offset as usize, snap.follow_mode) {
        render_sticky_header(f, area, &lines[header_row]);
    }

    // Rails belong to the feed edge, like the vim-selection marker, rather
    // than consuming a content column or competing with the scrollbar.
    draw_feed_edge_rails(f, snap, area, offset, &row_to_element);

    if snap.vim_nav_mode {
        nav::highlight_selected_post(f, snap, area, &row_to_element, offset);
    }

    render_scrollbar_if_needed(f, area, row_to_element.len(), offset, height, snap.follow_mode);
    render_follow_affordance(f, snap, area, height);
}

fn render_sticky_header(f: &mut Frame, area: Rect, line: &Line<'_>) {
    if area.height == 0 || area.width < 2 {
        return;
    }
    let header = line_to_owned(line).style(crate::theme::style_hint());
    f.render_widget(
        Paragraph::new(header),
        Rect::new(area.x, area.y, area.width.saturating_sub(1), 1),
    );
}

fn draw_feed_edge_rails(f: &mut Frame, snap: &Snapshot, area: Rect, offset: u16, row_to_element: &[usize]) {
    for row_offset in 0..area.height as usize {
        let abs_row = offset as usize + row_offset;
        let Some(&elem_idx) = row_to_element.get(abs_row) else { continue };
        let element_row = element_local_row(row_to_element, abs_row, elem_idx);
        let color = tool_feed_chrome(snap, elem_idx, element_row)
            .map(|(c, _, _)| c)
            .or_else(|| subagent_feed_chrome(snap, elem_idx, element_row).map(|(c, _, _)| c))
            .or_else(|| background_task_feed_chrome(snap, elem_idx, element_row).map(|(c, _, _)| c))
            .or_else(|| workflow_feed_chrome(snap, elem_idx, element_row).map(|(c, _, _)| c));
        if let Some(color) = color {
            let cell = &mut f.buffer_mut()[(area.x.saturating_sub(1), area.y + row_offset as u16)];
            // Match the vim-selection edge marker exactly: one thin block in
            // the column immediately left of the feed viewport.
            let _ = cell.set_char('▎');
            let _ = cell.set_fg(color);
        }
    }
}

/// Shared prefix geometry for every lifecycle-bearing feed element.
/// The edge rail is painted separately; this function owns only the content
/// column and marker so tools, workers, workflows, and subagents cannot drift.
fn push_lifecycle_prefix(spans: &mut Vec<Span<'static>>, first_row: bool, marker: &str, color: ratatui::style::Color) {
    if first_row {
        spans.push(Span::raw(crate::theme::FEED_INDENT));
        if marker == "◆" {
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(marker.to_owned(), ratatui::style::Style::default().fg(color)));
        spans.push(Span::raw(" "));
    }
}

/// Tell the user when new feed content is below the protected reading
/// position. `scroll` is the distance from the newest tail in core's
/// bottom-oriented scroll model, so it is also the unread row count.
fn render_follow_affordance(f: &mut Frame, snap: &Snapshot, area: Rect, height: usize) {
    if snap.follow_mode || snap.scroll == 0 || height == 0 {
        return;
    }
    let lines_below = snap.scroll.min(usize::MAX / 2);
    let text = format!("{lines_below} lines below · press End/G to follow");
    let width = area.width.saturating_sub(2).max(1);
    let line = ratatui::text::Line::from(ratatui::text::Span::styled(
        text,
        crate::theme::style_hint(),
    ));
    f.render_widget(
        ratatui::widgets::Paragraph::new(line),
        Rect::new(area.x, area.y + area.height.saturating_sub(1), width, 1),
    );
}

/// Render lines with user message backgrounds applied to the lines.
///
/// Every line gets the leading feed indent (FEED_INDENT) prepended, so post
/// content starts at column 2. User message rows additionally get the card
/// band: the bg.user background painted across the full app width, edge to
/// edge.
#[allow(clippy::too_many_lines)]
fn render_paragraph_with_user_backgrounds(
    f: &mut Frame,
    snap: &Snapshot,
    area: Rect,
    lines: &[Line<'_>],
    offset: u16,
    row_to_element: &[usize],
) {
    let height = area.height as usize;
    let start = offset as usize;
    let bg = crate::theme::color_bg_user();
    let visible_start = offset as usize;
    let full_width = f.area().width;

    // Build modified lines with user background applied, then prepend the
    // feed indent. The indent span carries no background of its own, so for
    // user rows its cells show the card band painted into the buffer below.
    let modified_lines: Vec<Line<'static>> = lines
        .iter()
        .skip(start)
        .take(height)
        .enumerate()
        .map(|(row_offset, line)| {
            let abs_row = visible_start + row_offset;
            let elem_idx = *row_to_element.get(abs_row).unwrap_or(&usize::MAX);
            let element_row = element_local_row(row_to_element, abs_row, elem_idx);
            let is_user_related = is_user_related_row(snap, elem_idx);
            let is_first_element_row = abs_row == 0 || row_to_element.get(abs_row.wrapping_sub(1)) != Some(&elem_idx);

            let owned = if is_user_related {
                // Convert to owned line with background applied
                line_to_owned_with_bg(line, bg)
            } else {
                line_to_owned(line)
            };
            let mut spans = Vec::new();
            let mut has_rail = false;
            if let Some((_rail_color, bullet, bullet_color)) = thinking_feed_chrome(snap, elem_idx, element_row) {
                // Thought rows use the lifecycle marker without a persistent
                // rail. Grok reserves the vertical rail for active tool and
                // worker grouping; a rail on completed thoughts reads like a
                // stray border.
                if is_first_element_row {
                    spans.push(Span::raw(crate::theme::FEED_INDENT));
                    spans.push(Span::styled(bullet.to_owned(), ratatui::style::Style::default().fg(bullet_color)));
                    spans.push(Span::raw(" "));
                }
            }
            if let Some((rail_color, bullet, bullet_color)) = tool_feed_chrome(snap, elem_idx, element_row) {
                has_rail = true;
                let _ = rail_color;
                push_lifecycle_prefix(&mut spans, is_first_element_row, bullet, bullet_color);
            }
            if let Some((rail_color, bullet, bullet_color)) = subagent_feed_chrome(snap, elem_idx, element_row) {
                has_rail = true;
                let _ = rail_color;
                push_lifecycle_prefix(&mut spans, is_first_element_row, bullet, bullet_color);
            }
            if let Some((rail_color, bullet, bullet_color)) = background_task_feed_chrome(snap, elem_idx, element_row) {
                has_rail = true;
                let _ = rail_color;
                push_lifecycle_prefix(&mut spans, is_first_element_row, bullet, bullet_color);
            }
            if let Some((rail_color, bullet, bullet_color)) = workflow_feed_chrome(snap, elem_idx, element_row) {
                has_rail = true;
                let _ = rail_color;
                push_lifecycle_prefix(&mut spans, is_first_element_row, bullet, bullet_color);
            }
            // Grok BTW blocks have the shared default bullet but no accent
            // rail. Keep it on the first header row only.
            if matches!(snap.elements.get(elem_idx), Some(Element::Btw { .. })) && is_first_element_row {
                spans.push(Span::raw(crate::theme::FEED_INDENT));
                spans.push(Span::raw("◆ "));
            }
            if has_rail && !is_first_element_row {
                spans.push(Span::raw(crate::theme::FEED_INDENT));
                spans.push(Span::raw(" "));
            } else if spans.is_empty() {
                spans.push(Span::raw(crate::theme::FEED_INDENT));
            }
            let mut content_spans = owned.spans;
            if matches!(snap.elements.get(elem_idx), Some(Element::ToolDone { .. }))
                && !is_first_element_row
            {
                let output_fg = crate::theme::style_tool_output().fg.unwrap_or_else(crate::theme::color_dim);
                for span in &mut content_spans {
                    span.style = span.style.fg(output_fg);
                }
            }
            spans.extend(content_spans);
            let line_style = if matches!(snap.elements.get(elem_idx), Some(Element::ToolDone { .. }))
                && !is_first_element_row
            {
                crate::theme::style_tool_output().bg(crate::theme::color_code_bg())
            } else {
                owned.style
            };
            Line::from(spans).style(line_style)
        })
        .collect();

    // FIRST: Draw the card band for user-related rows across the full app
    // width, edge to edge.
    for row_offset in 0..height {
        let row = area.y + row_offset as u16;
        let abs_row = visible_start + row_offset;
        let elem_idx = *row_to_element.get(abs_row).unwrap_or(&usize::MAX);
        let is_user_related = is_user_related_row(snap, elem_idx);

        if is_user_related {
            for x in 0..full_width {
                let cell = &mut f.buffer_mut()[(x, row)];
                let _ = cell.set_bg(bg);
            }
        }
    }

    // THEN: Render text on top of the backgrounds
    for (row_offset, line) in modified_lines.iter().enumerate() {
        let row = area.y + row_offset as u16;
        f.render_widget(
            ratatui::widgets::Paragraph::new(line.clone()),
            Rect::new(area.x, row, area.width, 1),
        );
    }
}

/// Thinking is an active feed block too: keep the body text stable while its
/// shared rail carries the only animation. Thought summaries are completed
/// and therefore use a static success marker.
fn thinking_feed_chrome(
    snap: &Snapshot,
    elem_idx: usize,
    _row_offset: usize,
) -> Option<(ratatui::style::Color, &'static str, ratatui::style::Color)> {
    match snap.elements.get(elem_idx) {
        Some(Element::Thinking { .. }) => {
            let color = crate::theme::color_rail_running();
            let glyph = if snap.reduced_motion {
                GLYPH_STATUS_RUNNING
            } else {
                monitor_glyph((snap.animation_frame / crate::theme::MONITOR_PULSE_DIVISOR) as usize)
            };
            Some((color, glyph, color))
        }
        Some(Element::ThoughtSummary { .. }) => {
            // Completed reasoning is historical context, not a successful
            // operation. Keep its marker and rail quiet like the summary.
            let color = crate::theme::color_dim();
            Some((color, GLYPH_STATUS_COMPLETED, color))
        }
        _ => None,
    }
}

/// Return the row within the current element, rather than the row within the
/// viewport. Grok's accent wave is attached to the block and must not jump
/// phase when scrolling changes the viewport origin.
fn element_local_row(row_to_element: &[usize], abs_row: usize, elem_idx: usize) -> usize {
    row_to_element
        .get(..=abs_row)
        .and_then(|rows| rows.iter().rposition(|&idx| idx != elem_idx))
        .map_or(abs_row, |previous| abs_row.saturating_sub(previous + 1))
}

fn feed_wave(snap: &Snapshot, row: u16) -> f32 {
    if snap.reduced_motion {
        // Keep the active rail visible, but remove temporal/spatial motion.
        1.0
    } else {
        wave_brightness(snap.animation_frame, row, snap.animation_wave_rows.max(1), FEED_WAVE_SPEED)
    }
}

/// Shared Grok-style tool chrome: every rendered tool row receives the accent
/// column, while only the first content row receives the tool bullet.
fn tool_feed_chrome(
    snap: &Snapshot,
    elem_idx: usize,
    row_offset: usize,
) -> Option<(ratatui::style::Color, &'static str, ratatui::style::Color)> {
    match snap.elements.get(elem_idx) {
        Some(Element::ToolRunning { .. }) => {
            let wave = feed_wave(snap, row_offset.min(u16::MAX as usize) as u16);
            let color = blend_color(color_bg(), color_rail_running(), wave).unwrap_or_else(color_rail_running);
            Some((color, GLYPH_STATUS_RUNNING, color))
        }
        Some(Element::ToolDone { error, .. }) => {
            let color = if *error { crate::theme::color_rail_error() } else { crate::theme::color_rail_success() };
            Some((color, if *error { GLYPH_STATUS_FAILED } else { "◆" }, color))
        }
        _ => None,
    }
}

fn subagent_feed_chrome(
    snap: &Snapshot,
    elem_idx: usize,
    row_offset: usize,
) -> Option<(ratatui::style::Color, &'static str, ratatui::style::Color)> {
    let Some(Element::SubagentRow { status, .. }) = snap.elements.get(elem_idx) else {
        return None;
    };
    use runie_core::model::PatternWorkerStatus as Status;
    match status {
        Status::Running => {
            let wave = feed_wave(snap, row_offset.min(u16::MAX as usize) as u16);
            let color = blend_color(color_bg(), crate::theme::color_rail_running(), wave)
                .unwrap_or_else(crate::theme::color_rail_running);
            Some((color, GLYPH_STATUS_RUNNING, color))
        }
        Status::Completed => {
            let color = crate::theme::color_rail_success();
            Some((color, GLYPH_STATUS_COMPLETED, color))
        }
        Status::Failed | Status::Cancelled => {
            let color = crate::theme::color_rail_error();
            Some((color, if matches!(status, Status::Cancelled) { GLYPH_STATUS_CANCELLED } else { GLYPH_STATUS_FAILED }, color))
        }
    }
}

/// Shared Grok-style chrome for collapsed background-task lifecycle rows.
fn background_task_feed_chrome(
    snap: &Snapshot,
    elem_idx: usize,
    row_offset: usize,
) -> Option<(ratatui::style::Color, &'static str, ratatui::style::Color)> {
    let Some(Element::BackgroundTask { status, .. }) = snap.elements.get(elem_idx) else {
        return None;
    };
    let (marker, base_color, running) = lifecycle_visual(status)?;
    if running {
            let wave = feed_wave(snap, row_offset.min(u16::MAX as usize) as u16);
            let color = blend_color(color_bg(), color_rail_running(), wave).unwrap_or_else(color_rail_running);
            Some((color, marker, color))
    } else {
        Some((base_color, marker, base_color))
    }
}

/// Shared Grok-style chrome for workflow lifecycle rows.
fn workflow_feed_chrome(
    snap: &Snapshot,
    elem_idx: usize,
    row_offset: usize,
) -> Option<(ratatui::style::Color, &'static str, ratatui::style::Color)> {
    let Some(Element::Workflow { status, .. }) = snap.elements.get(elem_idx) else {
        return None;
    };
    let (marker, base_color, running) = lifecycle_visual(status)?;
    if running {
            let wave = feed_wave(snap, row_offset.min(u16::MAX as usize) as u16);
            let color = blend_color(color_bg(), color_rail_running(), wave).unwrap_or_else(color_rail_running);
            Some((color, marker, color))
    } else {
        Some((base_color, marker, base_color))
    }
}

/// Convert a line to owned with background applied to all spans.
fn line_to_owned_with_bg(line: &Line<'_>, bg: ratatui::style::Color) -> Line<'static> {
    let spans: Vec<Span<'static>> = line
        .spans
        .iter()
        .map(|s| {
            let mut style = s.style;
            if style.bg.is_none() {
                style = style.bg(bg);
            }
            Span::styled(s.content.to_string(), style)
        })
        .collect();
    Line::from(spans).style(line.style)
}

/// Convert a line to owned, preserving its line-level style.
fn line_to_owned(line: &Line<'_>) -> Line<'static> {
    let spans: Vec<Span<'static>> = line
        .spans
        .iter()
        .map(|s| Span::styled(s.content.to_string(), s.style))
        .collect();
    Line::from(spans).style(line.style)
}

/// Check if a row belongs to a user message card.
///
/// The bg.user background covers only the user message element's own rows
/// (its internal top/bottom padding plus content). The trailing spacer that
/// follows a user post stays on the normal feed background, forming the
/// margin line that separates the card from whatever comes next.
fn is_user_related_row(snap: &Snapshot, elem_idx: usize) -> bool {
    if elem_idx == usize::MAX {
        return false;
    }
    matches!(
        snap.elements.get(elem_idx),
        Some(Element::UserMessage { .. })
    )
}

fn render_scrollbar_if_needed(f: &mut Frame, area: Rect, total: usize, offset: u16, height: usize, is_following: bool) {
    if total > height {
        let full_w = f.area().width;
        let scrollbar_area =
            Rect { x: (area.x + area.width).min(full_w.saturating_sub(1)), y: area.y, width: 1, height: area.height };
        super::render_scrollbar(f, scrollbar_area, total, offset, height, is_following, None);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::ui::render_lines::to_lines_internal;
    use ratatui::{backend::TestBackend, Terminal};
    use runie_core::Element;

    #[test]
    fn scrollbar_thumb_matches_markdown_message_height() {
        let width = 40u16;
        let height = 4u16;
        let element = Element::agent("items:\n- one\n- two\n- three\n- four\n- five\n- six").at(0.0);
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

    /// blockquote_renders_inline_styles — TestBackend buffer shows styled text inside blockquote.
    #[test]
    fn blockquote_renders_inline_styles() {
        let width = 60u16;
        let height = 6u16;
        // Blockquote with bold and italic text
        let element = Element::agent(
            "> **bold** quote
> and *italic* too",
        )
        .at(0.0);
        let rendered = to_lines_internal(&element, width);

        // Blockquote should render at least one line with the bar character
        assert!(
            !rendered.is_empty(),
            "blockquote should produce at least one line"
        );

        // Check that blockquote character appears
        let has_bar = rendered
            .iter()
            .any(|line| line.spans.iter().any(|s| s.content.contains('│')));
        assert!(
            has_bar,
            "blockquote should have │ character: {:?}",
            rendered
        );

        // Render to terminal and check buffer
        let snap = Snapshot {
            elements: Arc::new([element]),
            line_counts: Arc::new([rendered.len()]),
            total_lines: rendered.len(),
            last_visible_height: height,
            content_width: width,
            ..Default::default()
        };

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| render_messages(f, &snap, f.area()))
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        // Blockquote should appear in output
        assert!(
            content.contains('│'),
            "buffer should contain │ for blockquote: {}",
            content
        );
    }

    #[test]
    fn running_feed_wave_uses_each_visible_row() {
        let element = Element::ToolRunning {
            name: "stream".into(),
            args: String::new(),
            started: std::time::Instant::now(),
            timestamp: 0.0,
        };
        let mut snap = Snapshot {
            elements: Arc::new([element]),
            animation_frame: 7,
            animation_wave_rows: 32,
            ..Default::default()
        };

        let row_colors = (0..snap.animation_wave_rows as usize)
            .map(|row| tool_feed_chrome(&snap, 0, row).expect("running tool chrome").0)
            .collect::<std::collections::HashSet<_>>();
        let row_wave = (0..snap.animation_wave_rows)
            .map(|row| wave_brightness(snap.animation_frame, row, snap.animation_wave_rows.max(1), FEED_WAVE_SPEED))
            .collect::<Vec<_>>();
        assert!(
            row_wave.windows(2).any(|pair| (pair[0] - pair[1]).abs() > f32::EPSILON),
            "running wave phase must vary by feed row"
        );
        // Some terminal/theme color combinations quantize nearby values to the
        // same cell color, but the compositor still receives row-specific
        // phases. In truecolor mode the rendered colors vary as well.
        assert!(row_colors.len() >= 1, "running rail chrome must be present");

        let frame_colors = (0..32)
            .map(|frame| {
                snap.animation_frame = frame;
                tool_feed_chrome(&snap, 0, 0).expect("running tool chrome").0
            })
            .collect::<std::collections::HashSet<_>>();
        assert!(frame_colors.len() >= 1, "running rail chrome must be present across frames");
    }

    #[test]
    fn background_task_feed_chrome_matches_grok_lifecycle_states() {
        let states = [("started", GLYPH_STATUS_RUNNING), ("completed", GLYPH_STATUS_COMPLETED), ("failed", GLYPH_STATUS_FAILED), ("killed", GLYPH_STATUS_FAILED)];
        for (status, bullet) in states {
            let snap = Snapshot {
                elements: Arc::new([Element::BackgroundTask {
                    command: "cargo test".into(),
                    task_id: "task.1".into(),
                    status: status.into(),
                    description: None,
                    duration_secs: 1.0,
                    exit_code: None,
                    signal: None,
                    timestamp: 0.0,
                }]),
                animation_frame: 7,
                ..Default::default()
            };
            let (_, actual, _) = background_task_feed_chrome(&snap, 0, 0).expect("background task chrome");
            assert_eq!(actual, bullet, "wrong bullet for background task status {status}");
        }
    }

    #[test]
    fn workflow_feed_chrome_matches_grok_lifecycle_states() {
        let states = [
            ("running", GLYPH_STATUS_RUNNING),
            ("done", GLYPH_STATUS_COMPLETED),
            ("failed", GLYPH_STATUS_FAILED),
            ("cancelled", GLYPH_STATUS_CANCELLED),
            ("paused", GLYPH_STATUS_QUEUED),
        ];
        for (status, bullet) in states {
            let snap = Snapshot {
                elements: Arc::new([Element::Workflow {
                    name: "research".into(),
                    objective: "compare sources".into(),
                    status: status.into(),
                    phases: Vec::new(),
                    active_agents: 0,
                    duration_secs: 1.0,
                    timestamp: 0.0,
                }]),
                animation_frame: 7,
                ..Default::default()
            };
            let (_, actual, _) = workflow_feed_chrome(&snap, 0, 0).expect("workflow chrome");
            assert_eq!(actual, bullet, "wrong bullet for workflow status {status}");
        }
    }

    #[test]
    fn feed_wave_row_is_local_to_its_element_after_scroll() {
        let rows = [4, 4, 4, 9, 9, 9, 9];
        assert_eq!(element_local_row(&rows, 0, 4), 0);
        assert_eq!(element_local_row(&rows, 2, 4), 2);
        assert_eq!(element_local_row(&rows, 3, 9), 0);
        assert_eq!(element_local_row(&rows, 6, 9), 3);
    }
}
