//! Context usage modal — shows token usage and context statistics.
//!
//! Displays current session and turn token usage with cost estimates.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};
use crate::style::box_chars::H;
use crate::style::layout::{PADDING_X, PADDING_WIDTH};
use crate::style::StyleSet;
use crate::theme::ThemeWrapper;

macro_rules! draw_row {
    ($buf:expr, $inner:expr, $row:expr, $max_row:expr, $label:expr, $value:expr, $color:expr, $secondary:expr) => {
        if *$row < $max_row {
            $buf.set_string($inner.x + 2, *$row, $label, Style::default().fg($secondary));
            $buf.set_string($inner.x + 20, *$row, &$value, Style::default().fg($color));
            *$row += 1;
        }
    };
}

/// Context usage modal state.
#[derive(Debug, Clone, Default)]
pub struct ContextUsageModal {
    pub open: bool,
}

impl ContextUsageModal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }
}

impl Widget for &ContextUsageModal {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = ThemeWrapper::default();
        let styles = StyleSet::from_theme(&theme);

        clear_modal_area(area, buf, &styles);
        draw_modal_border(area, buf, &styles);

        let inner = Rect {
            x: area.x + PADDING_X,
            y: area.y + 1,
            width: area.width.saturating_sub(PADDING_WIDTH),
            height: area.height.saturating_sub(2),
        };

        render_modal_header(inner, buf, &styles);
    }
}

fn clear_modal_area(area: Rect, buf: &mut Buffer, styles: &StyleSet) {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_bg(styles.muted.bg.unwrap_or(Color::Rgb(25, 25, 25)));
            }
        }
    }
}

fn draw_modal_border(area: Rect, buf: &mut Buffer, styles: &StyleSet) {
    let border_color = styles.border.fg.unwrap_or(Color::DarkGray);
    for x in area.left()..area.right() {
        if let Some(cell) = buf.cell_mut((x, area.top())) {
            cell.set_fg(border_color);
        }
        if let Some(cell) = buf.cell_mut((x, area.bottom().saturating_sub(1))) {
            cell.set_fg(border_color);
        }
    }
    for y in area.top()..area.bottom() {
        if let Some(cell) = buf.cell_mut((area.left(), y)) {
            cell.set_fg(border_color);
        }
        if let Some(cell) = buf.cell_mut((area.right().saturating_sub(1), y)) {
            cell.set_fg(border_color);
        }
    }
}

fn render_modal_header(inner: Rect, buf: &mut Buffer, styles: &StyleSet) {
    let header = " Context Usage ";
    let header_style = styles.accent.add_modifier(Modifier::BOLD);
    buf.set_string(
        inner.x + inner.width.saturating_sub(header.len() as u16) / 2,
        inner.y,
        header,
        header_style,
    );

    if inner.height > 1 {
        let divider: String = std::iter::repeat(H).take(inner.width as usize).collect();
        buf.set_string(inner.x, inner.y + 1, &divider, styles.border);
    }
}

/// Render context usage modal with state data.
pub fn render_context_usage_modal(
    modal: &ContextUsageModal,
    state: &crate::tui::state::AppState,
    area: Rect,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
) {
    modal.render(area, buf);

    let inner = inner_rect(area);
    let styles = StyleSet::from_theme(theme);

    let model = state.current_model.as_deref().unwrap_or("—");
    let session = &state.session_token_usage;
    let turn = &state.token_usage;

    let mut row = inner.y + 3;
    let max_row = inner.y + inner.height;

    draw_row!(buf, inner, &mut row, max_row, "Model:", model.to_string(), styles.accent.fg.unwrap_or(Color::Cyan), styles.text_secondary.fg.unwrap_or(Color::Gray));
    render_session_stats(buf, inner, &mut row, max_row, session, &styles);
    render_turn_stats(buf, inner, &mut row, max_row, turn, state, &styles);
    render_context_stats(buf, inner, &mut row, max_row, state, session, &styles);
    render_footer(buf, inner, &styles);
}

fn inner_rect(area: Rect) -> Rect {
    Rect {
        x: area.x + PADDING_X,
        y: area.y + 1,
        width: area.width.saturating_sub(PADDING_WIDTH),
        height: area.height.saturating_sub(2),
    }
}

