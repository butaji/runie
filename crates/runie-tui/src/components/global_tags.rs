use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

/// GlobalTagsViewModel holds tags to display between feed and input.
#[derive(Debug, Clone)]
pub struct GlobalTagsViewModel {
    pub left: Option<String>,
    pub right: String,
}

impl Default for GlobalTagsViewModel {
    fn default() -> Self {
        Self {
            left: None,
            right: String::new(),
        }
    }
}

impl GlobalTagsViewModel {
    /// Create idle state: "{model} | {tokens} tok | ${cost}"
    pub fn idle(model: &str, tokens: u64, cost: f64) -> Self {
        Self {
            left: None,
            right: if tokens > 0 {
                format!("{} | {} tok | ${:.4}", model, tokens, cost)
            } else {
                model.to_string()
            },
        }
    }

    /// Create running state: "⣾ {status} [turn: {time}] [⇣{tokens}]"
    pub fn running(status: &str, time: &str, tokens: u64) -> Self {
        Self {
            left: Some(format!("⣾ {}", status)),
            right: format!("[turn: {}] [⇣{}]", time, tokens),
        }
    }
}

impl Widget for GlobalTagsViewModel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let bg_base: ratatui::style::Color = ratatui::style::Color::Reset;
        let text_dim: ratatui::style::Color = ratatui::style::Color::DarkGray;
        let accent: ratatui::style::Color = ratatui::style::Color::Blue;

        // Fill background
        for y in 0..area.height {
            for x in 0..area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_style(Style::default().bg(bg_base));
                }
            }
        }

        let width = area.width as usize;
        let right_len = self.right.len();

        // If nothing to show, return early
        if self.right.is_empty() && self.left.is_none() {
            return;
        }

        // Render left part
        let mut x = 1u16;
        if let Some(ref left) = self.left {
            let left_style = Style::default()
                .fg(accent)
                .add_modifier(Modifier::DIM);
            let span = Span::styled(left.clone(), left_style);
            buf.set_line(x, 0, &Line::from(span), left.len() as u16);
            x += left.len() as u16;
            x += 2; // spacing
        }

        // Render right part (right-aligned)
        let right_x = (width.saturating_sub(right_len + 1)) as u16;
        let right_style = Style::default()
            .fg(text_dim)
            .add_modifier(Modifier::DIM);
        let span = Span::styled(self.right.clone(), right_style);
        buf.set_line(right_x, 0, &Line::from(span), right_len as u16);
    }
}
