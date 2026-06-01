use ratatui::{
    buffer::Buffer,
    layout::Rect,
    prelude::Widget,
    style::Color,
    widgets::Clear,
};
use crate::components::panel::Panel;
use crate::theme::ThemeWrapper;

pub struct DialogFrame {
    pub width: u16,
    pub height: u16,
    pub title: Option<String>,
    pub show_close_hint: bool,
}

impl DialogFrame {

    #[must_use]
    #[must_use]
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
        let border_unfocused: Color = theme.color("border.unfocused").into();
        let bg_base: Color = theme.color("bg.base").into();

        let dialog_area = centered_rect(area, self.width, self.height);

        // Clear dialog area
        Clear.render(dialog_area, buf);

        // Draw panel with gradient border and base background
        let mut panel = Panel::new()
            .border_gradient(border_unfocused, accent)
            .title_color(accent)
            .title_right()
            .bg(bg_base);

        if let Some(ref title) = self.title {
            panel = panel.title(title.as_str());
        }

        if self.show_close_hint {
            panel = panel.show_close_hint(text_muted);
        }

        panel.render(dialog_area, buf, |inner, buf| {
            render_content(inner, buf);
        });
    }
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x.saturating_add(area.width.saturating_sub(width) / 2);
    let y = area.y.saturating_add(area.height.saturating_sub(height) / 2);
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}