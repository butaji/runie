//! Grok-style tasks (sub-agent) side pane.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use runie_core::{model::PatternWorkerStatus, Snapshot};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Render the tasks pane into the given area.
pub(crate) fn render_tasks_pane(f: &mut Frame, snap: &Snapshot, area: Rect) {
    if area.width < 6 || area.height < 3 {
        return;
    }

    draw_pane_box(f, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    // Leave a two-cell app-bg gutter before the right border, matching Grok.
    let content_width = inner.width.saturating_sub(2);

    render_header(
        f,
        snap.pattern_workers.len(),
        inner.x,
        inner.y,
        content_width,
    );

    let mut y = inner.y + 1;
    for worker in snap
        .pattern_workers
        .iter()
        .filter(|w| w.status == PatternWorkerStatus::Running || snap.tasks_pane_show_done)
    {
        if y >= inner.y + inner.height {
            break;
        }
        render_row(f, snap, worker, inner.x, y, content_width);
        y += 1;
    }
}

fn draw_pane_box(f: &mut Frame, area: Rect) {
    let border_style = crate::theme::style_border();
    let bg = crate::theme::color_bg();
    let base = border_style.bg(bg);

    let top = "┌".to_string() + &" ".repeat(area.width.saturating_sub(2) as usize) + "✗";
    let bottom = "└".to_string() + &" ".repeat(area.width.saturating_sub(2) as usize) + "┘";

    f.buffer_mut().set_string(area.x, area.y, top, base);
    f.buffer_mut()
        .set_string(area.x, area.y + area.height - 1, bottom, base);

    for row in 1..area.height.saturating_sub(1) {
        let y = area.y + row;
        f.buffer_mut()
            .set_string(area.x, y, "│", border_style.bg(bg));
        f.buffer_mut()
            .set_string(area.x + area.width - 1, y, "│", border_style.bg(bg));
    }
}

fn render_header(f: &mut Frame, count: usize, x: u16, y: u16, content_width: u16) {
    let header_style = crate::theme::style_tasks_pane_header();
    let count_style = header_style
        .remove_modifier(Modifier::BOLD)
        .add_modifier(Modifier::DIM);

    let header_text = format!("▾ Subagents {}", count);
    let header_width = header_text.width();
    let fill_len = (content_width as usize).saturating_sub(INDENT as usize + header_width);

    let mut spans = vec![
        Span::styled(" ".repeat(INDENT as usize), header_style),
        Span::styled("▾ ", header_style),
        Span::styled("Subagents", header_style),
        Span::styled(format!(" {}", count), count_style),
    ];
    if fill_len > 0 {
        spans.push(Span::styled(" ".repeat(fill_len), header_style));
    }

    f.render_widget(
        Paragraph::new(Line::from(spans)),
        Rect {
            x,
            y,
            width: content_width,
            height: 1,
        },
    );
}

const INDENT: u16 = 3;

/// Truncate `text` so its display width is at most `max_width`, appending an
/// ellipsis only when truncation actually occurs.
fn truncate_to_width(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if text.width() <= max_width {
        return text.to_owned();
    }
    let mut out = String::new();
    let mut w = 0usize;
    // Reserve one cell for the ellipsis.
    let limit = max_width.saturating_sub(1);
    for ch in text.chars() {
        let ch_width = ch.width().unwrap_or(0);
        if w + ch_width > limit {
            out.push('…');
            break;
        }
        out.push(ch);
        w += ch_width;
    }
    out
}

struct RowStyle {
    icon: String,
    icon_style: Style,
    type_style: Style,
    desc_style: Style,
    buttons: &'static str,
}

fn render_row(
    f: &mut Frame,
    snap: &Snapshot,
    worker: &runie_core::model::PatternWorkerRow,
    x: u16,
    y: u16,
    content_width: u16,
) {
    use PatternWorkerStatus as S;

    let style = match worker.status {
        S::Running => running_row_style(snap),
        S::Completed => completed_row_style(),
        S::Failed => failed_row_style(),
    };

    let elapsed = match worker.status {
        S::Running => {
            runie_core::labels::format_elapsed_secs(worker.started.elapsed().as_secs_f64())
        }
        _ => worker
            .duration_ms
            .map(|ms| runie_core::labels::format_elapsed_secs(ms as f64 / 1000.0))
            .unwrap_or_else(|| "0.0s".to_string()),
    };

    let fixed_left = format!("{} General ", style.icon);
    let right_text = format!("{} {} {}", worker.model, elapsed, style.buttons);

    let fixed_left_width = fixed_left.width();
    let right_width = right_text.width();
    let avail_desc_width = (content_width as usize)
        .saturating_sub(INDENT as usize + fixed_left_width + right_width + 1);
    let description = truncate_to_width(&worker.description, avail_desc_width);

    let left_text = format!("{}{}", fixed_left, description);
    let left_width = left_text.width();
    let pad_len = (content_width as usize)
        .saturating_sub(INDENT as usize + left_width + right_width)
        .max(1);

    let dim = Style::default().fg(crate::theme::color_dim());

    let spans = vec![
        Span::styled(" ".repeat(INDENT as usize), Style::default()),
        Span::styled(style.icon, style.icon_style),
        Span::styled(" General ", style.type_style),
        Span::styled(description, style.desc_style),
        Span::styled(" ".repeat(pad_len), Style::default()),
        Span::styled(format!("{} ", worker.model), dim),
        Span::styled(format!("{} ", elapsed), dim),
        Span::styled(style.buttons, dim),
    ];

    f.render_widget(
        Paragraph::new(Line::from(spans)),
        Rect {
            x,
            y,
            width: content_width,
            height: 1,
        },
    );
}

fn running_row_style(snap: &Snapshot) -> RowStyle {
    let frames = [':', '\u{2e2c}', '\u{22c5}'];
    let idx = runie_core::labels::BRAILLE_SIX
        .iter()
        .position(|&c| c == snap.spinner_frame)
        .unwrap_or(0);
    let glyph = frames[idx % frames.len()];

    let running = crate::theme::color_subagent_running();
    // Subtle brightness cycle for the running spinner.
    let factor = match idx % frames.len() {
        0 => 1.0,
        1 => 0.82,
        _ => 0.64,
    };
    let spinner_color = crate::theme::darken(running, factor);

    RowStyle {
        icon: glyph.to_string(),
        icon_style: Style::default().fg(spinner_color),
        type_style: Style::default().fg(running),
        desc_style: Style::default().fg(crate::theme::color_fg()),
        buttons: "[↗][✗]",
    }
}

fn completed_row_style() -> RowStyle {
    let completed = crate::theme::color_subagent_completed();
    let type_color = crate::theme::darken(completed, 0.55);
    RowStyle {
        icon: "✓".to_string(),
        icon_style: Style::default().fg(completed),
        type_style: Style::default().fg(type_color),
        desc_style: Style::default().fg(crate::theme::color_dim()),
        buttons: "[↗]",
    }
}

fn failed_row_style() -> RowStyle {
    let failed = crate::theme::color_subagent_failed();
    let type_color = crate::theme::darken(failed, 0.55);
    RowStyle {
        icon: "✗".to_string(),
        icon_style: Style::default().fg(failed),
        type_style: Style::default().fg(type_color),
        desc_style: Style::default().fg(crate::theme::color_dim()),
        buttons: "[↗]",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};
    use runie_core::model::{PatternWorkerRow, PatternWorkerStatus};
    use std::sync::Arc;

    #[test]
    fn tasks_pane_renders_header_rows_and_buttons() {
        let started = std::time::Instant::now();
        let workers = vec![
            PatternWorkerRow {
                id: "w.1".into(),
                description: "find callers".into(),
                model: "echo".into(),
                status: PatternWorkerStatus::Running,
                started,
                duration_ms: None,
                output: String::new(),
            },
            PatternWorkerRow {
                id: "w.2".into(),
                description: "summarize docs".into(),
                model: "gpt-4o".into(),
                status: PatternWorkerStatus::Running,
                started,
                duration_ms: None,
                output: String::new(),
            },
            PatternWorkerRow {
                id: "w.3".into(),
                description: "write tests".into(),
                model: "claude".into(),
                status: PatternWorkerStatus::Completed,
                started,
                duration_ms: Some(2500),
                output: String::new(),
            },
        ];
        let snap = Snapshot {
            tasks_pane_visible: true,
            tasks_pane_show_done: true,
            spinner_frame: runie_core::labels::BRAILLE_SIX[0],
            pattern_workers: Arc::from(workers),
            ..Default::default()
        };

        let backend = TestBackend::new(150, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| render_tasks_pane(f, &snap, f.area()))
            .unwrap();

        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(
            content.contains("Subagents 3"),
            "header should show count: {}",
            content
        );
        assert!(
            content.contains("find callers"),
            "first worker description missing: {}",
            content
        );
        assert!(
            content.contains("summarize docs"),
            "second worker description missing: {}",
            content
        );
        assert!(
            content.contains("write tests"),
            "completed worker description missing: {}",
            content
        );
        assert!(
            content.contains("[↗][✗]"),
            "running row should have open and kill buttons: {}",
            content
        );
        assert!(
            content.contains("[↗]"),
            "completed row should have open button: {}",
            content
        );
        assert!(
            content.contains(':') || content.contains('\u{2e2c}') || content.contains('\u{22c5}'),
            "running spinner frame missing: {}",
            content
        );
    }
}
