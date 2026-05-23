use ratatui::{
    buffer::Buffer,
    layout::{Margin, Rect},
    prelude::Widget,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Clear,
};
use crate::components::gradient_border::render_gradient_border;
use crate::theme::ThemeWrapper;

pub struct DialogFrame {
    pub width: u16,
    pub height: u16,
    pub title: Option<String>,
    pub show_close_hint: bool,
}

impl DialogFrame {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            title: None,
            show_close_hint: false,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn show_close_hint(mut self) -> Self {
        self.show_close_hint = true;
        self
    }

    pub fn render<F>(&self, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, render_content: F)
    where
        F: Fn(Rect, &mut Buffer),
    {
        let accent: Color = theme.color("accent.primary").into();
        let text_muted: Color = theme.color("text.muted").into();

        let dialog_area = centered_rect(area, self.width, self.height);

        // Clear dialog area
        Clear.render(dialog_area, buf);

        // Draw gradient border
        render_gradient_border(dialog_area, buf);

        // Draw title centered on top border row
        if let Some(title) = &self.title {
            let title_len = title.len() as u16;
            let title_x = dialog_area.x + (dialog_area.width.saturating_sub(title_len)) / 2;
            let title_line = Line::from(vec![Span::styled(
                title.as_str(),
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            )]);
            buf.set_line(title_x, dialog_area.y, &title_line, title_len);
        }

        // Draw close hint at bottom right
        if self.show_close_hint {
            let close_text = "[Esc] close";
            let close_len = close_text.len() as u16;
            let close_x = dialog_area.x + dialog_area.width.saturating_sub(close_len) - 1;
            let close_line = Line::from(vec![Span::styled(
                close_text,
                Style::default().fg(text_muted),
            )]);
            buf.set_line(close_x, dialog_area.y + dialog_area.height - 1, &close_line, close_len);
        }

        // Inner content area (accounting for 1-char border on each side)
        let inner = dialog_area.inner(Margin::new(1, 1));
        render_content(inner, buf);
    }
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x.saturating_add(area.width.saturating_sub(width) / 2);
    let y = area.y.saturating_add(area.height.saturating_sub(height) / 2);
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
