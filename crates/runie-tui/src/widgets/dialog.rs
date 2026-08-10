//! Pure renderer for the unified dialog stack.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap};
use runie_tui_model::{DialogFrame, DialogKind};

use crate::appearance;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogWidget {
    frame: DialogFrame,
    rows: Vec<String>,
    theme: runie_core::types::ThemeKind,
}

impl DialogWidget {
    pub fn new(frame: DialogFrame, rows: Vec<String>) -> Self {
        Self {
            frame,
            rows,
            theme: runie_core::types::ThemeKind::GrokNight,
        }
    }

    pub fn with_theme(mut self, theme: runie_core::types::ThemeKind) -> Self {
        self.theme = theme;
        self
    }
}

impl Widget for DialogWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let Some(panel) = centered_panel(area, self.rows.len()) else {
            return;
        };
        // Grok's modal is opaque: background content must never bleed through.
        Clear.render(panel, buf);
        // Paragraph styles do not reliably repaint every blank line on all
        // backends. Paint the panel surface first so the dialog background is
        // continuous behind rows, padding, and the footer.
        buf.set_style(panel, appearance::background_style_for(self.theme));
        Paragraph::new(dialog_lines(
            &self.frame,
            &self.rows,
            self.theme,
            panel.width.saturating_sub(2),
            panel.height.saturating_sub(2),
        ))
        .wrap(Wrap { trim: false })
        .block(dialog_block(&self.frame, self.theme))
        .render(panel, buf);
        // Grok reserves a five-cell bracketed close affordance on the top
        // border. Keep it visual-only; input routing remains actor-owned.
        let close_x = panel.x + panel.width.saturating_sub(7);
        // The close affordance sits on the border; using the muted text token
        // here made that section visibly different from the rest of the rim.
        buf.set_string(close_x, panel.y, " [✗] ", dialog_border_style(self.theme));
    }
}

fn centered_panel(area: Rect, _row_count: usize) -> Option<Rect> {
    const MODAL_MAX_WIDTH: u16 = 80;
    const MODAL_MIN_WIDTH: u16 = 44;
    const MODAL_VERTICAL_MARGIN: u16 = 4;
    let width = ((area.width as f32 * 0.50) as u16)
        .clamp(MODAL_MIN_WIDTH, MODAL_MAX_WIDTH)
        .min(area.width.saturating_sub(4));
    const MODAL_MIN_HEIGHT: u16 = 8;
    let available_height = area
        .height
        .saturating_sub(MODAL_VERTICAL_MARGIN.saturating_mul(2));
    // A dialog should fit its content, while still retaining the reference
    // modal's breathing room and never exceeding the available terminal.
    let content_height = (_row_count as u16).saturating_add(7).max(MODAL_MIN_HEIGHT);
    let height = available_height.min(content_height);
    (width >= 32 && height <= area.height).then_some(Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    })
}

fn dialog_lines(
    frame: &DialogFrame,
    rows: &[String],
    theme: runie_core::types::ThemeKind,
    inner_width: u16,
    inner_height: u16,
) -> Vec<Line<'static>> {
    // ModalSizing for Grok's command palette is h_pad=2, v_pad=1 and a
    // two-row, bottom-aligned footer. Build the same vertical bands here so
    // the dialog remains identical as the terminal grows or shrinks.
    let mut lines = dialog_header(frame, theme, inner_width);
    // Three header rows plus the blank footer spacer and shortcut row are
    // outside the scrolling viewport for every dialog kind. Reserving only
    // four rows made the final selected item in selectors/forms get rendered
    // and then truncated when the pinned footer was appended.
    let reserved = 5;
    append_dialog_rows(
        &mut lines,
        rows,
        frame.selected,
        theme,
        inner_width,
        inner_height.saturating_sub(reserved),
    );
    // Pin shortcuts to the panel's final interior row. Content may be shorter
    // than the modal or gain group separators, so appending directly would
    // either float the footer upward or push it beyond the visible viewport.
    let footer_start = inner_height.saturating_sub(2) as usize;
    lines.truncate(footer_start);
    lines.resize_with(footer_start, || Line::from(""));
    lines.extend(dialog_footer(frame, theme, inner_width));
    lines
}

fn dialog_text_style(theme: runie_core::types::ThemeKind) -> ratatui::style::Style {
    appearance::base_style_for(theme).bg(appearance::background_style_for(theme)
        .bg
        .expect("panel background token"))
}

fn dialog_muted_style(theme: runie_core::types::ThemeKind) -> ratatui::style::Style {
    appearance::muted_style_for(theme).bg(appearance::background_style_for(theme)
        .bg
        .expect("panel background token"))
}

fn dialog_border_style(theme: runie_core::types::ThemeKind) -> ratatui::style::Style {
    // Keep modal chrome aligned with the prompt's primary border.
    appearance::prompt_border_style_for(theme)
}

fn dialog_header(
    frame: &DialogFrame,
    theme: runie_core::types::ThemeKind,
    inner_width: u16,
) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from("")];
    lines.push(Line::from(Span::styled(
        format!("  {}{}", dialog_hint(frame.spec.kind), frame.query),
        dialog_muted_style(theme),
    )));
    let divider = if frame.spec.kind == DialogKind::List {
        Line::from(Span::styled(
            "─".repeat(inner_width as usize),
            dialog_muted_style(theme),
        ))
    } else {
        Line::from("")
    };
    lines.push(divider);
    lines
}

fn dialog_footer(
    frame: &DialogFrame,
    theme: runie_core::types::ThemeKind,
    inner_width: u16,
) -> [Line<'static>; 2] {
    let actions = dialog_actions(frame, theme);
    let action_width = actions
        .iter()
        .map(|span| span.content.chars().count())
        .sum::<usize>();
    let left_pad = inner_width.saturating_sub(action_width as u16) / 2;
    let mut footer = vec![Span::raw(" ".repeat(left_pad as usize))];
    footer.extend(actions);
    [Line::from(""), Line::from(footer)]
}

