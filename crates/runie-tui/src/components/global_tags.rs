use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use crate::glyphs;

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
    /// Create idle state: "{tokens} tok | ${cost}" or empty
    pub fn idle(_model: &str, tokens: u64, cost: f64) -> Self {
        Self {
            left: None,
            right: if tokens > 0 {
                format!("{} tok | ${:.4}", tokens, cost)
            } else {
                String::new()
            },
        }
    }

    /// Create running state: "⣾ {status} [turn: {time}] [⇣{tokens}]"
    pub fn running(status: &str, time: &str, tokens: u64) -> Self {
        Self {
            left: Some(format!("{} {}", glyphs::SPINNER_FRAMES[0], status)),
            right: format!("[turn: {}] [⇣{}]", time, tokens),
        }
    }

    /// Create error state: "Error" (no spinner, no turn info)
    pub fn error(status: &str) -> Self {
        Self {
            left: None,
            right: format!("[{}]", status),
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

        let right_len = self.right.len();

        // If nothing to show, return early
        if self.right.is_empty() && self.left.is_none() {
            return;
        }

        // Render left part — align to inner left edge (account for border at x=0)
        let x = area.x + 1;
        if let Some(ref left) = self.left {
            let left_style = Style::default()
                .fg(accent)
                .add_modifier(Modifier::DIM);
            let span = Span::styled(left.clone(), left_style);
            buf.set_line(x, area.y, &Line::from(span), left.len() as u16);
        }

        // Render right part — right-aligned to inner right edge (account for border)
        let right_x = area.x + (area.width.saturating_sub(right_len as u16 + 1));
        let right_style = Style::default()
            .fg(text_dim)
            .add_modifier(Modifier::DIM);
        let span = Span::styled(self.right.clone(), right_style);
        buf.set_line(right_x, area.y, &Line::from(span), right_len as u16);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_tags_renders_at_area_y_not_zero() {
        let vm = GlobalTagsViewModel::idle("openai/gpt-4o", 1000, 0.05);
        let area = Rect::new(5, 10, 80, 1);
        let mut buf = Buffer::empty(Rect::new(0, 0, 100, 20));

        Widget::render(vm, area, &mut buf);

        let any_content_at_y0: bool = (0..80).any(|x| {
            buf.cell((x, 0)).map(|c| c.symbol() != " ").unwrap_or(false)
        });
        assert!(!any_content_at_y0, "y=0 should be empty when area.y=10");
    }

    #[test]
    fn test_global_tags_running_state_position() {
        let vm = GlobalTagsViewModel::running("thinking", "5s", 200);
        let area = Rect::new(0, 15, 80, 1);
        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 20));

        Widget::render(vm, area, &mut buf);

        let cell_at_y15 = buf.cell((1, 15));
        let cell_at_y0 = buf.cell((1, 0));

        assert!(
            cell_at_y15.is_some() && cell_at_y15.unwrap().symbol() == "⠋",
            "Expected spinner at y=15, found: {:?}",
            cell_at_y15.map(|c| c.symbol())
        );
        assert!(
            cell_at_y0.is_none() || cell_at_y0.unwrap().symbol() == " ",
            "y=0 should be empty, found: {:?}",
            cell_at_y0.map(|c| c.symbol())
        );
    }
}
