//! Fullscreen framed subagent transcript detail view.
//!
//! Opened from the feed when the user presses Enter on a subagent lifecycle
//! row. Renders the worker's full output in a bordered overlay with a title
//! bar, scrollable body, and footer hint.

use ratatui::{
    layout::{Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::Paragraph,
    Frame,
};
use runie_core::model::{PatternWorkerRow, PatternWorkerStatus};
use runie_core::Snapshot;

/// Animation cycle length (frames) for the running-worker brightness pulse.
const PULSE_CYCLE: u32 = 24;
/// Brightness pulse amplitude: ±15% around the base subagent.running color.
const PULSE_AMPLITUDE: f32 = 0.15;

/// Render the subagent detail overlay fullscreen over the feed area.
pub fn render_subagent_detail(f: &mut Frame, snap: &Snapshot, area: Rect) {
    let Some(detail) = snap.subagent_detail.as_ref() else {
        return;
    };
    let Some(worker) = snap
        .pattern_workers
        .iter()
        .find(|w| w.id == detail.worker_id)
    else {
        return;
    };

    let popup_title = " Subagent ";
    let popup_area = crate::popups::palette_popup_rect(area);
    crate::popups::clear_panel_bg(f, popup_area);
    let block = crate::theme::block_popup(popup_title);
    let inner = block.inner(popup_area);
    f.render_widget(Paragraph::new("").block(block), popup_area);

    if inner.height < 3 {
        return;
    }

    // Same body/footer split as original: body takes all but the last row.
    let body_height = inner.height - 1;
    let body_area = Rect { x: inner.x, y: inner.y, width: inner.width, height: body_height };
    let footer_area =
        Rect { x: inner.x, y: inner.y + body_height, width: inner.width, height: 1 };

    render_body(f, worker, snap.animation_frame, detail.scroll, body_area);
    render_footer(f, footer_area);
}

fn build_title(worker: &PatternWorkerRow, frame: u32) -> String {
    let icon = match worker.status {
        PatternWorkerStatus::Completed => crate::theme::GLYPH_CHECK.to_string(),
        PatternWorkerStatus::Failed | PatternWorkerStatus::Cancelled => {
            crate::theme::GLYPH_X.to_string()
        }
        PatternWorkerStatus::Running => {
            let symbols = runie_core::labels::BRAILLE_TEN;
            symbols[frame as usize % symbols.len()].to_string()
        }
    };
    format!(
        "{} General {} {} {} [✗]",
        icon,
        worker.description,
        worker.model,
        format_duration(worker)
    )
}

fn build_status_icon(worker: &PatternWorkerRow, frame: u32) -> Span<'static> {
    match worker.status {
        PatternWorkerStatus::Completed => Span::styled(
            crate::theme::GLYPH_CHECK,
            Style::default().fg(crate::theme::color_subagent_completed()),
        ),
        PatternWorkerStatus::Failed | PatternWorkerStatus::Cancelled => Span::styled(
            crate::theme::GLYPH_X,
            Style::default().fg(crate::theme::color_subagent_failed()),
        ),
        PatternWorkerStatus::Running => {
            let symbols = runie_core::labels::BRAILLE_TEN;
            let icon = symbols[frame as usize % symbols.len()].to_string();
            let base = crate::theme::color_subagent_running();
            Span::styled(icon, Style::default().fg(pulse_color(base, frame)))
        }
    }
}

fn format_duration(worker: &PatternWorkerRow) -> String {
    worker
        .duration_ms
        .map(|ms| runie_core::labels::format_elapsed_secs(ms as f64 / 1000.0))
        .unwrap_or_else(|| {
            let elapsed = worker.started.elapsed().as_secs_f64();
            runie_core::labels::format_elapsed_secs(elapsed)
        })
}

fn pulse_color(base: Color, frame: u32) -> Color {
    let Color::Rgb(r, g, b) = base else {
        return base;
    };
    let t = frame % PULSE_CYCLE;
    let phase = (t as f32 / PULSE_CYCLE as f32) * std::f32::consts::TAU;
    let factor = 1.0 + PULSE_AMPLITUDE * phase.sin();
    let adjust = |c: u8| (c as f32 * factor).clamp(0.0, 255.0) as u8;
    Color::Rgb(adjust(r), adjust(g), adjust(b))
}

