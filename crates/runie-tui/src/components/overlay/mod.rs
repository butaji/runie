use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Clear, Widget},
};
use crate::components::panel::Panel;
use crate::style::layout::PADDING_X;
use crate::style::StyleSet;
use crate::theme::ThemeWrapper;

pub mod builder;
pub use builder::*;

#[derive(Clone)]
pub struct Overlay {
    pub title: String,
    pub content: Vec<Vec<Span<'static>>>,
    pub tabs: Vec<String>,
    pub active_tab: usize,
    pub show_close: bool,
    pub theme: ThemeWrapper,
}

impl Default for Overlay {
    fn default() -> Self {
        Self {
            title: String::new(),
            content: Vec::new(),
            tabs: Vec::new(),
            active_tab: 0,
            show_close: true,
            theme: ThemeWrapper::default(),
        }
    }
}

impl Overlay {
    pub fn centered(size: (u16, u16), screen: Rect) -> Rect {
        let x = screen.x + (screen.width.saturating_sub(size.0)) / 2;
        let y = screen.y + (screen.height.saturating_sub(size.1)) / 2;
        Rect::new(x, y, size.0, size.1)
    }

    pub fn render_ref(&self, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
        if area.width < 10 || area.height < 5 {
            return;
        }

        let styles = StyleSet::from_theme(theme);
        let text_tertiary = styles.text_dim;
        let syntax_phase = styles.accent;

        // Clear area first
        Clear.render(area, buf);

        // Draw panel border (without title since we render tabs in that space)
        Panel::new()
            .border_gradient(styles.border.fg.unwrap_or(text_tertiary.fg.unwrap()), syntax_phase.fg.unwrap())
            .render(area, buf, |_inner, _buf| {
                // Inner content is rendered separately below
            });

        // Draw title centered on top border row
        let title_len = self.title.len() as u16;
        let title_x = area.x + (area.width.saturating_sub(title_len)) / 2;
        let title_line = Line::from(vec![Span::styled(
            self.title.as_str(),
            syntax_phase,
        )]);
        buf.set_line(title_x, area.y, &title_line, title_len);

        if self.show_close {
            let close_line = Line::from(vec![Span::styled("[x]", text_tertiary)]);
            let close_x = area.x + area.width.saturating_sub(5);
            buf.set_line(close_x, area.y, &close_line, 4);
        }

        render_tabs(self, area, buf, &styles);
        fill_content_area(self, area, buf, PADDING_X);
    }
}

fn render_tabs(
    overlay: &Overlay,
    area: Rect,
    buf: &mut Buffer,
    styles: &StyleSet,
) {
    if overlay.tabs.is_empty() {
        return;
    }

    let mut tab_x = area.x + 2;
    for (i, tab) in overlay.tabs.iter().enumerate() {
        let tab_style = if i == overlay.active_tab {
            styles.accent
        } else {
            styles.text_dim
        };
        let tab_text = format!(" {} ", tab);
        let tab_line = Line::from(vec![Span::styled(&tab_text, tab_style)]);
        let tab_width = tab_text.chars().count() as u16;
        buf.set_line(tab_x, area.y + 1, &tab_line, tab_width);
        tab_x += tab_width + 1;
    }
}

fn fill_content_area(overlay: &Overlay, area: Rect, buf: &mut Buffer, padding_x: u16) {
    let content_start_y = if overlay.tabs.is_empty() { 1 } else { 2 };
    let content_rect = Rect::new(
        area.x + padding_x,
        area.y + content_start_y,
        area.width - padding_x * 2,
        area.height - content_start_y - 1,
    );
    Clear.render(content_rect, buf);

    let content_y_offset = if overlay.tabs.is_empty() { 2 } else { 3 };
    for (i, line_spans) in overlay.content.iter().enumerate() {
        let y = area.y + content_y_offset + i as u16;
        if y >= area.y + area.height - 1 {
            break;
        }
        let line = Line::from(line_spans.clone());
        buf.set_line(area.x + padding_x, y, &line, area.width - padding_x * 2);
    }
}