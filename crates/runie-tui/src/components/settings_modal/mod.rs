//! Settings modal — grok-style configuration overlay.
//!
//! Shows current settings and allows switching themes.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Widget,
};
use crate::theme::ThemeWrapper;

/// Available themes matching grok's theme system.
pub static THEMES: &[(&str, &str)] = &[
    ("Default", "default"),
    ("GrokNight", "groknight"),
    ("GrokDay", "grokday"),
    ("TokyoNight", "tokyonight"),
    ("RosePineMoon", "rosepinemoon"),
];

/// Settings panel state.
#[derive(Debug, Clone, Default)]
pub struct SettingsModal {
    pub open: bool,
    pub selected_tab: usize,
    pub selected_item: usize,
}

impl SettingsModal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.open = true;
        self.selected_tab = 0;
        self.selected_item = 0;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn move_up(&mut self) {
        if self.selected_item > 0 {
            self.selected_item -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let max = if self.selected_tab == 0 { THEMES.len() } else { 0 };
        if self.selected_item + 1 < max {
            self.selected_item += 1;
        }
    }

    pub fn next_tab(&mut self) {
        self.selected_tab = (self.selected_tab + 1) % 2;
        self.selected_item = 0;
    }

    pub fn prev_tab(&mut self) {
        self.selected_tab = self.selected_tab.saturating_sub(1);
        self.selected_item = 0;
    }
}

fn render_settings_border(area: Rect, buf: &mut Buffer, border: Color) {
    for x in area.x..area.right() {
        buf.get_mut(x, area.y).set_char('─').set_fg(border);
        buf.get_mut(x, area.bottom() - 1).set_char('─').set_fg(border);
    }
    for y in area.y..area.bottom() {
        buf.get_mut(area.x, y).set_char('│').set_fg(border);
        buf.get_mut(area.right() - 1, y).set_char('│').set_fg(border);
    }
    buf.get_mut(area.x, area.y).set_char('┌');
    buf.get_mut(area.right() - 1, area.y).set_char('┐');
    buf.get_mut(area.x, area.bottom() - 1).set_char('└');
    buf.get_mut(area.right() - 1, area.bottom() - 1).set_char('┘');
}

fn render_settings_tabs(modal: &SettingsModal, area: Rect, buf: &mut Buffer, tab_active: Color, text_muted: Color) {
    let tabs = ["Theme", "Behavior"];
    let mut tab_x = area.x + 2;
    for (i, tab) in tabs.iter().enumerate() {
        let is_active = i == modal.selected_tab;
        let style = if is_active { Style::default().fg(tab_active).add_modifier(Modifier::BOLD | Modifier::UNDERLINED) } else { Style::default().fg(text_muted) };
        buf.set_string(tab_x, area.y + 1, tab, style);
        tab_x += tab.len() as u16 + 3;
    }
}

fn render_theme_tab(modal: &SettingsModal, area: Rect, buf: &mut Buffer, accent: Color, text_primary: Color, text_muted: Color) {
    let mut y = area.y + 3;
    for (i, (name, _)) in THEMES.iter().enumerate() {
        if y >= area.bottom() - 1 { break; }
        let is_selected = i == modal.selected_item;
        let indicator = if is_selected { "▸" } else { " " };
        let indicator_style = if is_selected { Style::default().fg(accent).add_modifier(Modifier::BOLD) } else { Style::default().fg(text_muted) };
        buf.set_string(area.x + 2, y, indicator, indicator_style);
        let name_style = if is_selected { Style::default().fg(text_primary).add_modifier(Modifier::BOLD) } else { Style::default().fg(text_primary) };
        buf.set_string(area.x + 4, y, name, name_style);
        y += 1;
    }
}

impl Widget for &SettingsModal {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border = Color::DarkGray;
        let text_primary = Color::White;
        let text_muted = Color::DarkGray;
        let accent = Color::Cyan;
        let tab_active = Color::Yellow;
        render_settings_border(area, buf, border);
        let title = " Settings ";
        buf.set_line(area.x + 2, area.y, &Line::raw(title).style(Style::default().fg(text_primary).add_modifier(Modifier::BOLD)), area.width - 4);
        let close = " [✗] ";
        let close_x = area.right() - close.len() as u16 - 2;
        buf.set_line(close_x, area.y, &Line::raw(close).style(Style::default().fg(text_muted)), area.width);
        render_settings_tabs(self, area, buf, tab_active, text_muted);
        let sep_y = area.y + 2;
        for x in area.x + 1..area.right() - 1 { buf.get_mut(x, sep_y).set_char('─').set_fg(border); }
        if self.selected_tab == 0 { render_theme_tab(self, area, buf, accent, text_primary, text_muted); }
        let footer = " Tab:next/prev · Enter:select · Esc:close ";
        buf.set_line(area.x + 2, area.bottom() - 1, &Line::raw(footer).style(Style::default().fg(text_muted)), area.width - 4);
    }
}

pub fn render_settings_modal(modal: &SettingsModal, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    modal.render(area, buf);
}