fn render_body(
    f: &mut Frame,
    worker: &PatternWorkerRow,
    frame: u32,
    scroll: usize,
    area: Rect,
) {
    // Build header row: status icon + General + description + model + duration + [✗]
    let header_spans = vec![
        build_status_icon(worker, frame),
        Span::styled(" General ", Style::default().fg(pulse_color_for_status(worker.status, frame))),
        Span::styled(
            worker.description.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(worker.model.clone(), crate::theme::style_hint()),
        Span::raw("  "),
        Span::styled(format_duration(worker), crate::theme::style_hint()),
        Span::raw(" "),
        Span::styled("[✗]", crate::theme::style_hint()),
    ];
    let header = Line::from(header_spans);

    let content_width = area.width.saturating_sub(2).max(1);
    let spans =
        crate::markdown_render::apply_color_to_inlines(&worker.output, crate::theme::color_agent_text());
    let rows = crate::message::wrap_styled_spans(&spans, content_width, content_width);

    let mut lines: Vec<Line<'static>> = rows
        .into_iter()
        .map(|row| Line::from(crate::markdown_render::md_to_spans(&row)))
        .collect();
    if lines.is_empty() {
        lines.push(Line::from(""));
    }

    // Prepend header to content
    lines.insert(0, header);
    lines.insert(1, Line::from("")); // blank line after header

    let max_scroll = lines.len().saturating_sub(area.height as usize);
    let offset = scroll.min(max_scroll);
    let visible: Vec<Line<'static>> = lines
        .into_iter()
        .skip(offset)
        .take(area.height as usize)
        .collect();

    let margin = Margin::new(1, 0);
    let padded = area.inner(margin);
    f.render_widget(Paragraph::new(Text::from(visible)), padded);
}

/// Get the color for status styling (used for "General" label in header).
fn pulse_color_for_status(status: PatternWorkerStatus, frame: u32) -> Color {
    match status {
        PatternWorkerStatus::Completed => crate::theme::color_subagent_completed(),
        PatternWorkerStatus::Failed | PatternWorkerStatus::Cancelled => {
            crate::theme::color_subagent_failed()
        }
        PatternWorkerStatus::Running => pulse_color(crate::theme::color_subagent_running(), frame),
    }
}

fn render_footer(f: &mut Frame, area: Rect) {
    let spans = vec![
        Span::styled("q/Esc", crate::theme::style_hint_key()),
        Span::styled(":back", crate::theme::style_hint()),
        Span::styled(" │ ", crate::theme::style_hint()),
        Span::styled("Enter", crate::theme::style_hint_key()),
        Span::styled(":open", crate::theme::style_hint()),
    ];
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::caps::{MouseCapability, TermCaps};
    use crate::theme::set_current_theme_with_caps;
    use ratatui::{backend::TestBackend, Terminal};
    use runie_core::model::PatternWorkerStatus;
    use runie_core::model::SubagentDetail;
    use std::sync::Arc;

    fn truecolor_caps() -> TermCaps {
        TermCaps { truecolor: true, mouse: MouseCapability::Sgr, ..Default::default() }
    }

    fn worker(status: PatternWorkerStatus, output: &str) -> PatternWorkerRow {
        PatternWorkerRow {
            id: "w.1".into(),
            description: "find callers".into(),
            model: "grok-3".into(),
            status,
            started: std::time::Instant::now(),
            duration_ms: Some(2500),
            activity: "Waiting for response…".into(),
            output: output.into(),
        }
    }

    fn snapshot_with_worker(worker: PatternWorkerRow, detail: Option<SubagentDetail>) -> Snapshot {
        Snapshot { pattern_workers: Arc::new([worker]), subagent_detail: detail, ..Default::default() }
    }

    fn buffer_string(terminal: &Terminal<TestBackend>) -> String {
        let buf = terminal.backend().buffer();
        (0..buf.area().height)
            .map(|y| {
                (0..buf.area().width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn completed_worker_title_shows_check_description_model_duration() {
        let _lock = crate::theme::test_lock();
        set_current_theme_with_caps("runie", truecolor_caps());

        let snap = snapshot_with_worker(
            worker(PatternWorkerStatus::Completed, "first line\nsecond line"),
            Some(SubagentDetail { worker_id: "w.1".into(), scroll: 0 }),
        );
        let backend = TestBackend::new(80, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| render_subagent_detail(f, &snap, f.area()))
            .unwrap();

        let text = buffer_string(&terminal);
        assert!(
            text.contains(crate::theme::GLYPH_CHECK),
            "title must contain checkmark: {text}"
        );
        assert!(
            text.contains("General"),
            "title must show agent type General: {text}"
        );
        assert!(
            text.contains("find callers"),
            "title must contain description: {text}"
        );
        assert!(text.contains("grok-3"), "title must contain model: {text}");
        assert!(
            text.contains("2.5s") || text.contains("2.5 s"),
            "title must contain duration: {text}"
        );
        assert!(
            text.contains("[✗]"),
            "title must contain close button: {text}"
        );
        assert!(
            text.contains("first line"),
            "body must render output text: {text}"
        );
        assert!(
            text.contains("second line"),
            "body must render output text: {text}"
        );
    }

    #[test]
    fn failed_worker_title_shows_x_and_red_styling() {
        let _lock = crate::theme::test_lock();
        set_current_theme_with_caps("runie", truecolor_caps());

        let snap = snapshot_with_worker(
            worker(PatternWorkerStatus::Failed, "something went wrong"),
            Some(SubagentDetail { worker_id: "w.1".into(), scroll: 0 }),
        );
        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| render_subagent_detail(f, &snap, f.area()))
            .unwrap();

        let buf = terminal.backend().buffer();
        let text = buffer_string(&terminal);
        assert!(
            text.contains(crate::theme::GLYPH_X),
            "title must contain X icon: {text}"
        );

        // Find the X icon cell and assert its foreground is red-ish.
        let x_cell = buf
            .content()
            .iter()
            .find(|c| c.symbol() == crate::theme::GLYPH_X)
            .expect("X icon cell must exist");
        assert!(
            matches!(x_cell.fg, ratatui::style::Color::Rgb(r, _, _) if r > 200),
            "failed icon should be red-ish, got {:?}",
            x_cell.fg
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn running_worker_title_shows_braille_spinner_and_pulsed_purple() {
        let _lock = crate::theme::test_lock();
        set_current_theme_with_caps("runie", truecolor_caps());

        let mut worker = worker(PatternWorkerStatus::Running, "still working");
        worker.duration_ms = Some(0);
        let mut snap = snapshot_with_worker(
            worker,
            Some(SubagentDetail { worker_id: "w.1".into(), scroll: 0 }),
        );
        snap.animation_frame = 0;

        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| render_subagent_detail(f, &snap, f.area()))
            .unwrap();

        let buf = terminal.backend().buffer();
        let text = buffer_string(&terminal);

        let frame0_symbol = runie_core::labels::BRAILLE_TEN[0];
        assert!(
            text.contains(frame0_symbol),
            "frame 0 must show braille {frame0_symbol}: {text}"
        );

        let braille_cell = buf
            .content()
            .iter()
            .find(|c| c.symbol() == frame0_symbol.to_string())
            .expect("braille icon cell must exist");
        assert!(
            matches!(braille_cell.fg, ratatui::style::Color::Rgb(r, _g, b) if r > 160 && b > 220),
            "running icon should be purple-ish, got {:?}",
            braille_cell.fg
        );
        let frame0_color = braille_cell.fg;

        // A later frame should use the next braille symbol in the Grok sequence.
        snap.animation_frame = 1;
        terminal
            .draw(|f| render_subagent_detail(f, &snap, f.area()))
            .unwrap();
        let text1 = buffer_string(&terminal);
        let frame1_symbol = runie_core::labels::BRAILLE_TEN[1];
        assert!(
            text1.contains(frame1_symbol),
            "frame 1 must show braille {frame1_symbol}: {text1}"
        );

        // Three quarters through the pulse cycle the icon should be at its
        // dimmest point (sin = -1, factor = 1 - amplitude), confirming the
        // deterministic brightness animation.
        let dim_frame = PULSE_CYCLE * 3 / 4;
        snap.animation_frame = dim_frame;
        terminal
            .draw(|f| render_subagent_detail(f, &snap, f.area()))
            .unwrap();
        let buf_dim = terminal.backend().buffer();
        let dim_symbol = runie_core::labels::BRAILLE_TEN[dim_frame as usize % runie_core::labels::BRAILLE_TEN.len()];
        let dim_cell = buf_dim
            .content()
            .iter()
            .find(|c| c.symbol() == dim_symbol.to_string())
            .expect("dim-frame braille cell must exist");
        let Color::Rgb(r0, g0, b0) = frame0_color else {
            panic!("frame 0 fg should be Rgb");
        };
        let Color::Rgb(r1, g1, b1) = dim_cell.fg else {
            panic!("dim-frame fg should be Rgb");
        };
        assert!(
            r1 < r0 || g1 < g0 || b1 < b0,
            "pulse should dim the running icon at frame {dim_frame}, got {:?} vs {:?}",
            dim_cell.fg,
            frame0_color
        );
    }

    #[test]
    fn footer_renders_hint_bar() {
        let _lock = crate::theme::test_lock();
        set_current_theme_with_caps("runie", truecolor_caps());

        let snap = snapshot_with_worker(
            worker(PatternWorkerStatus::Completed, "body"),
            Some(SubagentDetail { worker_id: "w.1".into(), scroll: 0 }),
        );
        let backend = TestBackend::new(80, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| render_subagent_detail(f, &snap, f.area()))
            .unwrap();

        let text = buffer_string(&terminal);
        assert!(
            text.contains("q/Esc:back"),
            "footer must contain back hint: {text}"
        );
        assert!(
            text.contains("Enter:open"),
            "footer must contain open hint: {text}"
        );
    }
}
