//! Context usage modal — shows token usage and context statistics.
//!
//! Displays current session and turn token usage with cost estimates.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Widget,
};
use crate::theme::ThemeWrapper;

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
        let text_primary = Color::White;
        let text_secondary = Color::Gray;
        let highlight_bg = Color::Rgb(40, 40, 40);

        // Clear area
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf.get_mut(x, y).set_bg(bg);
            }
        }

        // Border
        for x in area.left()..area.right() {
            buf.get_mut(x, area.top()).set_fg(border);
            buf.get_mut(x, area.bottom().saturating_sub(1)).set_fg(border);
        }
        for y in area.top()..area.bottom() {
            buf.get_mut(area.left(), y).set_fg(border);
            buf.get_mut(area.right().saturating_sub(1), y).set_fg(border);
        }

        let inner = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };

        // Header
        let header = " Context Usage ";
        let header_style = Style::default().fg(accent).add_modifier(Modifier::BOLD);
        buf.set_string(
            inner.x + inner.width.saturating_sub(header.len() as u16) / 2,
            inner.y,
            header,
            header_style,
        );

        // Divider
        if inner.height > 1 {
            let divider = "─".repeat(inner.width as usize);
            buf.set_string(inner.x, inner.y + 1, &divider, Style::default().fg(border));
        }
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

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let text_primary = Color::White;
    let text_secondary = Color::Gray;
    let accent = Color::Cyan;
    let green = Color::Green;
    let yellow = Color::Yellow;

    let model = state.current_model.as_deref().unwrap_or("—");
    let session = &state.session_token_usage;
    let turn = &state.token_usage;

    let mut row = inner.y + 3;
    let max_row = inner.y + inner.height;

    macro_rules! draw {
        ($label:expr, $value:expr, $color:expr) => {
            if row < max_row {
                buf.set_string(inner.x + 2, row, $label, Style::default().fg(text_secondary));
                buf.set_string(inner.x + 20, row, &$value, Style::default().fg($color));
                row += 1;
            }
        };
    }

    draw!("Model:", model.to_string(), accent);
    row += 1;
    draw!("Session Prompt:", format!("{} tok", session.prompt_tokens), text_primary);
    draw!("Session Completion:", format!("{} tok", session.completion_tokens), text_primary);
    draw!("Session Total:", format!("{} tok", session.total_tokens), green);
    draw!("Session Cost:", format!("${:.4}", session.estimated_cost), yellow);
    row += 1;
    draw!("Turn Total:", format!("{} tok", turn.total_tokens), text_primary);
    if let Some(duration) = state.last_turn_duration_secs {
        draw!("Turn Duration:", format!("{}s", duration), text_primary);
    }
    if let Some(tool_calls) = state.last_turn_tool_calls {
        draw!("Tool Calls:", format!("{}", tool_calls), text_primary);
    }
    row += 1;

    let context_window = state.top_bar.context_window.unwrap_or(128_000);
    let estimated = state.top_bar.estimated_tokens.unwrap_or(session.total_tokens);
    let remaining = context_window.saturating_sub(estimated);
    let pct = (estimated as f64 / context_window as f64 * 100.0).min(100.0);

    draw!("Context Window:", format!("{} tok", context_window), text_secondary);
    draw!("Estimated Usage:", format!("{} tok", estimated), text_primary);
    draw!("Remaining:", format!("{} tok", remaining), green);
    draw!("Utilization:", format!("{:.1}%", pct), if pct > 80.0 { Color::Red } else { accent });

    // Footer
    if inner.height > 2 {
        let footer = "Esc:close";
        let footer_y = inner.y + inner.height.saturating_sub(1);
        buf.set_string(inner.x, footer_y, footer, Style::default().fg(text_secondary));
    }
}