fn render_session_stats(
    buf: &mut Buffer,
    inner: Rect,
    row: &mut u16,
    max_row: u16,
    session: &runie_ai::TokenUsage,
    styles: &StyleSet,
) {
    draw_row!(buf, inner, row, max_row, "Session Prompt:", format!("{} tok", session.prompt_tokens), styles.text_primary.fg.unwrap_or(Color::White), styles.text_secondary.fg.unwrap_or(Color::Gray));
    draw_row!(buf, inner, row, max_row, "Session Completion:", format!("{} tok", session.completion_tokens), styles.text_primary.fg.unwrap_or(Color::White), styles.text_secondary.fg.unwrap_or(Color::Gray));
    draw_row!(buf, inner, row, max_row, "Session Total:", format!("{} tok", session.total_tokens), styles.success.fg.unwrap_or(Color::Green), styles.text_secondary.fg.unwrap_or(Color::Gray));
    draw_row!(buf, inner, row, max_row, "Session Cost:", format!("${:.4}", session.estimated_cost), styles.warning.fg.unwrap_or(Color::Yellow), styles.text_secondary.fg.unwrap_or(Color::Gray));
}

fn render_turn_stats(
    buf: &mut Buffer,
    inner: Rect,
    row: &mut u16,
    max_row: u16,
    turn: &runie_ai::TokenUsage,
    state: &crate::tui::state::AppState,
    styles: &StyleSet,
) {
    draw_row!(buf, inner, row, max_row, "Turn Total:", format!("{} tok", turn.total_tokens), styles.text_primary.fg.unwrap_or(Color::White), styles.text_secondary.fg.unwrap_or(Color::Gray));
    if let Some(duration) = state.last_turn_duration_secs {
        draw_row!(buf, inner, row, max_row, "Turn Duration:", format!("{}s", duration), styles.text_primary.fg.unwrap_or(Color::White), styles.text_secondary.fg.unwrap_or(Color::Gray));
    }
    if let Some(tool_calls) = state.last_turn_tool_calls {
        draw_row!(buf, inner, row, max_row, "Tool Calls:", format!("{}", tool_calls), styles.text_primary.fg.unwrap_or(Color::White), styles.text_secondary.fg.unwrap_or(Color::Gray));
    }
}

fn render_context_stats(
    buf: &mut Buffer,
    inner: Rect,
    row: &mut u16,
    max_row: u16,
    state: &crate::tui::state::AppState,
    session: &runie_ai::TokenUsage,
    styles: &StyleSet,
) {
    let context_window = state.top_bar.context_window.unwrap_or(128_000);
    let estimated = state.top_bar.estimated_tokens.unwrap_or(session.total_tokens);
    let remaining = context_window.saturating_sub(estimated);
    let pct = (estimated as f64 / context_window as f64 * 100.0).min(100.0);

    draw_row!(buf, inner, row, max_row, "Context Window:", format!("{} tok", context_window), styles.text_secondary.fg.unwrap_or(Color::Gray), styles.text_secondary.fg.unwrap_or(Color::Gray));
    draw_row!(buf, inner, row, max_row, "Estimated Usage:", format!("{} tok", estimated), styles.text_primary.fg.unwrap_or(Color::White), styles.text_secondary.fg.unwrap_or(Color::Gray));
    draw_row!(buf, inner, row, max_row, "Remaining:", format!("{} tok", remaining), styles.success.fg.unwrap_or(Color::Green), styles.text_secondary.fg.unwrap_or(Color::Gray));
    let util_color = if pct > 80.0 { Color::Red } else { styles.accent.fg.unwrap_or(Color::Cyan) };
    draw_row!(buf, inner, row, max_row, "Utilization:", format!("{:.1}%", pct), util_color, styles.text_secondary.fg.unwrap_or(Color::Gray));
}

fn render_footer(buf: &mut Buffer, inner: Rect, styles: &StyleSet) {
    if inner.height > 2 {
        let footer = "Esc:close";
        let footer_y = inner.y + inner.height.saturating_sub(1);
        buf.set_string(inner.x, footer_y, footer, styles.muted);
    }
}
