use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Widget},
};
use crate::theme::ThemeWrapper;

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

        let bg_panel: ratatui::style::Color = theme.color("bg.panel").into();
        let text_tertiary: ratatui::style::Color = theme.color("text.dim").into();
        let syntax_phase: ratatui::style::Color = theme.color("accent.secondary").into();

        Clear.render(area, buf);

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(text_tertiary))
            .style(Style::default().bg(bg_panel))
            .title(self.title.as_str())
            .title_style(Style::default().fg(syntax_phase));
        block.render(area, buf);

        if self.show_close {
            let close_line = Line::from(vec![Span::styled("[x]", Style::default().fg(text_tertiary))]);
            let close_x = area.x + area.width.saturating_sub(5);
            buf.set_line(close_x, area.y, &close_line, 4);
        }

        render_tabs(self, area, buf, text_tertiary, syntax_phase);
        fill_content_area(self, area, buf);
    }
}

fn render_tabs(
    overlay: &Overlay,
    area: Rect,
    buf: &mut Buffer,
    text_tertiary: ratatui::style::Color,
    syntax_phase: ratatui::style::Color,
) {
    if overlay.tabs.is_empty() {
        return;
    }

    let mut tab_x = area.x + 2;
    for (i, tab) in overlay.tabs.iter().enumerate() {
        let tab_style = if i == overlay.active_tab {
            Style::default().fg(syntax_phase)
        } else {
            Style::default().fg(text_tertiary)
        };
        let tab_text = format!(" {} ", tab);
        let tab_line = Line::from(vec![Span::styled(&tab_text, tab_style)]);
        buf.set_line(tab_x, area.y + 1, &tab_line, tab_text.len() as u16);
        tab_x += tab_text.len() as u16 + 1;
    }
}

fn fill_content_area(overlay: &Overlay, area: Rect, buf: &mut Buffer) {
    let content_start_y = if overlay.tabs.is_empty() { 1 } else { 2 };
    let content_rect = Rect::new(
        area.x + 1,
        area.y + content_start_y,
        area.width - 2,
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
        buf.set_line(area.x + 2, y, &line, area.width - 4);
    }
}
