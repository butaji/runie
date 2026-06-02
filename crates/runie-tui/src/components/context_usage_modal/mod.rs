//! Context usage modal — shows token usage and context statistics.
//!
//! Displays current session and turn token usage with cost estimates.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};
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
        let bg = Color::Rgb(25, 25, 25);
        let border = Color::DarkGray;
        let accent = Color::Cyan;

        clear_modal_area(area, buf, bg);
        draw_modal_border(area, buf, border);

        let inner = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };

        render_modal_header(inner, buf, accent, border);
    }
}

fn clear_modal_area(area: Rect, buf: &mut Buffer, bg: Color) {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            buf.get_mut(x, y).set_bg(bg);
        }
    }
}

fn draw_modal_border(area: Rect, buf: &mut Buffer, border: Color) {
    for x in area.left()..area.right() {
        buf.get_mut(x, area.top()).set_fg(border);
        buf.get_mut(x, area.bottom().saturating_sub(1)).set_fg(border);
    }
    for y in area.top()..area.bottom() {
        buf.get_mut(area.left(), y).set_fg(border);
        buf.get_mut(area.right().saturating_sub(1), y).set_fg(border);
    }
}

fn render_modal_header(inner: Rect, buf: &mut Buffer, accent: Color, border: Color) {
    let header = " Context Usage ";
    let header_style = Style::default().fg(accent).add_modifier(Modifier::BOLD);
    buf.set_string(
        inner.x + inner.width.saturating_sub(header.len() as u16) / 2,
        inner.y,
        header,
        header_style,
    );

    if inner.height > 1 {
        let divider = "─".repeat(inner.width as usize);
        buf.set_string(inner.x, inner.y + 1, &divider, Style::default().fg(border));
    }
}

/// Render context usage modal with state data.
pub fn render_context_usage_modal(
    modal: &ContextUsageModal,
    state: &crate::tui::state::AppState,
    area: Rect,
    buf: &mut Buffer,
    _theme: &ThemeWrapper,
) {
    modal.render(area, buf);

    let inner = inner_rect(area);
    let colors = ModalColors::default();

    let model = state.current_model.as_deref().unwrap_or("—");
    let session = &state.session_token_usage;
    let turn = &state.token_usage;

    let mut row = inner.y + 3;
    let max_row = inner.y + inner.height;

    draw_row!(buf, inner, &mut row, max_row, "Model:", model.to_string(), colors.accent, colors.text_secondary);
    render_session_stats(buf, inner, &mut row, max_row, session, &colors);
    render_turn_stats(buf, inner, &mut row, max_row, turn, state, &colors);
    render_context_stats(buf, inner, &mut row, max_row, state, session, &colors);
    render_footer(buf, inner, &colors);
}

fn inner_rect(area: Rect) -> Rect {
    Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}

struct ModalColors {
    text_primary: Color,
    text_secondary: Color,
    accent: Color,
    green: Color,
    yellow: Color,
}

impl Default for ModalColors {
    fn default() -> Self {
        Self {
            text_primary: Color::White,
            text_secondary: Color::Gray,
            accent: Color::Cyan,
            green: Color::Green,
            yellow: Color::Yellow,
        }
    }
}

fn render_session_stats(
    buf: &mut Buffer,
    inner: Rect,
    row: &mut u16,
    max_row: u16,
    session: &runie_ai::TokenUsage,
    colors: &ModalColors,
) {
    draw_row!(buf, inner, row, max_row, "Session Prompt:", format!("{} tok", session.prompt_tokens), colors.text_primary, colors.text_secondary);
    draw_row!(buf, inner, row, max_row, "Session Completion:", format!("{} tok", session.completion_tokens), colors.text_primary, colors.text_secondary);
    draw_row!(buf, inner, row, max_row, "Session Total:", format!("{} tok", session.total_tokens), colors.green, colors.text_secondary);
    draw_row!(buf, inner, row, max_row, "Session Cost:", format!("${:.4}", session.estimated_cost), colors.yellow, colors.text_secondary);
}

fn render_turn_stats(
    buf: &mut Buffer,
    inner: Rect,
    row: &mut u16,
    max_row: u16,
    turn: &runie_ai::TokenUsage,
    state: &crate::tui::state::AppState,
    colors: &ModalColors,
) {
    draw_row!(buf, inner, row, max_row, "Turn Total:", format!("{} tok", turn.total_tokens), colors.text_primary, colors.text_secondary);
    if let Some(duration) = state.last_turn_duration_secs {
        draw_row!(buf, inner, row, max_row, "Turn Duration:", format!("{}s", duration), colors.text_primary, colors.text_secondary);
    }
    if let Some(tool_calls) = state.last_turn_tool_calls {
        draw_row!(buf, inner, row, max_row, "Tool Calls:", format!("{}", tool_calls), colors.text_primary, colors.text_secondary);
    }
}

fn render_context_stats(
    buf: &mut Buffer,
    inner: Rect,
    row: &mut u16,
    max_row: u16,
    state: &crate::tui::state::AppState,
    session: &runie_ai::TokenUsage,
    colors: &ModalColors,
) {
    let context_window = state.top_bar.context_window.unwrap_or(128_000);
    let estimated = state.top_bar.estimated_tokens.unwrap_or(session.total_tokens);
    let remaining = context_window.saturating_sub(estimated);
    let pct = (estimated as f64 / context_window as f64 * 100.0).min(100.0);

    draw_row!(buf, inner, row, max_row, "Context Window:", format!("{} tok", context_window), colors.text_secondary, colors.text_secondary);
    draw_row!(buf, inner, row, max_row, "Estimated Usage:", format!("{} tok", estimated), colors.text_primary, colors.text_secondary);
    draw_row!(buf, inner, row, max_row, "Remaining:", format!("{} tok", remaining), colors.green, colors.text_secondary);
    let util_color = if pct > 80.0 { Color::Red } else { colors.accent };
    draw_row!(buf, inner, row, max_row, "Utilization:", format!("{:.1}%", pct), util_color, colors.text_secondary);
}

fn render_footer(buf: &mut Buffer, inner: Rect, colors: &ModalColors) {
    if inner.height > 2 {
        let footer = "Esc:close";
        let footer_y = inner.y + inner.height.saturating_sub(1);
        buf.set_string(inner.x, footer_y, footer, Style::default().fg(colors.text_secondary));
    }
}
