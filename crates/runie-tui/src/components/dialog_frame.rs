use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Margin, Rect},
    prelude::Widget,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Clear},
};
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

        Clear.render(dialog_area, buf);

        let mut block = Block::bordered().border_style(Style::default().fg(accent));

        if let Some(title) = &self.title {
            block = block.title(
                Line::from(title.as_str())
                    .style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
                    .alignment(Alignment::Center),
            );
        }

        if self.show_close_hint {
            block = block.title_bottom(
                Line::from("[Esc] close")
                    .style(Style::default().fg(text_muted))
                    .alignment(Alignment::Right),
            );
        }

        block.render(dialog_area, buf);

        let inner = dialog_area.inner(Margin::new(2, 1));
        render_content(inner, buf);
    }
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x.saturating_add(area.width.saturating_sub(width) / 2);
    let y = area.y.saturating_add(area.height.saturating_sub(height) / 2);
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