fn append_dialog_rows(
    lines: &mut Vec<Line<'static>>,
    rows: &[String],
    selected: usize,
    theme: runie_core::types::ThemeKind,
    inner_width: u16,
    max_rows: u16,
) {
    let selected_row = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| !row.starts_with('§'))
        .nth(selected)
        .map(|(index, _)| index)
        .unwrap_or(0);
    let visible = max_rows.max(1) as usize;
    let start = selected_row
        .saturating_sub(visible / 2)
        .min(rows.len().saturating_sub(visible));
    let end = (start + visible).min(rows.len());
    let mut selectable = selectable_before(rows, start);
    let mut seen_group = false;
    for row in rows.iter().skip(start).take(end.saturating_sub(start)) {
        if let Some(header) = row.strip_prefix('§') {
            if seen_group {
                lines.push(Line::from(""));
            }
            lines.push(Line::from(Span::styled(
                format!("  {header}"),
                dialog_muted_style(theme),
            )));
            seen_group = true;
        } else {
            lines.push(dialog_row(selectable, row, selected, theme, inner_width));
            selectable += 1;
        }
    }
}

fn selectable_before(rows: &[String], end: usize) -> usize {
    rows.iter()
        .take(end)
        .filter(|row| !row.starts_with('§'))
        .count()
}

fn dialog_hint(kind: DialogKind) -> &'static str {
    match kind {
        // Runie's palette is always type-to-filter on open, matching Grok's
        // active search bar (` search: `). `/ to search` is only Grok's
        // unfocused vim-nav placeholder, a mode Runie does not expose.
        DialogKind::List => "search: ",
        DialogKind::Selector => "Select an item",
        DialogKind::Form => "Enter values: ",
        DialogKind::Confirm => "Confirm action",
        DialogKind::TextInput => "Enter text",
    }
}

fn dialog_row(
    index: usize,
    row: &str,
    selected_index: usize,
    theme: runie_core::types::ThemeKind,
    inner_width: u16,
) -> Line<'static> {
    // Selection is conveyed by the full-row background and bold text; dialog
    // items intentionally have no decorative per-item glyph.
    let prefix = "  ";
    let selected = index == selected_index;
    let selection_bg = appearance::selected_style_for(theme).bg.unwrap_or_default();
    let style = if selected {
        dialog_text_style(theme)
            .bg(selection_bg)
            .add_modifier(Modifier::BOLD)
    } else {
        dialog_text_style(theme)
    };
    let (label, detail) = row
        .split_once("  · ")
        .map_or((row, ""), |(label, detail)| (label, detail));
    let available = inner_width.saturating_sub(prefix.len() as u16) as usize;
    if inner_width < 60 {
        return narrow_dialog_row(label, prefix, style, inner_width, available);
    }
    wide_dialog_row(
        prefix,
        label,
        detail,
        selected,
        theme,
        available,
        style,
        selection_bg,
    )
}

#[allow(clippy::too_many_arguments)]
fn wide_dialog_row(
    prefix: &str,
    label: &str,
    detail: &str,
    selected: bool,
    theme: runie_core::types::ThemeKind,
    available: usize,
    style: ratatui::style::Style,
    selection_bg: ratatui::style::Color,
) -> Line<'static> {
    let detail_width = detail.chars().count();
    let label_width = available.saturating_sub(detail_width.saturating_add(1));
    let fitted_label = label.chars().take(label_width).collect::<String>();
    let padding = available.saturating_sub(fitted_label.chars().count() + detail_width);
    let fill_style = if selected {
        appearance::selected_style_for(theme)
    } else {
        appearance::panel_background_style_for(theme)
    };
    let detail_style = if selected {
        dialog_muted_style(theme).bg(selection_bg)
    } else {
        dialog_muted_style(theme)
    };
    Line::from(vec![
        Span::styled(format!("{prefix}{fitted_label}"), style),
        Span::styled(" ".repeat(padding), fill_style),
        Span::styled(detail.to_owned(), detail_style),
    ])
}

fn narrow_dialog_row(
    label: &str,
    prefix: &str,
    style: ratatui::style::Style,
    inner_width: u16,
    available: usize,
) -> Line<'static> {
    let fitted = label.chars().take(available).collect::<String>();
    let text = format!("{prefix}{fitted}");
    let padding = inner_width.saturating_sub(text.chars().count() as u16) as usize;
    Line::from(Span::styled(
        format!("{text}{}", " ".repeat(padding)),
        style,
    ))
}

fn dialog_actions(frame: &DialogFrame, theme: runie_core::types::ThemeKind) -> Vec<Span<'static>> {
    appearance::footer_hotkey_actions(
        theme,
        frame
            .spec
            .actions
            .iter()
            .filter(|action| action.enabled.evaluate(frame))
            .filter_map(|action| action.hotkey.map(|key| (key, action.label))),
    )
}

fn dialog_block(frame: &DialogFrame, theme: runie_core::types::ThemeKind) -> Block<'static> {
    let border = dialog_border_style(theme).add_modifier(Modifier::BOLD);
    let title = dialog_text_style(theme).add_modifier(Modifier::BOLD);
    Block::default()
        .style(appearance::background_style_for(theme))
        .title(Line::from(vec![
            Span::styled("─ ", border),
            Span::styled(frame.spec.title, title),
            Span::styled(" ─", border),
        ]))
        .borders(Borders::ALL)
        .border_style(dialog_border_style(theme))
}

#[cfg(test)]
#[path = "dialog_tests.rs"]
mod tests;
