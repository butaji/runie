//! Home screen — grok-style welcome overlay.
//!
//! Shows when no session is active. Menu items with keyboard hints.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Widget,
};
use crate::theme::ThemeWrapper;

/// Home screen menu items.
pub static HOME_MENU_ITEMS: &[(&str, &str, &str)] = &[
    ("New Session", "Start a new chat", "n"),
    ("Resume Last Session", "Continue where you left off", "r"),
    ("Settings", "Configure preferences", "s"),
    ("Help", "Show keyboard shortcuts", "h"),
    ("Quit", "Exit runie", "q"),
];

/// Home screen state.
#[derive(Debug, Clone, Default)]
pub struct HomeScreen {
    pub visible: bool,
    pub selected: usize,
}

impl HomeScreen {
    pub fn new() -> Self {
        Self { visible: true, selected: 0 }
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.selected = 0;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < HOME_MENU_ITEMS.len() {
            self.selected += 1;
        }
    }

    pub fn selected_action(&self) -> &str {
        HOME_MENU_ITEMS.get(self.selected).map(|(action, _, _)| *action).unwrap_or("")
    }
}

fn render_home_bg(area: Rect, buf: &mut Buffer, bg: Color) {
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) { cell.set_bg(bg); }
        }
    }
}

fn render_home_title(content_x: u16, content_y: u16, content_width: u16, buf: &mut Buffer, text_primary: Color, text_muted: Color) {
    let title = "runie";
    let title_x = content_x + (content_width - title.len() as u16) / 2;
    buf.set_line(title_x, content_y, &Line::raw(title).style(Style::default().fg(text_primary).add_modifier(Modifier::BOLD)), content_width);
    let subtitle = "Your coding companion";
    let subtitle_x = content_x + (content_width - subtitle.len() as u16) / 2;
    buf.set_line(subtitle_x, content_y + 1, &Line::raw(subtitle).style(Style::default().fg(text_muted)), content_width);
}

fn render_home_menu(screen: &HomeScreen, area: Rect, content_x: u16, div_y: u16, content_width: u16, buf: &mut Buffer, text_primary: Color, text_muted: Color, accent: Color) {
    let mut y = div_y + 2;
    for (i, (name, desc, hint)) in HOME_MENU_ITEMS.iter().enumerate() {
        if y >= area.bottom() { break; }
        let is_selected = i == screen.selected;
        let indicator = if is_selected { "▸" } else { " " };
        let indicator_style = if is_selected { Style::default().fg(accent).add_modifier(Modifier::BOLD) } else { Style::default().fg(text_muted) };
        buf.set_string(content_x + 2, y, indicator, indicator_style);
        let name_style = if is_selected { Style::default().fg(text_primary).add_modifier(Modifier::BOLD) } else { Style::default().fg(text_primary) };
        buf.set_string(content_x + 4, y, name, name_style);
        let hint_text = format!("{}", hint);
        let hint_x = content_x + content_width - hint_text.len() as u16 - 2;
        buf.set_string(hint_x, y, &hint_text, Style::default().fg(text_muted));
        let desc_y = y + 1;
        if desc_y < area.bottom() {
            buf.set_string(content_x + 4, desc_y, desc, Style::default().fg(text_muted));
        }
        y += 3;
    }
}

impl Widget for &HomeScreen {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let bg = Color::Black;
        let border = Color::DarkGray;
        let text_primary = Color::White;
        let text_muted = Color::DarkGray;
        let accent = Color::Cyan;
        render_home_bg(area, buf, bg);
        let content_width = 50u16;
        let content_height = 20u16;
        let content_x = area.x + (area.width.saturating_sub(content_width)) / 2;
        let content_y = area.y + (area.height.saturating_sub(content_height)) / 2;
        render_home_title(content_x, content_y, content_width, buf, text_primary, text_muted);
        let div_y = content_y + 3;
        for x in content_x..content_x + content_width { buf.get_mut(x, div_y).set_char('─').set_fg(border); }
        render_home_menu(self, area, content_x, div_y, content_width, buf, text_primary, text_muted, accent);
        let footer = "↑/↓ navigate · Enter select · q quit";
        let footer_y = area.bottom().saturating_sub(2);
        let footer_x = content_x + (content_width - footer.len() as u16) / 2;
        buf.set_line(footer_x, footer_y, &Line::raw(footer).style(Style::default().fg(text_muted)), content_width);
    }
}

pub fn render_home_screen(screen: &HomeScreen, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    screen.render(area, buf);
}
