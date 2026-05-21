use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Widget},
};
use crate::theme::ThemeWrapper;

#[derive(Clone)]
pub struct Overlay {
    pub title: String,
    pub content: Vec<Vec<Span<'static>>>,
    pub tabs: Vec<String>,
    pub active_tab: usize,
    pub show_close: bool,
}

impl Default for Overlay {
    fn default() -> Self {
        Self {
            title: String::new(),
            content: Vec::new(),
            tabs: Vec::new(),
            active_tab: 0,
            show_close: true,
        }
    }
}

struct StyleHelpers {
    text_tertiary: Style,
    syntax_phase: Style,
    text_primary: Style,
    bg_panel: Style,
}

impl StyleHelpers {
    fn new(theme: &ThemeWrapper) -> Self {
        Self {
            text_tertiary: Style::default().fg(theme.color("text.dim").into()),
            syntax_phase: Style::default().fg(theme.color("accent.secondary").into()),
            text_primary: Style::default().fg(theme.color("text.primary").into()),
            bg_panel: Style::default().bg(theme.color("bg.panel").into()),
        }
    }
    fn tertiary(&self) -> Style {
        self.text_tertiary
    }
    fn phase(&self) -> Style {
        self.syntax_phase
    }
    fn primary(&self) -> Style {
        self.text_primary
    }
    fn bg_panel(&self) -> Style {
        self.bg_panel
    }
}

impl Widget for Overlay {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = ThemeWrapper::default();
        let sp = StyleHelpers::new(&theme);

        if area.width < 10 || area.height < 5 {
            return;
        }

        let text_tertiary: ratatui::style::Color = theme.color("text.dim").into();

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(text_tertiary))
            .title(self.title.as_str())
            .title_style(sp.phase());

        block.render(area, buf);

        if !self.title.is_empty() {
            let title_line = Line::from(vec![Span::styled(&self.title, sp.phase())]);
            buf.set_line(area.x + 2, area.y, &title_line, area.width - 4);
        }

        if self.show_close {
            let close_line = Line::from(vec![Span::styled("[x]", sp.tertiary())]);
            let close_x = area.x + area.width.saturating_sub(5);
            buf.set_line(close_x, area.y, &close_line, 4);
        }

        if !self.tabs.is_empty() {
            let mut tab_x = area.x + 2;
            for (i, tab) in self.tabs.iter().enumerate() {
                let tab_style = if i == self.active_tab {
                    sp.primary()
                } else {
                    sp.tertiary()
                };
                let tab_text = format!(" {} ", tab);
                let tab_line = Line::from(vec![Span::styled(&tab_text, tab_style)]);
                buf.set_line(tab_x, area.y + 1, &tab_line, tab_text.len() as u16);
                tab_x += tab_text.len() as u16 + 1;
            }
        }

        let content_start_y = if self.tabs.is_empty() { 1 } else { 2 };
        for y in (area.y + content_start_y)..(area.y + area.height - 1) {
            for x in (area.x + 1)..(area.x + area.width - 1) {
                buf.get_mut(x, y).set_style(sp.bg_panel());
            }
        }

        let content_y_offset = if self.tabs.is_empty() { 2 } else { 3 };
        for (i, line_spans) in self.content.iter().enumerate() {
            let y = area.y + content_y_offset + i as u16;
            if y >= area.y + area.height - 1 {
                break;
            }
            let line = Line::from(line_spans.clone());
            buf.set_line(area.x + 2, y, &line, area.width - 4);
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
        let bg_panel: ratatui::style::Color = theme.color("bg.panel").into();
        let text_tertiary: ratatui::style::Color = theme.color("text.dim").into();
        let syntax_phase: ratatui::style::Color = theme.color("accent.secondary").into();

        // Clear the area
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf[(x, y)].set_style(Style::default().bg(bg_panel));
            }
        }

        // Draw border
        for x in area.x..area.x + area.width {
            buf[(x, area.y)].set_symbol("─");
            buf[(x, area.y + area.height - 1)].set_symbol("─");
        }
        for y in area.y..area.y + area.height {
            buf[(area.x, y)].set_symbol("│");
            buf[(area.x + area.width - 1, y)].set_symbol("│");
        }
        buf[(area.x, area.y)].set_symbol("┌");
        buf[(area.x + area.width - 1, area.y)].set_symbol("┐");
        buf[(area.x, area.y + area.height - 1)].set_symbol("└");
        buf[(area.x + area.width - 1, area.y + area.height - 1)].set_symbol("┘");

        // Title
        if !self.title.is_empty() {
            buf.set_string(area.x + 2, area.y, &self.title, Style::default().fg(syntax_phase));
        }

        // Close button
        if self.show_close {
            buf.set_string(area.x + area.width - 4, area.y, "[×]", Style::default().fg(text_tertiary));
        }
    }
}
